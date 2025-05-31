
mod command;
#[macro_use]
mod print;

use clap::{Parser, Subcommand};
use tokio::io::{self, BufReader, AsyncBufReadExt};
use crate::{server::SseService, state::QuizStateService};

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

pub async fn start(state: QuizStateService, sse: SseService) {
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
            QuizCommand::Exit               => break,
            QuizCommand::Status             => command::status(state.clone()).await,
            QuizCommand::Questions          => command::questions(state.clone()).await,
            QuizCommand::Users              => command::users(state.clone()).await,
            QuizCommand::Start              => command::start_event(state.clone(),sse.clone()).await,
            QuizCommand::Next               => command::next(state.clone(),sse.clone()).await,
            QuizCommand::Lock               => command::lock_question(state.clone()).await,
            QuizCommand::Redo { id } => command::redo_question(state.clone(),sse.clone(), id).await,
            QuizCommand::Ranking            => command::ranking(state.clone()).await,
            QuizCommand::Share              => command::share_ranking(state.clone(),sse.clone()).await,
            QuizCommand::Qsumm { id } => command::qsumm(state.clone(), id, false).await,
            QuizCommand::Grade { id } => command::qsumm(state.clone(), id, true).await,
            QuizCommand::Backup { file } => command::backup(state.clone(), file).await,
            QuizCommand::Import { file } => command::import_backup(state.clone(),sse.clone(), file).await,
        }
    }
    sse.close().await;
    println!("Closing server...");
}
