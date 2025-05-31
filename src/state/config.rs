use std::path::PathBuf;
use serde::{Deserialize, Serialize};

use crate::error::QuizResult;

use super::Question;


#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    title: String,
    questions: Vec<Question>,
}

impl Config {
    pub fn from(root: &PathBuf) -> QuizResult<Self> {
        let path = root.join("quiz.config");
        let data = std::fs::read_to_string(path)?;
        Ok(serde_json::from_str(&data)?)
    }

    pub fn title(&self) -> &str {
        &self.title
    }

    pub fn question_count(&self) -> usize {
        self.questions.len()
    }

    pub fn questions(&self) -> &Vec<Question> {
        &self.questions
    }
    
    pub fn question(&self, title: &str) -> Option<(usize,&Question)> {
        self.questions.iter().enumerate().filter(|(_,q)| q.title() == title).next()
    }
}

pub fn get_config(root: &str) -> Result<(PathBuf, Config),String> {
	let path = root.to_string();
	std::fs::canonicalize(&path)
		.map_err(|_| format!("Could not find quiz root: {}\n", path))
		.and_then(|root|{
			Config::from(&root)
				.map_err(|e| format!("Could not import quiz.config file: {}\n", e))
				.map(|config| (root, config))
		})
}