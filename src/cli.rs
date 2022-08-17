
macro_rules! quiz_command_prefix {
    () => ({
        use std::io::Write;
        print!("Quiz command> ");
        std::io::stdout().flush().expect("Output flush failed");
    });
}

macro_rules! quiz_print {
    ($($arg:tt)*) => ({
        println!();
        println!($($arg)*);
        quiz_command_prefix!();
    })
}

use clap::{App,AppSettings,Arg,ArgMatches,SubCommand};
use std::sync::Arc;
use tabular::{Table, Row};
use tokio::sync::RwLock;
use tokio::io::{self, BufReader, AsyncBufReadExt};
use crate::quiz_state::{Ranking,Score,QuizState,QuizStatus,Event};
use crate::sse::SSE;

async fn read<'a>() -> Result<ArgMatches<'a>,String> {
    let mut s = String::new();
    BufReader::new(io::stdin()).read_line(&mut s).await.expect("Did not enter a correct string");
    App::new("Quiz command>")
        .setting(AppSettings::NoBinaryName)
        .setting(AppSettings::DisableVersion)
        .setting(AppSettings::SubcommandRequiredElseHelp)
        .setting(AppSettings::VersionlessSubcommands)
        .setting(AppSettings::InferSubcommands)
        .subcommand(SubCommand::with_name("exit")
            .about("Closes the quiz server."))
        .subcommand(SubCommand::with_name("status")
            .about("Prints the current status of the quiz."))
        .subcommand(SubCommand::with_name("users")
            .about("Prints the list of users."))
        .subcommand(SubCommand::with_name("questions")
            .about("Prints the list of questions."))
        .subcommand(SubCommand::with_name("start")
            .about("Starts the quiz and sets the status to the first question."))
        .subcommand(SubCommand::with_name("next")
            .about("Sets the status to the next question or finishes the quiz if there are no more questions."))
        .subcommand(SubCommand::with_name("lock")
            .about("Locks the current question and prevents users from submitting answers. (To unlock again, use `redo`)"))
        .subcommand(SubCommand::with_name("redo")
            .about("Redo a question. Give a question id or use the current question.")
            .arg(Arg::with_name("id")
                    .help("Id of the question to redo.")))
        .subcommand(SubCommand::with_name("ranking")
            .about("Print the current ranking."))
        .subcommand(SubCommand::with_name("share")
            .about("Share the current ranking to all users."))
        .subcommand(SubCommand::with_name("qsumm")
            .about("Question summary. Give a question id or use the current question.")
            .arg(Arg::with_name("id")
                    .help("Id of the question to summarize.")))
        .subcommand(SubCommand::with_name("grade")
            .about("Grade question. Give a question id or use the current question.")
            .arg(Arg::with_name("id")
                    .help("Id of the question to grade.")))
        .subcommand(SubCommand::with_name("backup")
            .about("Backup the current state of the quiz.")
            .arg(Arg::with_name("file")
                    .help("File to write backup to. (default:\".backup_quiz\")")))
        .subcommand(SubCommand::with_name("import")
            .about("Import a backup state of a quiz.")
            .arg(Arg::with_name("file")
                    .help("File to read backup from.")
                    .required(true)))
        .get_matches_from_safe(s.trim().split_whitespace().collect::<Vec<_>>())
        .map_err(|e|e.message)
}

pub async fn main(state: Arc<RwLock<QuizState>>, sse: Arc<RwLock<SSE>>, ) {
    loop{
        quiz_command_prefix!();
        match read().await.as_ref().map(|m|m.subcommand()) {
            Ok(("exit", Some(_)))       => break,
            Ok(("status", Some(_)))     => status(state.clone()).await,
            Ok(("questions", Some(_)))  => questions(state.clone()).await,
            Ok(("users", Some(_)))      => users(state.clone()).await,
            Ok(("start", Some(_)))      => start(state.clone(),sse.clone()).await,
            Ok(("next", Some(_)))       => next(state.clone(),sse.clone()).await,
            Ok(("lock", Some(_))) 		=> lock_question(state.clone()).await,
            Ok(("redo", Some(matches))) => redo_question(state.clone(),sse.clone(), matches).await,
            Ok(("ranking", Some(_)))    => ranking(state.clone()).await,
            Ok(("share", Some(_)))      => share_ranking(state.clone(),sse.clone()).await,
            Ok(("qsumm", Some(matches)))=> qsumm(state.clone(), matches, false).await,
            Ok(("grade", Some(matches)))=> qsumm(state.clone(), matches, true).await,
            Ok(("backup", Some(matches)))=> backup(state.clone(), matches).await,
            Ok(("import", Some(matches)))=> import_backup(state.clone(),sse.clone(), matches).await,
            Err(e) => println!("{}",e),
            _ => unreachable!()
        }
    }
    sse.write().await.close();
    println!("Closing server...");
}

async fn status(state: Arc<RwLock<QuizState>>) {
    print!("status: ");
    match state.read().await.status() {
        QuizStatus::Lobby => println!("In Lobby"),
        QuizStatus::Done => println!("Finished Quiz"),
        QuizStatus::Question{id,locked} => {
            println!("Question in progress\nid: {}\nquestion: {}", id, state.read().await.questions()[*id].0);
            if *locked {
                println!("LOCKED");
            }
        }
    }
}

async fn questions(state: Arc<RwLock<QuizState>>) {
    let mut table = Table::new("\t{:<}: {:<} {:<}");
    for (i,(title,type_)) in state.read().await.questions().iter().enumerate() {
        table.add_row(Row::new()
            .with_cell(i)
            .with_cell(title)
            .with_cell(type_));
    }
    println!("{}", table);
}

async fn users(state: Arc<RwLock<QuizState>>) {
    let mut table = Table::new("\t{:<}");
    table.add_heading("\tUsers:");
    for user in state.read().await.users() {
        table.add_row(Row::new().with_cell(user));
    }
    println!("{}", table);
}

async fn start(state: Arc<RwLock<QuizState>>, sse: Arc<RwLock<SSE>>, ) {
    if state.read().await.users().count() == 0 {
        println!("No users logged in yet.");
        return;
    }
    if !state.read().await.status().is_lobby() { return }

    users(state.clone()).await;
    loop {
        use std::io::Write;
        print!("Do you want to start the quiz? (y/n)> ");
        std::io::stdout().flush().expect("Output flush failed");
        let mut s = String::new();
        BufReader::new(io::stdin()).read_line(&mut s).await.expect("Did not enter a correct string");
        match s.trim() {
            "y" => break,
            "n" => return,
            _   => println!("Answer y or n"),
        }
    }

    let start = state.write().await.start();
    if let Some(e) = start {
        if let Ok(e) = e.to_string() {
            status(state.clone()).await;
            sse.write().await.send_to_clients(e).await; 
        }
    }
}

async fn all_answered_current_question(state: Arc<RwLock<QuizState>>) -> bool {
    let no_answer_users = state.read().await.no_answer_users();
    if no_answer_users.is_empty() {
        true
    } else {
        if state.read().await.status().question().is_some() {
            let mut table = Table::new("\t{:<}");
            table.add_heading("Not all users have answered yet:");
            for user in no_answer_users {
                table.add_row(Row::new().with_cell(user));
            }
            println!("{}", table);
        }
        false
    }
}

async fn next(state: Arc<RwLock<QuizState>>, sse: Arc<RwLock<SSE>>, ) {
    if !all_answered_current_question(state.clone()).await {
        return
    }

    let next = state.write().await.next();
    if let Some(e) = next {
        if let Ok(e) = e.to_string() {
            status(state.clone()).await;
            sse.write().await.send_to_clients(e).await; 
        }
    }
}

async fn lock_question(state: Arc<RwLock<QuizState>>) {
    if !all_answered_current_question(state.clone()).await {
        return
    }
    state.write().await.lock_question();
}

async fn redo_question<'a>(state: Arc<RwLock<QuizState>>, sse: Arc<RwLock<SSE>>, args: &ArgMatches<'a>) {
    let index = match args.value_of("id").and_then(|o|o.parse::<usize>().ok()) {
        Some(index) => index,
        None => if let QuizStatus::Question{id,..} = state.read().await.status() { *id } 
                else { return }
    };
    let redo_event = state.write().await.redo(index);
    if let Some(e) = redo_event {
        if let Ok(e) = e.to_string() {
            sse.write().await.send_to_clients(e).await; 
        }
    }
}

async fn share_ranking(state: Arc<RwLock<QuizState>>, sse: Arc<RwLock<SSE>>, ) {
    if state.read().await.status().is_lobby() { return }
    if !all_answered_current_question(state.clone()).await {
        return
    }

    let ungraded_answers = state.read().await.ungraded_answers();
    if ungraded_answers.is_empty() {
        ranking(state.clone()).await;
        let ranking = state.read().await.ranking();
        if let Ok(e) = Event::Ranking(ranking).to_string() {
            sse.write().await.send_to_clients(e).await; 
        }
    } else {
        println!("There are ungraded answers left.\nUngraded questions:{:?}",ungraded_answers);
    }
}

async fn ranking(state: Arc<RwLock<QuizState>>) {
    let Ranking{max_score,scores} = state.read().await.ranking();
    let mut table = Table::new("\t{:<}: {:>}/{:<}");
    for (user,score) in scores {
        table.add_row(Row::new()
            .with_cell(user)
            .with_cell(score)
            .with_cell(max_score));
    }
    println!("{}", table);
}

async fn qsumm<'a>(state: Arc<RwLock<QuizState>>, args: &ArgMatches<'a>, do_grade: bool) {
    let index = match args.value_of("id").and_then(|o|o.parse::<usize>().ok()) {
        Some(index) => index,
        None => if let Some(index) = state.read().await.status().question() { index } 
                else { return }
    };

    let state_ = state.read().await;
    if let (Some(question),Some((answers,score_range))) = (state_.questions().get(index),state_.answers(index)) {
        let title = question.0.to_string();
        let type_ = question.1.clone();
        drop(state_);        
        let table_head = Table::new("\t{:<}\t{:^} {:>}")
            .with_heading(format!("question: {}", title))
            .with_heading(format!("type: {}", type_))
            .with_row(Row::new().with_cell("")
                .with_cell("Answer")
                .with_cell("Grade"));
        let mut table = table_head.clone();
        for (user, (answer,score)) in answers {
            let row_head = Row::new()
                    .with_cell(user.clone())
                    .with_cell(answer.clone().replace("\n"," "));
            let mut row = row_head.clone().with_cell(match score {
                        Score::Grade(s) => format!("{}/{}",s,score_range.end()),
                        Score::Ungraded => "not graded yet".to_string(),
                    });
            if do_grade {
                println!("{}", table_head.clone().with_row(row.clone()));
                let new_grade = grade(state.clone(), &user, &title, score_range.clone()).await;
                row = row_head.with_cell(format!("{}/{}",new_grade,score_range.end()))
            }
            table.add_row(row);
        }
        println!("{}", table);
    }
}

async fn grade(state: Arc<RwLock<QuizState>>,user: &String, question: &String, range: std::ops::RangeInclusive<usize>) -> usize {
    loop {
        use std::io::Write;
        print!("Grade (range: {},...,{})> ",range.start(),range.end());
        std::io::stdout().flush().expect("Output flush failed");
        let mut s = String::new();
        BufReader::new(io::stdin()).read_line(&mut s).await.expect("Did not enter a correct string");
        let result = s.trim().parse::<usize>()
            .map_err(|e| e.to_string())
            .and_then(|s|{
                if range.contains(&s) { Ok(s) }
                else { Err("Score not in range.".to_string()) }
            });
        match result {
            Ok(s) => {
                state.write().await.update_grade(user,question,s);
                return s;
            },
            Err(e) => println!("{}",e),
        }
    }
}

async fn backup<'a>(state: Arc<RwLock<QuizState>>, args: &ArgMatches<'a>) {
    let path = args.value_of("file").unwrap_or(".backup_quiz").into();
    match state.read().await.backup(&path) {
        Ok(_) => println!("Backup created: {:?}", path),
        Err(e) => println!("An error occurred while trying to backup: {:?}", e),
    }
}

async fn import_backup<'a>(state: Arc<RwLock<QuizState>>, sse: Arc<RwLock<SSE>>, args: &ArgMatches<'a>) {
    let path = args.value_of("file").unwrap().into();
    match state.write().await.import_backup(&path) {
        Ok(ev) => { 
            println!("Succesfully imported: {:?}", path);
            if let Some(ev) = ev {
                if let Ok(ev) = ev.to_string() {
                    sse.write().await.send_to_clients(ev).await; 
                }
            }
        },
        Err(e) => println!("An error occurred while trying to import backup: {:?}", e),
    }
}