//! Shared set-up for the external checker tests: a small KB under Git
//! declaring the checked kind `src`, and a fake checker
//! `writfold-check-acme-src` (a shell script) in a directory put first on
//! `PATH` for the binary only. The fake answers each token as its `want=`
//! field says, appends a line to `$FAKE_COUNTER` each time it starts, and
//! misbehaves as `$FAKE_MODE` says.
#![allow(dead_code)]
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

pub type Result<T = (), E = Box<dyn std::error::Error>> = std::result::Result<T, E>;
pub const WRITFOLD: &str = env!("CARGO_BIN_EXE_writfold");
pub const RUN: &str = "run-20261101T090000Z-a1b2c3";

pub const KIND: &str = "schema = \"writfold-source/1\"\nkind = \"src\"\nfreshness = \"checked\"\nchecker = \"acme-src\"\nfields = [\"depot\", \"rev\", \"path\", \"L\", \"digest\"]\nrequired = [\"depot\", \"path\"]\n";

pub const REVIEW: &str = "schema = \"writfold-gate/1\"\nid = \"review\"\nversion = 1\nclass = \"checklist\"\nscope = [\"notes/sources.md\"]\nitems = [\"src: the cited source is current\"]\n";

pub const FAKE: &str = r#"#!/bin/sh
if [ -n "$FAKE_COUNTER" ]; then echo run >> "$FAKE_COUNTER"; fi
DESCRIBE='{"v":1,"name":"acme-src","version":"1.4.0","kinds":["src"],"ops":["check","make"],"token_versions":[1]}'
case "${FAKE_MODE:-}" in
hang) sleep 600 & echo $! > "$FAKE_PIDFILE"; wait; exit 0 ;;
garbage) echo "$DESCRIBE"; echo 'this is not json'; exit 0 ;;
exit1) echo "$DESCRIBE"; exit 1 ;;
nokind) echo '{"v":1,"name":"acme-src","version":"1.4.0","kinds":["other"],"ops":["check"]}'; exit 0 ;;
esac
field() { printf '%s' "$line" | sed -n "s/.*\"$1\":\"\([^\"]*\)\".*/\1/p"; }
while IFS= read -r line; do
  op=$(field op); id=$(field id); want=$(field want)
  case "$op" in
  describe) echo "$DESCRIBE" ;;
  check)
    if [ -z "$want" ]; then
      if [ "$(field digest)" = d0 ]; then want=fresh; else want=gone; fi
    fi
    case "$want" in
    moved) echo "{\"v\":1,\"id\":\"$id\",\"status\":\"moved\",\"at\":\"r42\",\"moved\":{\"path\":\"lib/new.c\",\"first\":12,\"last\":18}}" ;;
    odd) echo "{\"v\":1,\"id\":\"$id\",\"status\":\"stale\"}" ;;
    dup) echo "{\"v\":1,\"id\":\"$id\",\"status\":\"fresh\"}"; echo "{\"v\":1,\"id\":\"$id\",\"status\":\"fresh\"}" ;;
    silent) ;;
    *) echo "{\"v\":1,\"id\":\"$id\",\"status\":\"$want\",\"at\":\"r42\",\"detail\":\"said $want\"}" ;;
    esac ;;
  make)
    extra=""
    if [ "${FAKE_MODE:-}" = extra ]; then extra=',"owner":"x"'; fi
    echo "{\"v\":1,\"id\":\"$id\",\"fields\":{\"depot\":\"$(field depot)\",\"rev\":\"r42\",\"path\":\"$(field path)\",\"L\":\"$(field L)\",\"digest\":\"d0\",\"v\":\"1\"$extra}}" ;;
  esac
done
"#;

pub struct Root(pub PathBuf);

impl Drop for Root {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

impl Root {
    pub fn kb(&self) -> PathBuf {
        self.0.join("kb")
    }

    pub fn bin(&self) -> PathBuf {
        self.0.join("bin")
    }

    pub fn counter(&self) -> PathBuf {
        self.0.join("counter")
    }

    /// How many times the fake checker started.
    pub fn spawns(&self) -> usize {
        fs::read_to_string(self.counter()).map_or(0, |t| t.lines().count())
    }
}

fn put(path: &Path, text: &str) -> Result {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, text)?;
    Ok(())
}

fn git(dir: &Path, args: &[&str]) -> Result {
    let ok = Command::new("git")
        .current_dir(dir)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_AUTHOR_NAME", "t")
        .env("GIT_AUTHOR_EMAIL", "t@example.invalid")
        .env("GIT_COMMITTER_NAME", "t")
        .env("GIT_COMMITTER_EMAIL", "t@example.invalid")
        .args([
            "-c",
            "commit.gpgsign=false",
            "-c",
            "init.defaultBranch=main",
        ])
        .args(args)
        .status()?
        .success();
    if !ok {
        return Err(format!("git {args:?} failed").into());
    }
    Ok(())
}

/// A KB declaring `src`, with `tokens` in `notes/sources.md`, committed;
/// the fake checker installed when `fake`.
pub fn world(name: &str, tokens: &[&str], fake: bool) -> Result<Root> {
    let dir = std::env::temp_dir().join(format!("writfold-checker-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(dir.join("bin"))?;
    let root = Root(fs::canonicalize(&dir)?);
    let kb = root.kb();
    put(
        &kb.join("inventory/assets/demo.json"),
        "{\"id\": \"demo\", \"kind\": \"service\", \"name\": \"Demo\"}\n",
    )?;
    put(&kb.join("harbor/README.md"), "# Harbor\n")?;
    put(
        &kb.join(format!("runs/{RUN}/manifest.json")),
        &format!(
            "{{\"run_id\": \"{RUN}\", \"task\": \"t\", \"project\": \"harbor\", \"mode\": \"authoring\", \"created\": \"2026-11-01T09:00:00Z\", \"state\": \"open\"}}\n"
        ),
    )?;
    put(&kb.join(".writfold/sources/src.toml"), KIND)?;
    put(&kb.join(".writfold/gates/review.toml"), REVIEW)?;
    let body: String = tokens.iter().map(|t| format!("{t}\n\n")).collect();
    put(
        &kb.join("notes/sources.md"),
        &format!("# Sources\n\n{body}"),
    )?;
    git(&kb, &["init", "-q"])?;
    git(&kb, &["add", "-A"])?;
    git(&kb, &["commit", "-q", "-m", "kb"])?;
    if fake {
        let file = root.bin().join("writfold-check-acme-src");
        fs::write(&file, FAKE)?;
        fs::set_permissions(&file, fs::Permissions::from_mode(0o755))?;
    }
    Ok(root)
}

/// Run the binary with the fake's directory first on `PATH`.
pub fn run(root: &Root, args: &[&str], env: &[(&str, &str)], stdin: &str) -> Result<Output> {
    let path = format!(
        "{}:{}",
        root.bin().display(),
        std::env::var("PATH").unwrap_or_default()
    );
    let mut command = Command::new(WRITFOLD);
    command
        .args(args)
        .current_dir(root.kb())
        .env("PATH", path)
        .env("HOME", &root.0)
        .env("XDG_CONFIG_HOME", &root.0)
        .env("FAKE_COUNTER", root.counter())
        .env_remove("WRITFOLD_KB")
        .env_remove("FAKE_MODE")
        .env_remove("WRITFOLD_CHECKER_TIMEOUT")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    for (key, value) in env {
        command.env(key, value);
    }
    let mut child = command.spawn()?;
    if let Some(mut input) = child.stdin.take() {
        std::io::Write::write_all(&mut input, stdin.as_bytes())?;
    }
    Ok(child.wait_with_output()?)
}

/// `writfold kb <args>`: stdout and the exit code.
pub fn kb(root: &Root, args: &[&str], env: &[(&str, &str)]) -> Result<(String, i32)> {
    let mut all = vec!["kb"];
    all.extend_from_slice(args);
    let out = run(root, &all, env, "")?;
    let text =
        String::from_utf8_lossy(&out.stdout).into_owned() + &String::from_utf8_lossy(&out.stderr);
    Ok((text, out.status.code().unwrap_or(-1)))
}

/// The output row for the token at `path=`.
pub fn row<'a>(text: &'a str, path: &str) -> &'a str {
    text.lines()
        .find(|l| l.starts_with(&format!("cite src:{path} ")))
        .unwrap_or("")
}

pub fn cached_rows(w: &Root) -> Result<Vec<u8>> {
    let script = "import pathlib,sqlite3,sys,json\n\
        files=list(pathlib.Path(sys.argv[1]).rglob('kb-index-*.sqlite'))\n\
        assert len(files)==1, files\n\
        db=sqlite3.connect('file:'+str(files[0])+'?mode=ro',uri=True)\n\
        print(json.dumps(db.execute('SELECT * FROM cite_checks ORDER BY path,ord').fetchall()))";
    let out = std::process::Command::new("python3")
        .args(["-c", script])
        .arg(w.kb().join(".git"))
        .output()?;
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    Ok(out.stdout)
}
