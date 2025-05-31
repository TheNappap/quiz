
mod config;
mod status;
mod question;
mod owner;
mod service;

pub use config::{Config, get_config};
pub use question::{Answer, AnswerType, Question, QuestionType};
pub use status::{Event, QuizStatus, Ranking, Score};
pub use service::QuizStateService;

use std::path::PathBuf;
use tokio::sync::mpsc::channel;

pub fn create_quiz_state(root: PathBuf, config: Config) -> QuizStateService {            
    let (job_sender, job_receiver) = channel(1000);

    owner::create_quiz_state(root, config, job_receiver);

    QuizStateService::new(job_sender)
}


