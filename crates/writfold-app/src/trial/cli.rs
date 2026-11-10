//! Argument handling for `writfold trial ...`; returns the text to print.
use super::show::show;
use super::{Result, Trial, apply, approve, prepare, recover};
use std::path::{Path, PathBuf};

pub const USAGE: &str = "\
writfold trial init                     create the private trial state directory
writfold trial prepare --repo DIR --file PATH --recipe FILE [--report FILE]
                                        worktree at HEAD + Splicery replay; records Prepared
writfold trial run --repo DIR --file PATH --recipe FILE [--tuft QUERY.tuft]
                  [--weft REVIEW.weft --weft-bin PATH]
                                        tuft run, then Weft (citation \"tuft:@VIEW#N\" names a
                                        retained row), then prepare; stops before approval
writfold trial retain --repo DIR --commit SHA --file PATH --bytes LO..HI
                                        retain exact bytes at a pinned commit; prints the
                                        binding digest a Weft report cites as evidence.
                                        --label NAME also points a local label at it
writfold trial evidence list [--path PREFIX] [--commit SHA] [--limit N] [--out NEW_FILE]
                                        retained bindings as `binding12 path@commit7:line: text`
                                        rows plus a `# count= shown= omitted=` trailer
writfold trial evidence show BINDING [--context-lines N --repo DIR]
                                        the verified exact retained bytes; BINDING is a digest,
                                        a unique 8+ hex prefix or a label; context lines come
                                        from the pinned blob, in separately marked sections
writfold trial evidence labels          local labels and the digests they name
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

fn trial_run(parsed: &Args, state: &Path) -> Result<String> {
    let weft = match (parsed.option("weft"), parsed.option("weft-bin")) {
        (Some(review), Some(bin)) => Some((Path::new(review), Path::new(bin))),
        (None, None) => None,
        _ => return Err("--weft and --weft-bin are given together".into()),
    };
    let ran = super::run::run(&super::run::Request {
        state,
        repo: Path::new(parsed.required("repo")?),
        file: parsed.required("file")?,
        recipe: Path::new(parsed.required("recipe")?),
        tuft: parsed.option("tuft").map(Path::new),
        weft,
        weft_sha256: None,
    })?;
    Ok(super::run::render(&ran))
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
        Some("run") => trial_run(&parsed, &state),
        Some("retain") => {
            if let Some(name) = parsed.option("label") {
                super::evidence::label::check(name)?;
            }
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
            let mut out = format!(
                "retained binding {}\n  {} at {} bytes [{},{})\n  cite in Weft: asset excerpt = citation \"{}\";",
                r.binding, r.path, r.commit, r.range.0, r.range.1, r.binding
            );
            if let Some(name) = parsed.option("label") {
                super::evidence::label::set(&state, name, &r.binding)?;
                out.push_str(&format!("\n  local label {name} (not a citation)"));
            }
            Ok(out)
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
        Some("evidence") => super::evidence::main(&parsed.positional[1..], &parsed.options, &state),
        _ => Ok(USAGE.to_owned()),
    }
}
