use std::{collections::HashMap, path::PathBuf};

use crate::error::{Error, QuizResult};

use super::{service::QuizStateJob, Answer, AnswerType, Config, Event, Question, QuestionType, QuizStatus, Ranking, Score};

use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::Receiver;

pub(super) fn create_quiz_state(root: PathBuf, config: Config, job_receiver: Receiver<QuizStateJob>) {            
    tokio::task::spawn(async move {
        let owner = QuizStateOwner{
            root,
            state: QuizState {
                config,
                status: QuizStatus::Lobby,
                users: HashMap::new(),
            }
        };

        owner.handle_jobs(job_receiver).await
    });
}

#[derive(Debug, Serialize, Deserialize)]
struct QuizState {
    config: Config,
    status: QuizStatus,
    users: HashMap<String,HashMap<String,(AnswerType,Score)>>,
}

#[derive(Debug)]
struct QuizStateOwner {
    root: PathBuf,
    state: QuizState,
}

impl QuizStateOwner {
    async fn handle_jobs(mut self, mut job_receiver: Receiver<QuizStateJob>) {
        loop {
            if let Some(job) = job_receiver.recv().await {
                match job {
                    QuizStateJob::RootPath(sender)                                     => sender.send(self.root().clone()).unwrap(),
                    QuizStateJob::Title(sender)                                         => sender.send(self.title().into()).unwrap(),
                    QuizStateJob::Status(sender)                                    => sender.send(self.status().clone()).unwrap(),
                    QuizStateJob::UserCount(sender)                                      => sender.send(self.user_count()).unwrap(),
                    QuizStateJob::Users(sender)                                    => sender.send(self.users()).unwrap(),
                    QuizStateJob::UserExists(username, sender)                    => sender.send(self.user_exists(&username)).unwrap(),
                    QuizStateJob::Lobby(sender)                                  => sender.send(self.lobby()).unwrap(),
                    QuizStateJob::AddUser(username, sender)          => sender.send(self.add_user(username)).unwrap(),
                    QuizStateJob::Questions(sender)                => sender.send(self.questions()).unwrap(),
                    QuizStateJob::Question(index, sender)              => sender.send(self.question(index)).unwrap(),
                    QuizStateJob::Ranking(sender)                                      => sender.send(self.ranking()).unwrap(),
                    QuizStateJob::UsersNoAnswer(sender)                            => sender.send(self.no_answer_users()).unwrap(),
                    QuizStateJob::UngradedAnswers(sender)                           => sender.send(self.ungraded_answers()).unwrap(),
                    QuizStateJob::Answers(index, sender)    => sender.send(self.answers(index)).unwrap(),
                    QuizStateJob::UpdateGrade(user, question_title, grade)       => self.update_grade(user, question_title, grade),
                    QuizStateJob::Start(sender)                                  => sender.send(self.start()).unwrap(),
                    QuizStateJob::Next(sender)                                   => sender.send(self.next()).unwrap(),
                    QuizStateJob::LockQuestion                                                          => self.lock_question(),
                    QuizStateJob::Redo(index, sender)                     => sender.send(self.redo(index)).unwrap(),
                    QuizStateJob::SubmitAnswer(answer, sender)  => sender.send(self.submit_answer(answer)).unwrap(),
                    QuizStateJob::Backup(path, sender)              => sender.send(self.backup(&path)).unwrap(),
                    QuizStateJob::ImportBackup(path, sender) => sender.send(self.import_backup(&path)).unwrap(),
                }
            }
        }
    }

    pub fn root(&self) -> &PathBuf {
        &self.root
    }

    pub fn title(&self) -> &str {
        &self.state.config.title()
    }
    
    pub fn status(&self) -> &QuizStatus {
        &self.state.status
    }

    pub fn user_count(&self) -> usize {
        self.state.users.len()
    }

    pub fn users(&self) -> Vec<String> {
        self.state.users.keys().cloned().collect()
    }

    pub fn user_exists(&self, username: &String) -> bool {
        self.state.users.contains_key(username)
    }
    
    pub fn lobby(&self) -> Option<Event> {
        match self.status() {
            QuizStatus::Lobby => Some(Event::Lobby{users:self.state.users.keys().map(|s|s.clone()).collect()}),
            _ => None
        }
    }

    pub fn add_user(&mut self, username: String) -> QuizResult<()> {
        if !username.is_empty() && !self.user_exists(&username) {
            self.state.users.insert(username,HashMap::new());
            Ok(())
        } else { Err(Error::Other) }
    }
    
    pub fn questions(&self) -> Vec<(String,QuestionType)> {
        self.state.config.questions().iter().map(|q|{
            (q.title().clone(),q.type_spec().clone())
        }).collect()
    }

    fn question(&self, index: usize) -> Option<Question> {
        self.state.config.questions().get(index).cloned()
    }
    
    pub fn ranking(&self) -> Ranking {
        let max_score = self.state.config.questions().iter().fold(0,|acc,q| acc + q.max_score());
        let mut scores: Vec<_> = self.state.users.iter().map(|(user,answers)|{
            let score = self.state.config.questions().iter().fold(0,|acc, q| {
                match answers.get(q.title()) {
                    Some((_,Score::Grade(s))) => acc + s,
                    _ => acc
                }
            });
            (user.clone(),score)
        }).collect();
        scores.sort_by(|(_,a), (_,b)| b.cmp(a));
        Ranking{max_score,scores}
    }

    pub fn no_answer_users(&self) -> Vec<String> {
        let cur_q = match self.status() {
            QuizStatus::Question{id,..} => *id,
            _ => return Vec::new()
        };
        if let Some(question) = self.state.config.questions().get(cur_q) {
            self.state.users.iter().filter(|(_,answers)|{
                answers.get(question.title()).is_none()
            })
            .map(|(u,_)| u.to_string())
            .collect()
        }
        else { Vec::new() }
    }

    pub fn ungraded_answers(&self) -> Vec<usize> {
        let cur_q = match self.status() {
            QuizStatus::Lobby => return Vec::new(),
            QuizStatus::Question{id,..} => *id,
            QuizStatus::Done => self.state.config.question_count()-1
        };
        self.state.config.questions()[0..=cur_q].iter()
                .enumerate()
                .filter(|(_,q)|{
                    self.state.users.values().fold(false, |acc, answers|{
                        acc | match answers.get(q.title()).map(|(_,s)|s.is_ungraded()) {
                            Some(u) => u,
                            None => false
                        }
                    })
                })
                .map(|(i,_)|i)
                .collect()
    }

    pub fn answers(&self, index: usize) 
        -> Option<(HashMap<String,(String,Score)>,std::ops::RangeInclusive<usize>)>
	{
        let question = self.state.config.questions().get(index)?;
        Some((self.state.users.iter().map(|(user,answers)|{
            if let Some((answer,score)) = answers.get(question.title()) 
                { (user.clone(),(question.get_answer_string(&answer),*score)) }
            else { (user.clone(),("".to_string(),Score::Ungraded)) }
        }).collect(),question.grade_range().range()))
    }

    pub fn update_grade(&mut self, user: String, question_title: String, grade: usize) {
        self.state.users.get_mut(&user)
            .and_then(|answers| answers.get_mut(&question_title))
            .map(|(_,score)| *score = Score::Grade(grade));
    }

    fn question_event(&self, index: usize) -> Option<Event> {
        let question_count = self.state.config.question_count();
        if index == question_count {
            Some(Event::Finished)
        } else {
            self.question(index).map(|q|{
                Event::Question {
                    title: q.title().clone(), 
                    id:index, total:question_count,
                    image: q.image().and_then(|p| p.to_str().map(|s| s.to_string())), 
                    question_type: q.type_spec().into(),
                }
            })
        }
    }

    fn answer_string(&self, answer: &Answer) -> String {
        match self.state.config
            .question(answer.question())
            .map(|(_,q)| {
                q.get_answer_string(answer.answer())
            })
        {
            Some(s) => s,
            None => "".into()
        }
    }

    pub fn start(&mut self) -> Option<Event> {
        match &self.state.status {
            QuizStatus::Lobby => {
                self.question_event(0).map(|e|{
                    self.state.status = QuizStatus::Question{id:0,locked:false};
                    e
                })
            },
            _ => None
        }
    }
    
    pub fn next(&mut self) -> Option<Event> {
        match self.state.status {
            QuizStatus::Question{id,..} => {
                self.question_event(id+1).map(|e|{
                    match e {
                        Event::Question{..} => self.state.status = QuizStatus::Question{id:id+1,locked:false},
                        Event::Finished => self.state.status = QuizStatus::Done,
                        _ => ()
                    }
                    self.backup(&self.root.join(".backup_quiz")).unwrap();
                    e
                })
            },
            _ => None
        }
    }
    
    pub fn lock_question(&mut self) {
        match &mut self.state.status {
            QuizStatus::Question{locked,..} => *locked = true,
            _ => ()
        }
    }
    
    pub fn redo(&mut self, index: usize) -> Option<Event> {
        if index >= self.state.config.question_count() {
            None
        } else {
            self.question_event(index).map(|e|{
                    match e {
                        Event::Question{..} => self.state.status = QuizStatus::Question{id:index,locked:false},
                        _ => ()
                    }
					e
                })
        }
    }

    pub fn submit_answer(&mut self, answer: Answer) -> Result<String,String> {
        let question_title = answer.question().clone();
        if let (Some(answers),Some((index,question))) =
            (self.state.users.get_mut(answer.user()), self.state.config.question(&question_title)) 
        {
            match self.state.status {
                QuizStatus::Question{id,locked} if id == index => {
                    if locked {
                        return Err("Could not submit answer: question is locked.".into());
                    }
                    let answer_type = answer.answer().clone();
                    let score = question.calculate_score(&answer_type).into();
                    answers.insert( question_title, (answer_type, score) );
                    Ok(self.answer_string(&answer))
                },
                _ => Err("Could not submit answer: no question open.".into())
            }
        } else { Err("Could not submit answer: server error.".into()) }
    }

    pub fn backup(&self, path: &PathBuf) -> QuizResult<()> {
        let state = serde_json::to_string(&self.state)?;
        Ok(std::fs::write(path, state)?)
    }

    pub fn import_backup(&mut self, path: &PathBuf) -> QuizResult<Option<Event>> {
        let data = std::fs::read_to_string(path)?;
        let state: QuizState = serde_json::from_str(&data)?;
        if self.state.users.keys().collect::<Vec<_>>() != state.users.keys().collect::<Vec<_>>() {
            return Err(Error::String("Current users and imported users do not match".into()));
        }
        self.state = state;
        Ok(match &self.state.status {
            QuizStatus::Question{id,..} => self.question_event(*id),
            QuizStatus::Done => Some(Event::Finished),
            QuizStatus::Lobby => Some(Event::Lobby{users:self.state.users.keys().map(|s|s.clone()).collect()})
        })
    }
}