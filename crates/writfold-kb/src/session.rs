//! One invocation: the located KB, its admitted Git, and the index opened
//! at the observed snapshot.
use crate::emit::{self, Answer, Frame, Output, Scope};
use crate::error::{KbError, Result};
use crate::index::{self, IndexState, Pin, query};
use crate::workspace::{GitTool, Workspace};
use rusqlite::Connection;
use std::path::PathBuf;
use writfold_runtime::git::GitExecutable;

pub struct Session<'a> {
    pub ws: &'a Workspace,
    pub git: Option<GitExecutable<'a>>,
    pub conn: Connection,
    pub pin: Pin,
    pub index: IndexState,
    pub file: Option<PathBuf>,
    /// Seconds since the epoch, for `ttl` checks.
    pub now: i64,
}

impl<'a> Session<'a> {
    /// Admit Git (when the KB is under Git) and open the index, rebuilding
    /// it when the snapshot moved or `rebuild` is set.
    pub fn open(
        ws: &'a Workspace,
        tool: Option<&'a GitTool>,
        rebuild: bool,
        now: i64,
    ) -> Result<Self> {
        let git = match (&ws.repo, tool) {
            (Some(_), Some(tool)) => Some(tool.admit()?),
            (Some(_), None) => {
                return Err(KbError::Usage(
                    "the KB is under Git but no git executable was supplied".into(),
                ));
            }
            (None, _) => None,
        };
        let opened = index::open(ws, git.as_ref(), rebuild)?;
        Ok(Self {
            ws,
            git,
            conn: opened.conn,
            pin: opened.pin,
            index: opened.state,
            file: opened.file,
            now,
        })
    }

    /// The current text of a KB file (the snapshot the pin names).
    pub fn text(&self, path: &str) -> Option<String> {
        std::fs::read_to_string(self.ws.root.join(path)).ok()
    }

    /// Lines `first..=last` (one-based) of a KB file.
    pub fn lines(&self, path: &str, first: u32, last: u32) -> Vec<String> {
        let skip = usize::try_from(first.saturating_sub(1)).unwrap_or(usize::MAX);
        let take = usize::try_from(last.saturating_sub(first).saturating_add(1)).unwrap_or(0);
        self.text(path)
            .map(|text| {
                text.lines()
                    .skip(skip)
                    .take(take)
                    .map(str::to_owned)
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Page and render an answer against this snapshot.
    pub fn emit(&self, answer: &Answer, frame: &Frame) -> Result<Output> {
        let (examined, total, unreadable) = query::scope(&self.conn)?;
        let scope = Scope {
            pin: &self.pin,
            index: self.index,
            examined,
            total,
            unreadable,
        };
        emit::emit(answer, frame, &scope)
    }
}
