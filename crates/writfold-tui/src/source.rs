//! The verbs, called in-process. Every pane of `writfold watch` is the
//! JSON answer of one `writfold kb` verb, obtained through
//! [`writfold_kb::run`] exactly as the CLI obtains it; nothing here decides
//! a state, counts an obligation or reads a KB file itself.
use serde_json::Value;
use std::path::PathBuf;
use writfold_kb::gate::GateVerb;
use writfold_kb::workspace::{GitTool, discover};
use writfold_kb::{Request, Verb};

/// Where the verbs run: the KB (`--kb`, else the CLI's discovery from the
/// working directory) and a fixed clock for tests.
#[derive(Clone, Debug)]
pub struct Source {
    pub kb: Option<PathBuf>,
    pub cwd: PathBuf,
    pub now: Option<i64>,
}

/// One verb's answer: its rows, notes and trailer as the JSON form prints
/// them, its exit code, or the error it refused with.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Reply {
    pub rows: Vec<Value>,
    pub notes: Vec<String>,
    pub trailer: Value,
    pub exit: u8,
    pub error: Option<String>,
}

impl Reply {
    fn failed(message: String, exit: u8) -> Self {
        Self {
            error: Some(message),
            exit,
            ..Self::default()
        }
    }

    /// A trailer field as text (`kb`, `index`, …).
    pub fn trailer_text(&self, key: &str) -> Option<String> {
        self.trailer
            .get(key)
            .and_then(Value::as_str)
            .map(str::to_owned)
    }

    /// The trailer's extra counts (`gates=4 stale=1 …`), by name.
    pub fn counts(&self) -> Vec<(String, u64)> {
        self.trailer
            .get("counts")
            .and_then(Value::as_object)
            .map(|map| {
                map.iter()
                    .filter_map(|(k, v)| v.as_u64().map(|n| (k.clone(), n)))
                    .collect()
            })
            .unwrap_or_default()
    }
}

/// The text of a row field (`kind`, `id`, `status`, `text`, `path`).
pub fn field<'a>(row: &'a Value, key: &str) -> &'a str {
    row.get(key).and_then(Value::as_str).unwrap_or_default()
}

/// The row's flags (`stale`, `failed`, `current`, …).
pub fn flags(row: &Value) -> Vec<&str> {
    row.get("flags")
        .and_then(Value::as_array)
        .map(|list| list.iter().filter_map(Value::as_str).collect())
        .unwrap_or_default()
}

impl Source {
    /// Answer one verb with the whole answer on one page.
    pub fn call(&self, verb: Verb) -> Reply {
        let request = Request {
            verb,
            kb: self.kb.clone(),
            cwd: self.cwd.clone(),
            budget: writfold_kb::MAX_BUDGET,
            cursor: None,
            json: true,
            now: self.now,
            probe: writfold_kb::write::no_probe,
        };
        match writfold_kb::run(&request) {
            Ok(output) => match serde_json::from_str::<Value>(&output.text) {
                Ok(body) => Reply {
                    rows: list(&body, "rows"),
                    notes: list(&body, "notes")
                        .iter()
                        .filter_map(Value::as_str)
                        .map(str::to_owned)
                        .collect(),
                    trailer: body.get("trailer").cloned().unwrap_or_default(),
                    exit: output.exit,
                    error: None,
                },
                Err(error) => Reply::failed(format!("unreadable answer: {error}"), 1),
            },
            Err(error) => Reply::failed(error.to_string(), error.exit_code()),
        }
    }

    /// `writfold kb runs`.
    pub fn runs(&self) -> Reply {
        self.call(Verb::Runs {
            open: false,
            project: None,
            plan: false,
        })
    }

    /// `writfold kb resume <run>`.
    pub fn resume(&self, run: &str) -> Reply {
        self.call(Verb::Resume {
            run: Some(run.to_owned()),
        })
    }

    /// `writfold kb gate status --run <run>`.
    pub fn gates(&self, run: &str) -> Reply {
        self.call(Verb::Gate(GateVerb::Status {
            run: Some(run.to_owned()),
            repo: None,
        }))
    }

    /// `writfold kb stale <project> --cached`: never starts a checker.
    pub fn stale(&self, project: Option<&str>) -> Reply {
        self.call(Verb::Stale {
            scope: project.map(str::to_owned),
            offline: false,
            cached: true,
        })
    }

    /// `writfold kb brief <project>` (inferred from the working directory
    /// when the run names none).
    pub fn brief(&self, project: Option<&str>) -> Reply {
        self.call(Verb::Brief {
            project: project.map(str::to_owned),
            all: false,
        })
    }

    /// The KB pin's digest (HEAD plus the digests of dirty KB files), from
    /// one `git status`; `none` outside Git. Polled to decide whether to
    /// ask the verbs again.
    pub fn pin(&self, tool: &mut Option<GitTool>) -> String {
        let ws = match discover(self.kb.as_deref(), &self.cwd) {
            Ok(ws) => ws,
            Err(error) => return format!("error: {error}"),
        };
        let Some(repo) = &ws.repo else {
            return "none".into();
        };
        if tool.is_none() {
            match GitTool::locate() {
                Ok(found) => *tool = Some(found),
                Err(error) => return format!("error: {error}"),
            }
        }
        let observed = tool
            .as_ref()
            .ok_or_else(|| "git unavailable".to_owned())
            .and_then(|t| t.admit().map_err(|e| e.to_string()))
            .and_then(|git| {
                writfold_kb::index::observe(&ws, repo, &git).map_err(|e| e.to_string())
            });
        match observed {
            Ok(snapshot) => snapshot.pin.digest,
            Err(error) => format!("error: {error}"),
        }
    }
}

fn list(body: &Value, key: &str) -> Vec<Value> {
    body.get(key)
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
}
