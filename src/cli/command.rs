use tabular::{Row, Table};
use tokio::io::{self, AsyncBufReadExt, BufReader};

use crate::{server::SseService, state::{Event, QuizStateService, QuizStatus, Ranking, Score}};


pub async fn status(state: QuizStateService) {
    print!("status: ");
    match state.status().await {
        QuizStatus::Lobby => println!("In Lobby"),
        QuizStatus::Done => println!("Finished Quiz"),
        QuizStatus::Question{id,locked} => {
            let question = state.question(id).await.unwrap();
            println!("Question in progress\nid: {}\nquestion: {}", id, &question.title());
            if locked {
                println!("LOCKED");
            }
        }
    }
}

pub async fn questions(state: QuizStateService) {
    let mut table = Table::new("\t{:<}: {:<} {:<}");
    for (i,(title,type_)) in state.questions().await.iter().enumerate() {
        table.add_row(Row::new()
            .with_cell(i)
            .with_cell(title)
            .with_cell(type_));
    }
    println!("{}", table);
}

pub async fn users(state: QuizStateService) {
    let mut table = Table::new("\t{:<}");
    table.add_heading("\tUsers:");
    for user in state.users().await {
        table.add_row(Row::new().with_cell(user));
    }
    println!("{}", table);
}

async fn yes_no_question(message: &str) -> bool {
    loop {
        use std::io::Write;
        print!("{} (y/n)> ", message);
        std::io::stdout().flush().expect("Output flush failed");
        let mut s = String::new();
        BufReader::new(io::stdin()).read_line(&mut s).await.expect("Did not enter a correct string");
        match s.trim() {
            "y" => return true,
            "n" => return false,
            _   => println!("Answer y or n"),
        }
    }
}

pub async fn start_event(state: QuizStateService, sse: SseService) {
    if state.user_count().await == 0 {
        println!("No users logged in yet.");
        return;
    }
    if !state.status().await.is_lobby() { return }

    users(state.clone()).await;
    if !yes_no_question("Do you want to start the quiz?").await {
        return;
    }

    let start = state.start().await;
    if let Some(e) = start {
        status(state.clone()).await;
        sse.send_event(e).await;
    }
}

pub async fn continue_on_all_answered(state: QuizStateService) -> bool {
    let no_answer_users = state.no_answer_users().await;
    if no_answer_users.is_empty() {
        true
    } else {
        if state.status().await.question().is_some() {
            let mut table = Table::new("\t{:<}");
            table.add_heading("Not all users have answered yet:");
            for user in no_answer_users {
                table.add_row(Row::new().with_cell(user));
            }
            println!("{}", table);
            if yes_no_question("Do you want to continue anyway?").await {
                return true;
            }
        }
        false
    }
}

pub async fn next(state: QuizStateService, sse: SseService) {
    if !continue_on_all_answered(state.clone()).await {
        return
    }

    let next = state.next().await;
    if let Some(e) = next {
        sse.send_event(e).await;
    }
}

pub async fn lock_question(state: QuizStateService) {
    if !continue_on_all_answered(state.clone()).await {
        return
    }
    state.lock_question().await;
}

pub async fn redo_question(state: QuizStateService, sse: SseService, id: Option<usize>) {
    let index = match id {
        Some(index) => index,
        None => if let QuizStatus::Question{id,..} = state.status().await { id } 
                else { return }
    };
    let redo_event = state.redo(index).await;
    if let Some(e) = redo_event {
        sse.send_event(e).await;
    }
}

pub async fn share_ranking(state: QuizStateService, sse: SseService) {
    if state.status().await.is_lobby() { return }
    if !continue_on_all_answered(state.clone()).await {
        return
    }

    let ungraded_answers = state.ungraded_answers().await;
    if ungraded_answers.is_empty() {
        ranking(state.clone()).await;
        let ranking = state.ranking().await;
        sse.send_event(Event::Ranking(ranking)).await;
    } else {
        println!("There are ungraded answers left.\nUngraded questions:{:?}",ungraded_answers);
    }
}

pub async fn ranking(state: QuizStateService) {
    let Ranking{max_score,scores} = state.ranking().await;
    let mut table = Table::new("\t{:<}: {:>}/{:<}");
    for (user,score) in scores {
        table.add_row(Row::new()
            .with_cell(user)
            .with_cell(score)
            .with_cell(max_score));
    }
    println!("{}", table);
}

pub async fn qsumm(state: QuizStateService, id: Option<usize>, do_grade: bool) {
    if do_grade && !continue_on_all_answered(state.clone()).await {
        return;
    }

    let index = match id {
        Some(index) => index,
        None => if let Some(index) = state.status().await.question() { index } 
                else { return }
    };

    let question = state.question(index).await;
    let answers = state.answers(index).await;
    if let (Some(question),Some((answers,score_range))) = (question, answers) {
        let title = question.title();
        let type_ = question.type_spec();
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
                if let Some(new_grade) = grade(state.clone(), &user, &title, score_range.clone()).await {
                    row = row_head.with_cell(format!("{}/{}",new_grade,score_range.end()))
                }
            }
            table.add_row(row);
        }
        println!("{}", table);
    }
}

pub async fn grade(state: QuizStateService,user: &String, question: &String, range: std::ops::RangeInclusive<usize>) -> Option<usize> {
    loop {
        use std::io::Write;
        print!("Grade (range: {},...,{} or `skip`)> ",range.start(),range.end());
        std::io::stdout().flush().expect("Output flush failed");
        let mut s = String::new();
        BufReader::new(io::stdin()).read_line(&mut s).await.expect("Did not enter a correct string");
        let s = s.trim();
        if s == "skip" {
            return None;
        }

        let result = s.parse::<usize>()
            .map_err(|e| e.to_string())
            .and_then(|s|{
                if range.contains(&s) { Ok(s) }
                else { Err("Score not in range.".to_string()) }
            });
        match result {
            Ok(s) => {
                state.update_grade(user, question, s).await;
                return Some(s);
            },
            Err(e) => println!("{}",e),
        }
    }
}

pub async fn backup(state: QuizStateService, file: String) {
    let path = file.into();
    match state.backup(&path).await {
        Ok(_) => println!("Backup created: {:?}", path),
        Err(e) => println!("An error occurred while trying to backup: {:?}", e),
    }
}

pub async fn import_backup(state: QuizStateService, sse: SseService, file: String) {
    let path = file.into();
    match state.import_backup(&path).await {
        Ok(ev) => { 
            println!("Succesfully imported: {:?}", path);
            if let Some(ev) = ev {
                sse.send_event(ev).await;
            }
        },
        Err(e) => println!("An error occurred while trying to import backup: {}", e),
    }
}