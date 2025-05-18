use serde::{Serialize, Deserialize};
use std::path::PathBuf;
use std::collections::HashMap;
use crate::error::{Error, Result};
use crate::question::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Event {
    Lobby{
        users: Vec<String>
    },
    Question {
        title: String,
        id: usize,
        total: usize,
        image: Option<String>,
        question_type: QuestionSendType,
    },
    Ranking(Ranking),
    Finished,
    Closed,
}

impl Event {
    pub fn to_string(&self) -> String {
        serde_json::to_string(self).unwrap()
    }
} 

#[derive(Debug, Serialize, Deserialize)]
struct Config {
    title: String,
    questions: Vec<Question>,
}

impl Config {
    fn from(root: &PathBuf) -> Result<Self> {
        let path = root.join("quiz.config");
        let data = std::fs::read_to_string(path)?;
        Ok(serde_json::from_str(&data)?)
    }
    
    fn question(&self, title: &str) -> Option<(usize,&Question)> {
        self.questions.iter().enumerate().filter(|(_,q)| q.title() == title).next()
    }
}

#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub enum Score{
    Ungraded,
    Grade(usize)
}

impl Score {
    fn is_ungraded(&self) -> bool {
        match self {
            Score::Ungraded => true,
            _ => false
        }
    }
}

impl From<Option<usize>> for Score {
    fn from(opt: Option<usize>) -> Self {
        match opt {
            Some(s) => Score::Grade(s),
            None => Score::Ungraded
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ranking {
    pub max_score: usize,
    pub scores: Vec<(String,usize)>
}

#[derive(Debug, Serialize, Deserialize)]
pub enum QuizStatus {
    Lobby,
    Question{
        id:usize,
        locked:bool
    },
    Done
}

impl QuizStatus {
    pub fn is_lobby(&self) -> bool {
        if let QuizStatus::Lobby = self { true } else { false }
    }
    
    pub fn question(&self) -> Option<usize> {
        if let QuizStatus::Question{id,..} = self { Some(*id) } else { None }
    }

    pub fn _is_done(&self) -> bool {
        if let QuizStatus::Done = self { true } else { false }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct QuizState {
    config: Config,
    root: PathBuf,
    status: QuizStatus,
    users: HashMap<String,HashMap<String,(AnswerType,Score)>>,
}

impl QuizState {
    pub fn new(root: PathBuf) -> Result<Self> {
        Ok(QuizState{
            config: Config::from(&root)?,
            root,
            status: QuizStatus::Lobby,
            users: HashMap::new(),
        })
    }

    pub fn root(&self) -> &PathBuf {
        &self.root
    }

    pub fn title(&self) -> &String {
        &self.config.title
    }
    
    pub fn status(&self) -> &QuizStatus {
        &self.status
    }

    pub fn users(&self) -> impl Iterator<Item=&String> {
        self.users.keys()
    }

    pub fn user_exists(&self, username: &String) -> bool {
        self.users.contains_key(username)
    }
    
    pub fn lobby(&self) -> Option<Event> {
        match self.status() {
            QuizStatus::Lobby => Some(Event::Lobby{users:self.users.keys().map(|s|s.clone()).collect()}),
            _ => None
        }
    }

    pub fn add_user(&mut self, username: String) -> Result<()> {
        if !username.is_empty() && !self.user_exists(&username) {
            self.users.insert(username,HashMap::new());
            Ok(())
        } else { Err(Error::Other) }
    }
    
    pub fn questions(&self) -> Vec<(&String,&QuestionType)> {
        self.config.questions.iter().map(|q|{
            (q.title(),q.type_spec())
        }).collect()
    }
    
    pub fn ranking(&self) -> Ranking {
        let max_score = self.config.questions.iter().fold(0,|acc,q| acc + q.max_score());
        let mut scores: Vec<_> = self.users.iter().map(|(user,answers)|{
            let score = self.config.questions.iter().fold(0,|acc, q| {
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
        if let Some(question) = self.config.questions.get(cur_q) {
            self.users.iter().filter(|(_,answers)|{
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
            QuizStatus::Done => self.config.questions.len()-1
        };
        self.config.questions[0..=cur_q].iter()
                .enumerate()
                .filter(|(_,q)|{
                    self.users.values().fold(false, |acc, answers|{
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
        let question = self.config.questions.get(index)?;
        Some((self.users.iter().map(|(user,answers)|{
            if let Some((answer,score)) = answers.get(question.title()) 
                { (user.clone(),(question.get_answer_string(&answer),*score)) }
            else { (user.clone(),("".to_string(),Score::Ungraded)) }
        }).collect(),question.grade_range().range()))
    }

    pub fn update_grade(&mut self, user: &String, question_title: &String, grade: usize) {
        self.users.get_mut(user)
            .and_then(|answers| answers.get_mut(question_title))
            .map(|(_,score)| *score = Score::Grade(grade));
    }

    fn question(&self, index: usize) -> Option<&Question> {
        self.config.questions.get(index)
    }

    fn question_event(&self, index: usize) -> Option<Event> {
        if index == self.config.questions.len() {
            Some(Event::Finished)
        } else {
            self.question(index).map(|q|{
                Event::Question {
                    title: q.title().clone(), 
                    id:index, total:self.config.questions.len(),
                    image: q.image().and_then(|p| p.to_str().map(|s| s.to_string())), 
                    question_type: q.type_spec().into(),
                }
            })
        }
    }

    fn answer_string(&self, answer: &Answer) -> String {
        match self.config
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
        match &self.status {
            QuizStatus::Lobby => {
                self.question_event(0).map(|e|{
                    self.status = QuizStatus::Question{id:0,locked:false};
                    e
                })
            },
            _ => None
        }
    }
    
    pub fn next(&mut self) -> Option<Event> {
        match self.status {
            QuizStatus::Question{id,..} => {
                self.question_event(id+1).map(|e|{
                    match e {
                        Event::Question{..} => self.status = QuizStatus::Question{id:id+1,locked:false},
                        Event::Finished => self.status = QuizStatus::Done,
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
        match &mut self.status {
            QuizStatus::Question{locked,..} => *locked = true,
            _ => ()
        }
    }
    
    pub fn redo(&mut self, index: usize) -> Option<Event> {
        if index >= self.config.questions.len() {
            None
        } else {
            self.question_event(index).map(|e|{
                    match e {
                        Event::Question{..} => self.status = QuizStatus::Question{id:index,locked:false},
                        _ => ()
                    }
					e
                })
        }
    }

    pub fn submit_answer(&mut self, answer: &String) -> Result<std::result::Result<String,String>> {
        let answer: Answer = serde_json::from_str(answer)?;
        let question_title = answer.question().clone();
        if let (Some(answers),Some((index,question))) =
            (self.users.get_mut(answer.user()), self.config.question(&question_title)) 
        {
            match self.status {
                QuizStatus::Question{id,locked} if id == index => {
                    if locked {
                        return Ok(Err("Could not submit answer: question is locked.".into()));
                    }
                    let answer_type = answer.answer().clone();
                    let score = question.calculate_score(&answer_type).into();
                    answers.insert( question_title, (answer_type, score) );
                    Ok(Ok(self.answer_string(&answer)))
                },
                _ => Err(Error::Other)
            }
        } else { Err(Error::Other) }
    }

    pub fn backup(&self, path: &PathBuf) -> Result<()> {
        let state = serde_json::to_string(self)?;
        Ok(std::fs::write(path, state)?)
    }

    pub fn import_backup(&mut self, path: &PathBuf) -> Result<Option<Event>> {
        let data = std::fs::read_to_string(path)?;
        let state = serde_json::from_str(&data)?;
        *self = state;
        Ok(match &self.status {
            QuizStatus::Question{id,..} => self.question_event(*id),
            QuizStatus::Done => Some(Event::Finished),
            QuizStatus::Lobby => Some(Event::Lobby{users:self.users.keys().map(|s|s.clone()).collect()})
        })
    }
}