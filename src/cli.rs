
use clap::{Parser, Subcommand};
use std::sync::Arc;
use tabular::{Table, Row};
use tokio::sync::RwLock;
use tokio::io::{self, BufReader, AsyncBufReadExt};
use crate::quiz_state::{Ranking,Score,QuizState,QuizStatus,Event};
use crate::sse::SSE;

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

/// A simple quiz server app
#[derive(Parser, Debug)]
#[command(author,version, about,
    no_binary_name=true,
    subcommand_required=true,
    infer_subcommands=true,
)]
pub struct QuizArgs {
    #[command(subcommand)]
    command: QuizCommand,
}

#[derive(Subcommand, Debug)]
enum QuizCommand {
    /// Closes the quiz server.
    Exit,
    /// Prints the current status of the quiz.
    Status,
    /// Prints the list of users.
    Users,
    /// Prints the list of questions.
    Questions,
    /// Starts the quiz and sets the status to the first question.
    Start,
    /// Sets the status to the next question or finishes the quiz if there are no more questions.
    Next,
    /// Locks the current question and prevents users from submitting answers. (To unlock again, use `redo`)
    Lock,
    /// Redo a question. Give a question id or use the current question.
    Redo{
        /// Id of the question to redo.
        id: Option<usize>
    },
    /// Print the current ranking.
    Ranking,
    /// Share the current ranking to all users.
    Share,
    /// Question summary. Give a question id or use the current question.
    Qsumm{
        /// Id of the question to summarize.
        id: Option<usize>
    },
    /// Grade question. Give a question id or use the current question.
    Grade{
        /// Id of the question to grade.
        id: Option<usize>
    },
    /// Backup the current state of the quiz.
    Backup{
        // File to write backup to.
        #[arg(help="File to write backup to.", default_value_t = String::from(".backup_quiz"))]
        file: String
    },
    /// Import a backup state of a quiz.
    Import{
        /// File to read backup from.
        file: String
    },
}

pub async fn main(state: Arc<RwLock<QuizState>>, sse: Arc<RwLock<SSE>>) {
    loop{
        quiz_command_prefix!();
        let mut input = String::new();
        BufReader::new(io::stdin()).read_line(&mut input).await.expect("Did not enter a correct string");
        let quiz_args = QuizArgs::try_parse_from(input.trim().split_whitespace());
        let quiz_args = match quiz_args {
            Ok(args) =>  args,
            Err(e) => { println!("{}",e); continue; }
        };

        match quiz_args.command {
            QuizCommand::Exit       => break,
            QuizCommand::Status       => status(state.clone()).await,
            QuizCommand::Questions       => questions(state.clone()).await,
            QuizCommand::Users       => users(state.clone()).await,
            QuizCommand::Start       => start(state.clone(),sse.clone()).await,
            QuizCommand::Next       => next(state.clone(),sse.clone()).await,
            QuizCommand::Lock       => lock_question(state.clone()).await,
            QuizCommand::Redo { id } => redo_question(state.clone(),sse.clone(), id).await,
            QuizCommand::Ranking    => ranking(state.clone()).await,
            QuizCommand::Share      => share_ranking(state.clone(),sse.clone()).await,
            QuizCommand::Qsumm { id } => qsumm(state.clone(), id, false).await,
            QuizCommand::Grade { id } => qsumm(state.clone(), id, true).await,
            QuizCommand::Backup { file } => backup(state.clone(), file).await,
            QuizCommand::Import { file } => import_backup(state.clone(),sse.clone(), file).await,
        }
    }
    sse.write().await.close().await;
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

async fn redo_question<'a>(state: Arc<RwLock<QuizState>>, sse: Arc<RwLock<SSE>>, id: Option<usize>) {
    let index = match id {
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

async fn share_ranking(state: Arc<RwLock<QuizState>>, sse: Arc<RwLock<SSE>>) {
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

async fn qsumm<'a>(state: Arc<RwLock<QuizState>>, id: Option<usize>, do_grade: bool) {
    let index = match id {
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

async fn backup<'a>(state: Arc<RwLock<QuizState>>, file: String) {
    let path = file.into();
    match state.read().await.backup(&path) {
        Ok(_) => println!("Backup created: {:?}", path),
        Err(e) => println!("An error occurred while trying to backup: {:?}", e),
    }
}

async fn import_backup<'a>(state: Arc<RwLock<QuizState>>, sse: Arc<RwLock<SSE>>, file: String) {
    let path = file.into();
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
