use serde::{Deserialize, Serialize};

use super::question::QuestionSendType;


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

#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub enum Score{
    Ungraded,
    Grade(usize)
}

impl Score {
    pub fn is_ungraded(&self) -> bool {
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

#[derive(Debug, Clone, Serialize, Deserialize)]
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