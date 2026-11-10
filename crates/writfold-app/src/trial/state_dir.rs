//! Which trial state directory a CLI command uses, and why.
//!
//! `--state DIR`, else a non-empty `WRITFOLD_STATE`, else
//! `~/.local/share/writfold/trial`. The choice is reported with its source and
//! as an absolute path, so the `next:` commands a step prints can be copied
//! into a fresh shell (where an earlier `export` no longer applies) and an
//! error names the directory that was searched.
use super::Result;
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Source {
    Option,
    Environment,
    Default,
}

#[derive(Debug)]
pub struct StateDir {
    pub path: PathBuf,
    pub source: Source,
}

impl StateDir {
    /// Resolve from the `--state` option value and the process environment.
    pub fn resolve(option: Option<&str>) -> Result<Self> {
        let env = std::env::var_os("WRITFOLD_STATE").filter(|v| !v.is_empty());
        let home = std::env::var_os("HOME").filter(|v| !v.is_empty());
        Self::choose(
            option,
            env.as_deref().map(Path::new),
            home.as_deref().map(Path::new),
        )
    }

    fn choose(option: Option<&str>, env: Option<&Path>, home: Option<&Path>) -> Result<Self> {
        let (path, source) = match (option, env) {
            (Some(dir), _) => (PathBuf::from(dir), Source::Option),
            (None, Some(dir)) => (dir.to_owned(), Source::Environment),
            (None, None) => {
                let home = home.ok_or("HOME is not set; pass --state DIR")?;
                (home.join(".local/share/writfold/trial"), Source::Default)
            }
        };
        Ok(Self {
            path: std::path::absolute(&path)?,
            source,
        })
    }

    /// `trial state DIR (from ...)`, for error context.
    pub fn describe(&self) -> String {
        let why = match self.source {
            Source::Option => "from --state",
            Source::Environment => "from WRITFOLD_STATE",
            Source::Default => "the default; --state and WRITFOLD_STATE are unset",
        };
        format!("trial state {} ({why})", self.path.display())
    }

    /// ` --state DIR`, shell-quoted, to append to a printed command.
    pub fn flag(&self) -> String {
        format!(" --state {}", shell_word(&self.path.to_string_lossy()))
    }

    /// Refuse a state directory that `trial init` never created, naming it and
    /// the way forward: a command on an existing RUN was given the wrong
    /// directory; anything else may need a new one.
    pub fn require_initialized(&self, uses: Uses) -> Result<()> {
        if self.path.join("policy.json").is_file() {
            return Ok(());
        }
        let next = match uses {
            Uses::ExistingRun => "  the run was prepared in another state directory: pass the \
                 --state DIR shown on prepare's `next:` line"
                .to_owned(),
            Uses::AnyState => format!(
                "  to create one: writfold trial init{}\n  or pass --state DIR",
                self.flag()
            ),
        };
        Err(format!("no {}\n{next}", self.describe()).into())
    }
}

/// Whether a command acts on a run that an earlier `prepare` recorded.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Uses {
    ExistingRun,
    AnyState,
}

/// A RUN that this state's ledger has no record of.
pub fn not_prepared(run: &str) -> Box<dyn std::error::Error + Send + Sync> {
    format!(
        "run {run} was not prepared in this trial state; pass the --state DIR \
         shown on prepare's `next:` line"
    )
    .into()
}

/// Single-quote a word unless it is plainly safe for a POSIX shell.
fn shell_word(word: &str) -> String {
    let safe = |c: char| c.is_ascii_alphanumeric() || "/._-+=:,@%".contains(c);
    if !word.is_empty() && word.chars().all(safe) {
        word.to_owned()
    } else {
        format!("'{}'", word.replace('\'', r"'\''"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn option_wins_then_environment_then_default() -> Result<()> {
        let home = Path::new("/home/u");
        let env = Path::new("/e");
        let chosen = StateDir::choose(Some("/o"), Some(env), Some(home))?;
        assert_eq!(
            (chosen.path.as_path(), chosen.source),
            (Path::new("/o"), Source::Option)
        );
        let chosen = StateDir::choose(None, Some(env), Some(home))?;
        assert_eq!(chosen.source, Source::Environment);
        let chosen = StateDir::choose(None, None, Some(home))?;
        assert_eq!(chosen.path, home.join(".local/share/writfold/trial"));
        assert!(StateDir::choose(None, None, None).is_err());
        Ok(())
    }

    #[test]
    fn relative_state_becomes_absolute() -> Result<()> {
        let chosen = StateDir::choose(Some("rel/state"), None, None)?;
        assert!(chosen.path.is_absolute());
        assert!(chosen.path.ends_with("rel/state"));
        Ok(())
    }

    #[test]
    fn flag_quotes_unsafe_paths() {
        let dir = |p: &str| StateDir {
            path: PathBuf::from(p),
            source: Source::Option,
        };
        assert_eq!(dir("/tmp/s-1").flag(), " --state /tmp/s-1");
        assert_eq!(dir("/tmp/a b").flag(), " --state '/tmp/a b'");
        assert_eq!(dir("/tmp/it's").flag(), r" --state '/tmp/it'\''s'");
    }

    #[test]
    fn uninitialized_state_is_named() -> Result<()> {
        let dir = StateDir {
            path: PathBuf::from("/nonexistent/writfold-state"),
            source: Source::Environment,
        };
        let Err(error) = dir.require_initialized(Uses::AnyState) else {
            return Err("an uninitialized state was accepted".into());
        };
        let message = error.to_string();
        assert!(message.contains("/nonexistent/writfold-state (from WRITFOLD_STATE)"));
        assert!(message.contains("writfold trial init --state /nonexistent/writfold-state"));
        let Err(error) = dir.require_initialized(Uses::ExistingRun) else {
            return Err("an uninitialized state was accepted".into());
        };
        assert!(
            error
                .to_string()
                .contains("prepared in another state directory")
        );
        assert!(!error.to_string().contains("trial init"));
        Ok(())
    }
}
