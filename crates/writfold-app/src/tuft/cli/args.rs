use super::Context;
use crate::trial::Result;
use crate::tuft::state::{self, Chosen};
use std::path::Path;
use tuft::emit::Format;
use writfold_kb::workspace::{Workspace, discover};

pub(super) struct Args {
    pub positional: Vec<String>,
    pub options: Vec<(String, String)>,
    pub flags: Vec<String>,
    pub format: Format,
}

pub(super) fn parse(args: &[String]) -> Result<Args> {
    let mut parsed = Args {
        positional: Vec::new(),
        options: Vec::new(),
        flags: Vec::new(),
        format: Format::Compact,
    };
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        if arg == "--" {
            parsed.positional.extend(iter.cloned());
            break;
        }
        let Some(name) = arg.strip_prefix("--") else {
            parsed.positional.push(arg.clone());
            continue;
        };
        if matches!(name, "json" | "verbose") {
            if parsed.format != Format::Compact {
                return Err("give at most one of --verbose and --json".into());
            }
            parsed.format = if name == "json" {
                Format::Json
            } else {
                Format::Verbose
            };
        } else if matches!(
            name,
            "cite" | "regex" | "pinned" | "working-tree" | "print-program"
        ) {
            if parsed.flag(name) {
                return Err(format!("--{name} given twice").into());
            }
            parsed.flags.push(name.to_owned());
        } else if matches!(
            name,
            "repo" | "state" | "cursor" | "out" | "kb" | "run" | "at" | "in" | "limit" | "index"
        ) {
            let value = iter
                .next()
                .ok_or_else(|| format!("--{name} needs a value"))?;
            if name != "repo" && parsed.option(name).is_some() {
                return Err(format!("--{name} given twice").into());
            }
            parsed.options.push((name.to_owned(), value.clone()));
        } else {
            return Err(format!("unknown option --{name}").into());
        }
    }
    parsed.validate()?;
    Ok(parsed)
}

impl Args {
    pub fn option(&self, name: &str) -> Option<&str> {
        self.options
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, v)| v.as_str())
    }

    pub fn flag(&self, name: &str) -> bool {
        self.flags.iter().any(|n| n == name)
    }

    pub fn operand(&self) -> Result<&str> {
        match self.positional.as_slice() {
            [_, value] => Ok(value),
            _ => Err("expected exactly one program FILE (or grep PATTERN)".into()),
        }
    }

    fn validate(&self) -> Result<()> {
        match self.positional.first().map(String::as_str) {
            Some("check")
                if !self.options.is_empty()
                    || !self.flags.is_empty()
                    || self.format == Format::Verbose =>
            {
                Err("tuft check accepts only FILE and --json; saved programs must be pinned".into())
            }
            Some("run")
                if self.flag("regex")
                    || self.option("in").is_some()
                    || self.option("limit").is_some() =>
            {
                Err(
                    "--regex, --in and --limit belong to tuft grep; put them in a run program"
                        .into(),
                )
            }
            Some("run") if self.option("at").is_some() && self.operand()? != "-" => {
                Err("--at is for grep or run -; pin the at value in a saved FILE".into())
            }
            Some("help") | None
                if self.positional.len() <= 1
                    && self.options.is_empty()
                    && self.flags.is_empty()
                    && self.format == Format::Compact =>
            {
                Ok(())
            }
            Some("run" | "grep" | "check") => self.operand().map(|_| ()),
            _ => Err("unknown tuft command or arguments; see: writfold tuft help".into()),
        }
    }

    pub fn workspace(&self, context: &Context) -> Result<Option<Workspace>> {
        if let Some(kb) = self.option("kb") {
            return Ok(Some(discover(Some(Path::new(kb)), &context.cwd)?));
        }
        if self.option("run").is_some() {
            return Ok(Some(discover(None, &context.cwd)?));
        }
        Ok(context
            .discover
            .then(|| discover(None, &context.cwd).ok())
            .flatten())
    }

    pub fn state(&self, context: &Context) -> Result<Chosen> {
        state::choose(self.option("state"), &|| self.workspace(context))
    }
}
