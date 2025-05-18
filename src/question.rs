
use serde::{Serialize, Deserialize};
use std::path::PathBuf;

fn stringify_answers(options: &Vec<String>, answers: &Vec<usize>) -> String {
	let mut first = true;
    answers.iter()
        .map(|i| options[*i].clone())
        .fold("".to_string(),|acc,opt| {
            acc + &match first {
                false => format!(", {}", opt),
                true => { first=false; opt },
            }
        })
}

#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub struct GradeRange {
    min: usize,
    max: usize,
}

impl GradeRange {
    pub fn range(&self) -> std::ops::RangeInclusive<usize> {
        self.min..=self.max
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Question {
    title: String,
    image: Option<PathBuf>,
    grade_range: GradeRange,
    type_spec: QuestionType,
}

impl Question {
    pub fn title(&self) -> &String {
        &self.title
    }

    pub fn image(&self) -> Option<&PathBuf> {
        self.image.as_ref()
    }

    pub fn grade_range(&self) -> GradeRange {
        self.grade_range
    }

    pub fn type_spec(&self) -> &QuestionType {
        &self.type_spec
    }
}

impl Question {
    pub fn max_score(&self) -> usize {
        self.grade_range.max
    }

    pub fn calculate_score(&self, answer: &AnswerType) -> Option<usize> {
        match (&self.type_spec, answer) {
            (QuestionType::MultiChoice{answer:correct_answer,..},AnswerType::MultiChoice(answer))
                => if answer == correct_answer { Some(self.max_score()) } else { Some(0) },
            (QuestionType::MultiOption{answers:correct_answers,..},AnswerType::MultiOption(answers))
                => if answers == correct_answers { Some(self.max_score()) } else { Some(0) },
            (QuestionType::Open,AnswerType::Open(_)) => None,
            _ => None
        }
    }

    pub fn get_answer_string(&self, answer: &AnswerType) -> String {
        match (&self.type_spec, answer) {
            (QuestionType::MultiChoice{options,..},AnswerType::MultiChoice(answer))
                => options[*answer].clone(),
            (QuestionType::MultiOption{options,..},AnswerType::MultiOption(answers))
                => stringify_answers(options,answers),
            (QuestionType::Open,AnswerType::Open(answer))
                => answer.clone(),
            _ => "".to_string()
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum QuestionType {
    MultiChoice {
        options: Vec<String>,
        answer: usize,
    },
    MultiOption {
        options: Vec<String>,
        answers: Vec<usize>,
    },
    Open
}

impl std::fmt::Display for QuestionType {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        match self {
            QuestionType::MultiChoice { options, answer } 
				=> f.write_fmt(format_args!("MultiChoice\nexpected answer: {}", options[*answer])),
            QuestionType::MultiOption { options, answers } 
				=> f.write_fmt(format_args!("MultiOption\nexpected answer: {}", stringify_answers(options,answers))),
            QuestionType::Open => f.write_str("Open"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum QuestionSendType {
    MultiChoice(Vec<String>),
    MultiOption(Vec<String>),
    Open
}

impl From<&QuestionType> for QuestionSendType {
    fn from(q: &QuestionType) -> Self {
        match q {
            QuestionType::MultiChoice{options,..} => QuestionSendType::MultiChoice(options.clone()),
            QuestionType::MultiOption{options,..} => QuestionSendType::MultiOption(options.clone()),
            QuestionType::Open => QuestionSendType::Open
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Answer {
    user: String,
    question: String,
    answer: AnswerType,
}

impl Answer {
    pub fn user(&self) -> &String {
        &self.user
    }

    pub fn question(&self) -> &String {
        &self.question
    }

    pub fn answer(&self) -> &AnswerType {
        &self.answer
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum AnswerType {
    MultiChoice(usize),
    MultiOption(Vec<usize>),
    Open(String)
}