//! The verbs, called in-process. Every pane of `writfold watch` is the
//! JSON answer of one `writfold kb` verb, obtained through
//! [`writfold_kb::run`] exactly as the CLI obtains it; nothing here decides
//! a state, counts an obligation or reads a KB file itself.
use serde_json::Value;
use std::path::PathBuf;
use writfold_kb::gate::GateVerb;
use writfold_kb::workspace::{discover, find_repo};
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

    pub fn count(&self, name: &str) -> Option<u64> {
        self.trailer.get("counts")?.get(name)?.as_u64()
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
    /// Discover the KB before applying an optional gate-subject working directory.
    pub fn located(
        kb: Option<PathBuf>,
        cwd: PathBuf,
        repo: Option<PathBuf>,
        now: Option<i64>,
    ) -> Result<Self, String> {
        let ws = discover(kb.as_deref(), &cwd).map_err(|e| e.to_string())?;
        let cwd = match repo {
            None => cwd,
            Some(repo) => {
                let named = cwd.join(repo);
                let dir = std::fs::canonicalize(&named)
                    .map_err(|e| format!("--repo {}: {e}", named.display()))?;
                if !dir.is_dir() || find_repo(&dir).map_err(|e| e.to_string())?.is_none() {
                    return Err(format!(
                        "--repo {} is not in a Git work tree",
                        named.display()
                    ));
                }
                dir
            }
        };
        Ok(Self {
            kb: Some(ws.root),
            cwd,
            now,
        })
    }

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

    /// `writfold kb runs --plan`, with the verb's own filters.
    pub fn runs(&self, open: bool, project: Option<&str>) -> Reply {
        self.call(Verb::Runs {
            open,
            project: project.map(str::to_owned),
            plan: true,
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

    pub fn guards(&self) -> Reply {
        self.call(Verb::Guard(writfold_kb::guard::GuardVerb::Status))
    }

    pub fn plan(&self, run: &str) -> Reply {
        self.call(Verb::Plan(writfold_kb::plan::PlanVerb::Status {
            run: Some(run.to_owned()),
            repo: None,
            gates: true,
        }))
    }

    pub fn timeline(&self, run: &str) -> Reply {
        self.call(Verb::Timeline {
            target: run.to_owned(),
        })
    }

    pub fn clock(&self) -> String {
        let stamp = writfold_kb::time::stamp(self.now.unwrap_or_else(writfold_kb::time::now));
        stamp.get(11..).unwrap_or(&stamp).to_owned()
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
}

fn list(body: &Value, key: &str) -> Vec<Value> {
    body.get(key)
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
}
