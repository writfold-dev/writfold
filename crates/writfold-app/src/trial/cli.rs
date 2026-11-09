//! Argument handling for `writfold trial ...`; returns the text to print.
use super::show::show;
use super::{Result, Trial, apply, approve, prepare, recover};
use std::path::{Path, PathBuf};

pub const USAGE: &str = "\
writfold trial init                     create the private trial state directory
writfold trial prepare --repo DIR --file PATH --recipe FILE [--report FILE]
                                        worktree at HEAD + Splicery replay; records Prepared
writfold trial retain --repo DIR --commit SHA --file PATH --bytes LO..HI
                                        retain exact bytes at a pinned commit; prints the
                                        binding digest a Weft report cites as evidence
writfold trial approve RUN [--require CHECK]...
                                        sign a grant and an approval; records Approved.
                                        Each --require names a Weft check that must be entailed
writfold trial apply RUN                re-verify, claim, check the file, install; records Applied
writfold trial recover RUN              resolve a claim left without an outcome (e.g. after a crash)
writfold trial show RUN                 print the run's validated ledger history

Options: --state DIR (default: $WRITFOLD_STATE, else ~/.local/share/writfold/trial).
Profile writfold-trial-worktree/1: a cooperating single-user trial. Edits land only
in Writfold's own worktree of your repository, never in your checkout.";

struct Args {
    positional: Vec<String>,
    options: Vec<(String, String)>,
}

fn parse(args: &[String]) -> Result<Args> {
    let mut parsed = Args {
        positional: Vec::new(),
        options: Vec::new(),
    };
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        if let Some(name) = arg.strip_prefix("--") {
            let value = iter
                .next()
                .ok_or_else(|| format!("--{name} needs a value"))?;
            parsed.options.push((name.to_owned(), value.clone()));
        } else {
            parsed.positional.push(arg.clone());
        }
    }
    Ok(parsed)
}

impl Args {
    fn option(&self, name: &str) -> Option<&str> {
        self.options
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, v)| v.as_str())
    }
    fn all(&self, name: &str) -> Vec<String> {
        self.options
            .iter()
            .filter(|(n, _)| n == name)
            .map(|(_, v)| v.clone())
            .collect()
    }
    fn required(&self, name: &str) -> Result<&str> {
        self.option(name)
            .ok_or_else(|| format!("missing --{name}").into())
    }
    fn run(&self) -> Result<&str> {
        match self.positional.as_slice() {
            [_, run] => Ok(run),
            _ => Err("expected exactly one RUN".into()),
        }
    }
    fn state(&self) -> Result<PathBuf> {
        if let Some(state) = self
            .option("state")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("WRITFOLD_STATE").map(PathBuf::from))
        {
            return Ok(state);
        }
        let home = std::env::var_os("HOME").ok_or("HOME is not set; pass --state")?;
        Ok(Path::new(&home).join(".local/share/writfold/trial"))
    }
}

pub fn main(args: &[String]) -> Result<String> {
    let parsed = parse(args)?;
    let state = parsed.state()?;
    match parsed.positional.first().map(String::as_str) {
        Some("init") => {
            Trial::init(&state)?;
            Ok(format!("initialized trial state in {}", state.display()))
        }
        Some("prepare") => {
            let report = parsed.option("report").map(Path::new);
            let p = prepare(
                &state,
                Path::new(parsed.required("repo")?),
                parsed.required("file")?,
                Path::new(parsed.required("recipe")?),
                report,
            )?;
            let mut out = format!(
                "prepared run {}\n  file {} at {}\n  prepared edit {}\n",
                p.run, p.path, p.base, p.prepared
            );
            for (lo, hi, inserted) in &p.edits {
                out.push_str(&format!("  splice [{lo},{hi}) -> {inserted} bytes\n"));
            }
            out.push_str(&format!("next: writfold trial approve {}", p.run));
            Ok(out)
        }
        Some("retain") => {
            let (lo, hi) = parsed
                .required("bytes")?
                .split_once("..")
                .ok_or("--bytes takes LO..HI")?;
            let r = super::retain::retain(
                &state,
                Path::new(parsed.required("repo")?),
                parsed.required("commit")?,
                parsed.required("file")?,
                (lo.parse()?, hi.parse()?),
            )?;
            Ok(format!(
                "retained binding {}\n  {} at {} bytes [{},{})\n  cite in Weft: asset excerpt = citation \"{}\";",
                r.binding, r.path, r.commit, r.range.0, r.range.1, r.binding
            ))
        }
        Some("approve") => {
            let run = parsed.run()?;
            let (expires, summary) = approve(&state, run, &parsed.all("require"))?;
            Ok(format!(
                "approved run {run} until {expires} (unix seconds)\n  {summary}\nnext: writfold trial apply {run}"
            ))
        }
        Some("apply") => {
            let run = parsed.run()?;
            let worktree = apply(&state, run)?;
            Ok(format!(
                "applied run {run}\n  inspect: git -C {} diff",
                worktree.display()
            ))
        }
        Some("recover") => {
            let run = parsed.run()?;
            Ok(format!("run {run}: {}", recover(&state, run)?))
        }
        Some("show") => show(&state, parsed.run()?),
        _ => Ok(USAGE.to_owned()),
    }
}
