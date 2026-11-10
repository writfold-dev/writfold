//! `trial/run`, `trial/evidence-list` and `trial/evidence-show` over real
//! `writfold serve` pipes: each round trip gives what the CLI prints for the
//! same input, `trial/run` stops at a prepared edit, tampered evidence refuses
//! and an unnegotiated session refuses all three.
use serde_json::{Value, json};
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::fs::DirBuilderExt;
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, ChildStdout, Command, Output, Stdio};

type Result<T = (), E = Box<dyn std::error::Error>> = std::result::Result<T, E>;
const WRITFOLD: &str = env!("CARGO_BIN_EXE_writfold");
const SOURCE: &str = "let a = 1;\nrun(1);\n";
const RECIPE: &str =
    "edit \"src/lib.rs\" {\n  @n := one text \"run(1)\"\n  replace @n with \"run(2)\"\n}\n";
/// A stub Weft: a fixed complete report citing the first `citation` it reads.
const STUB: &str = r#"#!/bin/sh
[ "$1 $2 $3" = "prove --json --" ] || exit 9
ref=$(sed -n 's/.*citation "\([^"]*\)".*/\1/p' "$4" | head -n 1)
printf '{"schema":"weft.report/1","kind":"assessment","scope":"local-symbolic","execution":"complete","writfold_grounded":false,"effect_authority":false,"review":{"bindings":[{"kind":"asset","reference":"%s"}]},"checks":[{"name":"ok","result":"entailed"},{"name":"bad","result":"refuted"}]}' "$ref"
"#;
const NEW: [&str; 3] = ["trial/run", "trial/evidence-list", "trial/evidence-show"];

struct Fixture {
    root: PathBuf,
    repo: PathBuf,
    state: PathBuf,
    commit: String,
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn sh(script: &str, args: &[&str]) -> Result<String> {
    let out = Command::new("sh")
        .args(["-c", script, "sh"])
        .args(args)
        .env("GIT_AUTHOR_NAME", "t")
        .env("GIT_AUTHOR_EMAIL", "t@example.invalid")
        .env("GIT_COMMITTER_NAME", "t")
        .env("GIT_COMMITTER_EMAIL", "t@example.invalid")
        .output()?;
    if !out.status.success() {
        return Err(format!("{script} failed").into());
    }
    Ok(String::from_utf8(out.stdout)?.trim_end().to_owned())
}

fn path(p: &Path) -> Result<&str> {
    p.to_str().ok_or_else(|| "non-UTF-8 temp path".into())
}

fn cli(args: &[&str], state: &Path) -> Result<Output> {
    Ok(Command::new(WRITFOLD)
        .args(args)
        .arg("--state")
        .arg(state)
        .output()?)
}

fn stdout(args: &[&str], state: &Path) -> Result<String> {
    let out = cli(args, state)?;
    if !out.status.success() {
        return Err(format!("{args:?}: {}", String::from_utf8_lossy(&out.stderr)).into());
    }
    Ok(String::from_utf8(out.stdout)?)
}

fn fixture(name: &str) -> Result<Fixture> {
    let root = std::env::temp_dir().join(format!(".writfold-serve-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::DirBuilder::new().mode(0o700).create(&root)?;
    let repo = root.join("repo");
    fs::create_dir_all(repo.join("src"))?;
    fs::write(repo.join("src/lib.rs"), SOURCE)?;
    let dir = path(&repo)?;
    sh(
        r#"cd "$1" && git init -q && git add -A && git commit -qm f"#,
        &[dir],
    )?;
    let commit = sh(r#"git -C "$1" rev-parse HEAD"#, &[dir])?;
    let state = root.join("state");
    fs::write(root.join("edit.spl"), RECIPE)?;
    let tuft = format!(
        "@h := find text \"run(\" at \"{commit}\";\n@e := retain @h;\n\
@v := project @e [path, line, binding];\nemit @v;\n"
    );
    fs::write(root.join("q.tuft"), tuft)?;
    let review = format!(
        "review t {{\n    context \"trial\" at \"{commit}\";\n    profile \"weft.symbolic/1\";\n    \
target change = proposal \"run-2\";\n    asset call = citation \"tuft:@v#1\";\n    \
assume a: ok(\"x\");\n    check ok: ok(\"x\");\n}}\n"
    );
    fs::write(root.join("review.weft"), review)?;
    // Written by a child process, so no descriptor of this process holds it.
    let weft = root.join("weft");
    sh(
        r#"printf '%s' "$1" > "$2" && chmod 755 "$2""#,
        &[STUB, path(&weft)?],
    )?;
    stdout(&["trial", "init"], &state)?;
    Ok(Fixture {
        root,
        repo,
        state,
        commit,
    })
}

struct Server {
    child: Child,
    input: ChildStdin,
    output: BufReader<ChildStdout>,
    next: u64,
}

impl Server {
    fn start(state: &Path, capabilities: Value) -> Result<Self> {
        let mut child = Command::new(WRITFOLD)
            .args(["serve", "--state"])
            .arg(state)
            .env_remove("WRITFOLD_STATE")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()?;
        let input = child.stdin.take().ok_or("stdin")?;
        let output = BufReader::new(child.stdout.take().ok_or("stdout")?);
        let mut server = Self {
            child,
            input,
            output,
            next: 0,
        };
        let params = json!({"protocol_version": "2", "capabilities": capabilities});
        server.call("initialize", params)?["result"]
            .as_object()
            .ok_or("initialize refused")?;
        Ok(server)
    }

    fn call(&mut self, method: &str, params: Value) -> Result<Value> {
        self.next += 1;
        let request =
            json!({"jsonrpc": "2.0", "id": self.next, "method": method, "params": params});
        writeln!(self.input, "{request}")?;
        self.input.flush()?;
        let mut line = String::new();
        self.output.read_line(&mut line)?;
        if !line.ends_with('\n') || line.len() > 65_536 {
            return Err("reply is not one bounded LF-terminated frame".into());
        }
        let reply: Value = serde_json::from_str(&line)?;
        if reply["id"] != json!(self.next) {
            return Err(format!("reply lost its correlation: {reply}").into());
        }
        Ok(reply)
    }

    fn stop(mut self) -> Result {
        self.call("shutdown", json!({}))?;
        if !self.child.wait()?.success() {
            return Err("serve did not exit cleanly".into());
        }
        Ok(())
    }
}

/// The printed `trial run` summary with its run id and prepared digest, which
/// differ per run, replaced.
fn normalized(text: &str, run: &str) -> String {
    text.lines()
        .map(|line| match line.strip_prefix("  prepared edit ") {
            Some(_) => "  prepared edit P".to_owned(),
            None => line.replace(run, "RUN"),
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn run_id(printed: &str) -> Result<&str> {
    let line = printed
        .lines()
        .find_map(|l| l.strip_prefix("prepared run "));
    line.ok_or_else(|| "no run id".into())
}

/// `fixed` split on spaces, then `rest` (host paths) as whole arguments.
fn argv<'a>(fixed: &'a str, rest: &[&'a str]) -> Vec<&'a str> {
    fixed.split(' ').chain(rest.iter().copied()).collect()
}

/// A serve refusal that carries exactly the CLI's message for the same input;
/// returns its (category, outcome, message).
fn same_refusal(reply: &Value, printed: Output) -> Result<(String, String, String)> {
    let data = &reply["error"]["data"];
    let message = data["message"].as_str().ok_or("message")?.to_owned();
    let stderr = String::from_utf8(printed.stderr)?;
    if reply["error"]["code"] != -32000
        || printed.status.success()
        || stderr != format!("writfold: {message}\n")
    {
        return Err(format!("serve refused {reply}, the CLI printed {stderr:?}").into());
    }
    let text = |key: &str| data[key].as_str().unwrap_or_default().to_owned();
    Ok((text("category"), text("outcome"), message))
}

/// Retain `run(1);` with the CLI under the label `entry`; its full digest.
fn retain_entry(f: &Fixture) -> Result<String> {
    let rest = [path(&f.repo)?, "--commit", &f.commit];
    let fixed = "trial retain --file src/lib.rs --bytes 11..18 --label entry --repo";
    let retained = stdout(&argv(fixed, &rest), &f.state)?;
    let binding = retained.split_whitespace().nth(2).ok_or("binding")?;
    Ok(binding.to_owned())
}

#[test]
fn serve_run_stops_at_a_prepared_edit_and_matches_the_cli() -> Result {
    let f = fixture("run")?;
    let mut server = Server::start(&f.state, json!(["writfold.trial/1"]))?;
    let (repo, recipe) = (path(&f.repo)?, f.root.join("edit.spl"));
    let (recipe, tuft) = (path(&recipe)?, f.root.join("q.tuft"));
    let (review, weft) = (f.root.join("review.weft"), f.root.join("weft"));
    let (tuft, review, weft) = (path(&tuft)?, path(&review)?, path(&weft)?);
    let reply = server.call(
        "trial/run",
        json!({"repo": repo, "file": "src/lib.rs", "recipe": recipe,
               "tuft": tuft, "weft": review, "weft_bin": weft}),
    )?;
    let payload = &reply["result"]["payload"];
    let run = payload["run"].as_str().ok_or("run")?.to_owned();
    assert_eq!(payload["base"], f.commit.as_str());
    assert_eq!(
        payload["checks"],
        json!([{"name": "ok", "result": "entailed"}, {"name": "bad", "result": "refuted"}])
    );
    assert_eq!(payload["resolved"][0]["placeholder"], "\"tuft:@v#1\"");
    assert_eq!(payload["resolved"][0]["line"], 2);
    assert!(payload.get("worktree").is_none());
    let text = payload["text"].as_str().ok_or("text")?;
    assert!(text.ends_with(&format!("next: writfold trial approve {run} --require ok")));
    // It never approves: the run holds exactly its Prepared record.
    let shown = server.call("trial/show", json!({"run": run}))?;
    let history = shown["result"]["payload"]["text"].as_str().ok_or("show")?;
    assert!(history.contains("1 record(s), history validated"));
    assert!(history.contains("  approval    none\n"), "{history}");
    // The CLI's `trial run` on the same input prints the same summary.
    let rest = [
        repo,
        "--recipe",
        recipe,
        "--tuft",
        tuft,
        "--weft",
        review,
        "--weft-bin",
        weft,
    ];
    let printed = stdout(&argv("trial run --file src/lib.rs --repo", &rest), &f.state)?;
    let printed = printed.strip_suffix('\n').ok_or("no final LF")?;
    let cli_run = run_id(printed)?;
    assert_ne!(cli_run, run);
    assert_eq!(normalized(printed, cli_run), normalized(text, &run));
    server.stop()?;
    assert_eq!(fs::read_to_string(f.repo.join("src/lib.rs"))?, SOURCE);
    Ok(())
}

#[test]
fn a_refused_serve_run_has_the_cli_message_and_an_unknown_outcome() -> Result {
    let f = fixture("run-refused")?;
    let mut server = Server::start(&f.state, json!(["writfold.trial/1"]))?;
    let (repo, absent) = (path(&f.repo)?, f.root.join("absent.spl"));
    let absent = path(&absent)?;
    let reply = server.call(
        "trial/run",
        json!({"repo": repo, "file": "src/lib.rs", "recipe": absent}),
    )?;
    let rest = [repo, "--recipe", absent];
    let printed = cli(&argv("trial run --file src/lib.rs --repo", &rest), &f.state)?;
    let (category, outcome, message) = same_refusal(&reply, printed)?;
    assert!(message.starts_with("step prepare refused: "), "{message}");
    // A step may have written before a later one refused: unknown, not rejected.
    assert_eq!(
        (category.as_str(), outcome.as_str()),
        ("not-found", "unknown")
    );
    assert_eq!(reply["error"]["data"]["exit_code"], Value::Null);
    server.stop()
}

#[test]
fn serve_evidence_list_and_show_match_the_cli() -> Result {
    let f = fixture("evidence")?;
    let binding = retain_entry(&f)?;
    let repo = path(&f.repo)?;
    let mut server = Server::start(&f.state, json!(["writfold.trial/1"]))?;
    let listed = server.call("trial/evidence-list", json!({"path": "src/"}))?;
    let payload = &listed["result"]["payload"];
    let rows: Vec<String> = serde_json::from_value(payload["rows"].clone())?;
    let trailer = payload["trailer"].as_str().ok_or("trailer")?;
    let view: String = rows.iter().map(|r| format!("{r}\n")).collect();
    let printed = stdout(&argv("trial evidence list --path src/", &[]), &f.state)?;
    assert_eq!(printed, format!("{view}{trailer}\n"));
    assert_eq!(trailer, "# count=1 shown=1 omitted=0");
    let params = json!({"binding": "entry", "context_lines": 1, "repo": repo});
    let shown = server.call("trial/evidence-show", params)?;
    let payload = &shown["result"]["payload"];
    assert_eq!(payload["binding"], binding.as_str());
    assert_eq!(
        (payload["start"].as_u64(), payload["end"].as_u64()),
        (Some(11), Some(18))
    );
    let text = payload["text"].as_str().ok_or("text")?;
    assert!(text.contains("--- retained (exact bytes, verified)\nrun(1);\n"));
    let fixed = "trial evidence show entry --context-lines 1 --repo";
    let printed = stdout(&argv(fixed, &[repo]), &f.state)?;
    assert_eq!(printed, format!("{text}\n"));
    server.stop()
}

#[test]
fn tampered_evidence_refuses_over_serve_as_on_the_cli() -> Result {
    let f = fixture("tampered")?;
    let binding = retain_entry(&f)?;
    fs::write(f.state.join(format!("evidence/{binding}.bytes")), "run(9);")?;
    let mut server = Server::start(&f.state, json!(["writfold.trial/1"]))?;
    for which in ["entry", binding.as_str()] {
        let reply = server.call("trial/evidence-show", json!({"binding": which}))?;
        let printed = cli(&argv("trial evidence show", &[which]), &f.state)?;
        let (category, outcome, message) = same_refusal(&reply, printed)?;
        assert!(message.contains("(tampered); refusing"), "{message}");
        assert_eq!(
            (category.as_str(), outcome.as_str()),
            ("malformed", "rejected")
        );
    }
    // A listing never shows it, and counts it as refused.
    let listed = server.call("trial/evidence-list", json!({}))?;
    assert_eq!(listed["result"]["payload"]["rows"], json!([]));
    assert_eq!(
        listed["result"]["payload"]["trailer"],
        "# count=0 shown=0 omitted=0 refused=1"
    );
    server.stop()
}

#[test]
fn a_session_without_the_capability_refuses_run_and_evidence() -> Result {
    let f = fixture("plain")?;
    let mut server = Server::start(&f.state, json!([]))?;
    for method in NEW {
        let refused = server.call(method, json!({"binding": "x"}))?;
        assert_eq!(refused["error"]["code"], -32000, "{method}");
        assert_eq!(
            refused["error"]["data"]["message"],
            "unsupported requested capability"
        );
    }
    server.stop()?;
    assert!(!f.state.join("weft").exists() && !f.state.join("evidence").exists());
    Ok(())
}
