use anyhow::{anyhow, Error};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    Todo,
    Doing,
    Waiting,
    Done,
    Canceled,
}

impl Status {
    pub fn is_active(&self) -> bool {
        matches!(self, Status::Todo | Status::Doing | Status::Waiting)
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Status::Todo => "todo",
            Status::Doing => "doing",
            Status::Waiting => "waiting",
            Status::Done => "done",
            Status::Canceled => "canceled",
        }
    }
}

impl std::str::FromStr for Status {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim().to_lowercase();
        match s.as_str() {
            "todo" => Ok(Status::Todo),
            "doing" => Ok(Status::Doing),
            "waiting" => Ok(Status::Waiting),
            "done" => Ok(Status::Done),
            "canceled" | "cancelled" => Ok(Status::Canceled),
            _ => Err(anyhow!("invalid status: {}", s)),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrontMatter {
    pub id: String,
    pub title: String,
    pub status: Status,

    #[serde(default)]
    pub due: Option<String>,

    #[serde(default)]
    pub tags: Vec<String>,

    pub importance: i32,

    pub created_at: String,
    pub updated_at: String,

    #[serde(default)]
    pub done_at: Option<String>,

    /// archive等から戻したときの元パス記録（任意）
    #[serde(default)]
    pub restored_from: Option<String>,
}

#[derive(Debug, Clone)]
pub struct TodoFile {
    pub path: std::path::PathBuf,
    pub fm: FrontMatter,
    pub body: String,
}

impl TodoFile {
    pub fn short_id(&self) -> String {
        self.fm.id.chars().take(12).collect()
    }

    pub fn append_log_line(&mut self, date: &str, message: &str) {
        let line = format!("- {}: {}\n", date, message);

        if !self.body.contains("\n## ログ") && !self.body.starts_with("## ログ") {
            if !self.body.ends_with('\n') {
                self.body.push('\n');
            }
            self.body.push_str("\n## ログ\n");
            self.body.push_str(&line);
            return;
        }

        if self.body.starts_with("## ログ") {
            if let Some(pos) = self.body.find('\n') {
                self.body.insert_str(pos + 1, &line);
            } else {
                self.body.push('\n');
                self.body.push_str(&line);
            }
            return;
        }

        if let Some(h) = self.body.find("\n## ログ") {
            let from = h + 1;
            let after_header = if let Some(eol) = self.body[from..].find('\n') {
                from + eol + 1
            } else {
                self.body.push('\n');
                self.body.len()
            };
            self.body.insert_str(after_header, &line);
            return;
        }

        if !self.body.ends_with('\n') {
            self.body.push('\n');
        }
        self.body.push_str(&line);
    }
}
