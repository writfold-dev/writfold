//! KB and local stamps every two seconds, the subject every fifth tick.
use crate::source::Source;
use writfold_kb::watch::Stamp;
use writfold_kb::workspace::{GitTool, discover};

pub type Subject = Result<Option<Stamp>, String>;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Stamps {
    pub pin: String,
    pub local: String,
    pub subject: Subject,
}

impl Default for Stamps {
    fn default() -> Self {
        Self {
            pin: String::new(),
            local: String::new(),
            subject: Ok(None),
        }
    }
}

fn tool(found: &mut Option<GitTool>) -> Result<&GitTool, String> {
    if found.is_none() {
        *found = Some(GitTool::locate().map_err(|e| e.to_string())?);
    }
    found.as_ref().ok_or_else(|| "Git unavailable".into())
}

impl Source {
    /// The KB pin: HEAD and dirty KB contents, as observed by the index.
    pub fn pin(&self, found: &mut Option<GitTool>) -> String {
        let observe = || -> Result<String, String> {
            let ws = discover(self.kb.as_deref(), &self.cwd).map_err(|e| e.to_string())?;
            let Some(repo) = &ws.repo else {
                return Ok("none".into());
            };
            let git = tool(found)?.admit().map_err(|e| e.to_string())?;
            writfold_kb::index::observe(&ws, repo, &git)
                .map(|s| s.pin.digest)
                .map_err(|e| e.to_string())
        };
        let mut observe = observe;
        observe().unwrap_or_else(|error| format!("error: {error}"))
    }

    pub fn local(&self) -> String {
        match discover(self.kb.as_deref(), &self.cwd) {
            Ok(ws) => writfold_kb::watch::local(&ws),
            Err(error) => format!("error: {error}"),
        }
    }

    pub fn subject(&self, found: &mut Option<GitTool>, recorded: Option<&str>) -> Subject {
        let ws = discover(self.kb.as_deref(), &self.cwd).map_err(|e| e.to_string())?;
        writfold_kb::watch::subject(&ws, tool(found)?, &self.cwd, recorded)
            .map_err(|e| e.to_string())
    }
}

pub fn subject_text(stamp: &Subject) -> String {
    match stamp {
        Ok(Some(s)) => format!(
            "subject={}@{}+dirty:{}",
            s.name,
            s.head.get(..12).unwrap_or(&s.head),
            s.dirty
        ),
        Ok(None) => "subject=none".into(),
        Err(error) => format!("subject=error: {error}"),
    }
}
