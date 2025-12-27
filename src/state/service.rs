

use tokio::sync::mpsc::Sender;
use tokio::sync::oneshot::{self, Sender as Return};
use std::path::PathBuf;
use std::collections::HashMap;
use crate::error::QuizResult;

use super::{Answer, Event, Question, QuestionType, QuizStatus, Ranking, Score};

pub enum QuizStateJob {
    RootPath(Return<PathBuf>),
    Title(Return<String>),
    Status(Return<QuizStatus>),
    UserCount(Return<usize>),
    Users(Return<Vec<(String, i32)>>),
    UserExists(String, Return<bool>),
    Lobby(Return<Option<Event>>),
    AddUser(String, Return<QuizResult<()>>),
    RemoveUser(String, Return<QuizResult<()>>),
    Questions(Return<Vec<(String, QuestionType)>>),
    Question(usize, Return<Option<Question>>),
    Ranking(Return<Ranking>),
    UsersNoAnswer(Return<Vec<String>>),
    UngradedAnswers(Return<Vec<usize>>),
    Answers(usize, Return<Option<(HashMap<String,(String,Score)>,std::ops::RangeInclusive<usize>)>>),
    UpdateGrade(String, String, usize),
    Start(Return<Option<Event>>),
    Next(Return<Option<Event>>),
    LockQuestion,
    Redo(usize, Return<Option<Event>>),
    SubmitAnswer(Answer, Return<Result<String,String>>),
    Bonus(String, i32, Return<QuizResult<()>>),
    Backup(PathBuf, Return<QuizResult<()>>),
    ImportBackup(PathBuf, Return<QuizResult<Option<Event>>>),
}

#[derive(Debug, Clone)]
pub struct QuizStateService {
    job_channel: Sender<QuizStateJob>,
}

impl QuizStateService {
    pub(super) fn new(job_channel: Sender<QuizStateJob>) -> Self {
        QuizStateService { job_channel }
    }

    pub async fn root(&self) -> PathBuf {
        let (send, recv) = oneshot::channel();
        self.job_channel.send(QuizStateJob::RootPath(send)).await.expect("Send failed");
        recv.await.expect("Receive failed")
    }

    pub async fn title(&self) -> String {
        let (send, recv) = oneshot::channel();
        self.job_channel.send(QuizStateJob::Title(send)).await.expect("Send failed");
        recv.await.expect("Receive failed")
    }

    pub async fn status(&self) -> QuizStatus {
        let (send, recv) = oneshot::channel();
        self.job_channel.send(QuizStateJob::Status(send)).await.expect("Send failed");
        recv.await.expect("Receive failed")
    }

    pub async fn user_count(&self) -> usize {
        let (send, recv) = oneshot::channel();
        self.job_channel.send(QuizStateJob::UserCount(send)).await.expect("Send failed");
        recv.await.expect("Receive failed")
    }

    pub async fn users(&self) -> Vec<(String, i32)> {
        let (send, recv) = oneshot::channel();
        self.job_channel.send(QuizStateJob::Users(send)).await.expect("Send failed");
        recv.await.expect("Receive failed")
    }

    pub async fn user_exists(&self, username: &String) -> bool {
        let (send, recv) = oneshot::channel();
        self.job_channel.send(QuizStateJob::UserExists(username.clone(), send)).await.expect("Send failed");
        recv.await.expect("Receive failed")
    }
    
    pub async fn remove_user(&self, username: &String) -> QuizResult<()> {
        let (send, recv) = oneshot::channel();
        self.job_channel.send(QuizStateJob::RemoveUser(username.clone(), send)).await.expect("Send failed");
        recv.await.expect("Receive failed")
    }

    pub async fn lobby(&self) -> Option<Event> {
        let (send, recv) = oneshot::channel();
        self.job_channel.send(QuizStateJob::Lobby(send)).await.expect("Send failed");
        recv.await.expect("Receive failed")
    }

    pub async fn add_user(&self, username: &String) -> QuizResult<()> {
        let (send, recv) = oneshot::channel();
        self.job_channel.send(QuizStateJob::AddUser(username.clone(), send)).await.expect("Send failed");
        recv.await.expect("Receive failed")
    }

    pub async fn questions(&self) -> Vec<(String, QuestionType)> {
        let (send, recv) = oneshot::channel();
        self.job_channel.send(QuizStateJob::Questions(send)).await.expect("Send failed");
        recv.await.expect("Receive failed")
    }

    pub async fn question(&self, index: usize) -> Option<Question> {
        let (send, recv) = oneshot::channel();
        self.job_channel.send(QuizStateJob::Question(index, send)).await.expect("Send failed");
        recv.await.expect("Receive failed")
    }

    pub async fn ranking(&self) -> Ranking {
        let (send, recv) = oneshot::channel();
        self.job_channel.send(QuizStateJob::Ranking(send)).await.expect("Send failed");
        recv.await.expect("Receive failed")
    }

    pub async fn no_answer_users(&self) -> Vec<String> {
        let (send, recv) = oneshot::channel();
        self.job_channel.send(QuizStateJob::UsersNoAnswer(send)).await.expect("Send failed");
        recv.await.expect("Receive failed")
    }

    pub async fn ungraded_answers(&self) -> Vec<usize> {
        let (send, recv) = oneshot::channel();
        self.job_channel.send(QuizStateJob::UngradedAnswers(send)).await.expect("Send failed");
        recv.await.expect("Receive failed")
    }

    pub async fn answers(&self, index: usize)
        -> Option<(HashMap<String,(String,Score)>,std::ops::RangeInclusive<usize>)>
    {
        let (send, recv) = oneshot::channel();
        self.job_channel.send(QuizStateJob::Answers(index, send)).await.expect("Send failed");
        recv.await.expect("Receive failed")
    }

    pub async fn update_grade(&self, user: &String, question_title: &String, grade: usize) {
        let job = QuizStateJob::UpdateGrade(user.clone(), question_title.clone(), grade);
        self.job_channel.send(job).await.expect("Send failed");
    }

    pub async fn start(&self) -> Option<Event> {
        let (send, recv) = oneshot::channel();
        self.job_channel.send(QuizStateJob::Start(send)).await.expect("Send failed");
        recv.await.expect("Receive failed")
    }

    pub async fn next(&self) -> Option<Event> {
        let (send, recv) = oneshot::channel();
        self.job_channel.send(QuizStateJob::Next(send)).await.expect("Send failed");
        recv.await.expect("Receive failed")
    }

    pub async fn lock_question(&self) {
        self.job_channel.send(QuizStateJob::LockQuestion).await.expect("Send failed");
    }

    pub async fn redo(&self, index: usize) -> Option<Event> {
        let (send, recv) = oneshot::channel();
        self.job_channel.send(QuizStateJob::Redo(index, send)).await.expect("Send failed");
        recv.await.expect("Receive failed")
    }

    pub async fn submit_answer(&self, answer: &Answer) -> Result<String,String> {
        let (send, recv) = oneshot::channel();
        self.job_channel.send(QuizStateJob::SubmitAnswer(answer.clone(), send)).await.expect("Send failed");
        recv.await.expect("Receive failed")
    }
    
    pub async fn add_bonus(&self, user: &String, bonus: i32) -> QuizResult<()> {
        let (send, recv) = oneshot::channel();
        self.job_channel.send(QuizStateJob::Bonus(user.clone(), bonus, send)).await.expect("Send failed");
        recv.await.expect("Receive failed")
    }

    pub async fn backup(&self, path: &PathBuf) -> QuizResult<()> {
        let (send, recv) = oneshot::channel();
        self.job_channel.send(QuizStateJob::Backup(path.clone(), send)).await.expect("Send failed");
        recv.await.expect("Receive failed")
    }

    pub async fn import_backup(&self, path: &PathBuf) -> QuizResult<Option<Event>> {
        let (send, recv) = oneshot::channel();
        self.job_channel.send(QuizStateJob::ImportBackup(path.clone(), send)).await.expect("Send failed");
        recv.await.expect("Receive failed")
    }
}