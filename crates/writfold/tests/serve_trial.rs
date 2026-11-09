//! `writfold serve --state` driven over real stdio pipes on a throwaway
//! repository: initialize with `writfold.trial/1`, retain, prepare citing the
//! retained binding, show, shutdown. Then the same bad report through the CLI
//! and through `serve` gives the same refusal.
use serde_json::{Value, json};
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::fs::DirBuilderExt;
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, ChildStdout, Command, Output, Stdio};

type Result<T = (), E = Box<dyn std::error::Error>> = std::result::Result<T, E>;
const WRITFOLD: &str = env!("CARGO_BIN_EXE_writfold");
const SOURCE: &str = "fn main() {\n    run(1);\n}\n";
const RECIPE: &str =
    "edit \"src/main.rs\" {\n  @n := one text \"run(1)\"\n  replace @n with \"run(2)\"\n}\n";

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

fn git(dir: &Path, args: &[&str]) -> Result<String> {
    let out = Command::new("git")
        .current_dir(dir)
        .env("GIT_AUTHOR_NAME", "t")
        .env("GIT_AUTHOR_EMAIL", "t@example.invalid")
        .env("GIT_COMMITTER_NAME", "t")
        .env("GIT_COMMITTER_EMAIL", "t@example.invalid")
        .args(args)
        .output()?;
    if !out.status.success() {
        return Err(format!("git {args:?} failed").into());
    }
    Ok(String::from_utf8(out.stdout)?.trim_end().to_owned())
}

fn cli(args: &[&str], state: &Path) -> Result<Output> {
    Ok(Command::new(WRITFOLD)
        .args(args)
        .arg("--state")
        .arg(state)
        .output()?)
}

fn fixture(name: &str) -> Result<Fixture> {
    let root = std::env::temp_dir().join(format!(".writfold-serve-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::DirBuilder::new().mode(0o700).create(&root)?;
    let repo = root.join("repo");
    fs::create_dir_all(repo.join("src"))?;
    fs::write(repo.join("src/main.rs"), SOURCE)?;
    git(&repo, &["init", "-q"])?;
    git(&repo, &["add", "-A"])?;
    git(&repo, &["commit", "-qm", "fixture"])?;
    let commit = git(&repo, &["rev-parse", "HEAD"])?;
    let state = root.join("state");
    fs::write(root.join("r.spl"), RECIPE)?;
    let fixture = Fixture {
        root,
        repo,
        state,
        commit,
    };
    if !cli(&["trial", "init"], &fixture.state)?.status.success() {
        return Err("trial init failed".into());
    }
    Ok(fixture)
}

fn report(asset: &str) -> String {
    format!(
        "{{\"schema\":\"weft.report/1\",\"kind\":\"assessment\",\"scope\":\"local-symbolic\",\
\"execution\":\"complete\",\"writfold_grounded\":false,\"effect_authority\":false,\
\"review\":{{\"bindings\":[{{\"kind\":\"asset\",\"reference\":\"{asset}\"}}]}},\
\"checks\":[{{\"name\":\"c\",\"result\":\"entailed\"}}]}}"
    )
}

struct Server {
    child: Child,
    input: ChildStdin,
    output: BufReader<ChildStdout>,
    next: u64,
}

impl Server {
    fn start(state: &Path) -> Result<Self> {
        let mut child = Command::new(WRITFOLD)
            .args(["serve", "--state"])
            .arg(state)
            .env_remove("WRITFOLD_STATE")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()?;
        let input = child.stdin.take().ok_or("stdin")?;
        let output = BufReader::new(child.stdout.take().ok_or("stdout")?);
        Ok(Self {
            child,
            input,
            output,
            next: 0,
        })
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
        if reply["id"] != json!(self.next)
            || reply["result"]["generation"].is_number()
                && reply["result"]["generation"] != json!(self.next)
        {
            return Err(format!("reply lost its correlation: {reply}").into());
        }
        Ok(reply)
    }
}

fn path(p: &Path) -> Result<&str> {
    p.to_str().ok_or_else(|| "non-UTF-8 temp path".into())
}

#[test]
fn serve_runs_retain_prepare_and_show_through_the_trial_functions() -> Result {
    let f = fixture("flow")?;
    let mut server = Server::start(&f.state)?;
    let init = server.call(
        "initialize",
        json!({"protocol_version": "2", "capabilities": ["writfold.trial/1"]}),
    )?;
    assert_eq!(
        init["result"]["capabilities"],
        json!(["notifications", "writfold.trial/1"])
    );
    let retained = server.call(
        "trial/retain",
        json!({"repo": path(&f.repo)?, "commit": f.commit, "file": "src/main.rs", "start": 16, "end": 23}),
    )?;
    let binding = retained["result"]["payload"]["binding"]
        .as_str()
        .ok_or("binding")?
        .to_owned();
    assert_eq!(retained["result"]["payload"]["start"], 16);
    let report_path = f.root.join("report.json");
    fs::write(&report_path, report(&binding))?;
    let prepared = server.call(
        "trial/prepare",
        json!({"repo": path(&f.repo)?, "file": "src/main.rs", "recipe": path(&f.root.join("r.spl"))?, "report": path(&report_path)?}),
    )?;
    let payload = &prepared["result"]["payload"];
    let run = payload["run"].as_str().ok_or("run")?.to_owned();
    assert!(
        payload["prepared"]
            .as_str()
            .unwrap_or_default()
            .starts_with("wfwt1:prepared-edit:sha256:")
    );
    assert_eq!(
        payload["splices"],
        json!([{"start": 16, "end": 22, "inserted": 6}])
    );
    assert!(payload.get("worktree").is_none());
    // approve/apply/recover do not exist over serve, even for this run.
    for method in ["trial/approve", "trial/apply", "trial/recover"] {
        let refused = server.call(method, json!({"run": run}))?;
        assert_eq!(refused["error"]["code"], -32601, "{method}");
    }
    let shown = server.call("trial/show", json!({"run": run}))?;
    let text = shown["result"]["payload"]["text"].as_str().ok_or("text")?;
    assert!(text.contains("1 record(s), history validated"));
    assert!(text.contains("1 retained excerpt(s) cited by binding digest"));
    // The same function as the CLI: byte-identical rendering.
    let printed = cli(&["trial", "show", &run], &f.state)?;
    assert_eq!(String::from_utf8(printed.stdout)?, format!("{text}\n"));
    let stop = server.call("shutdown", json!({}))?;
    assert_eq!(stop["result"]["payload"], json!({"shutdown": true}));
    assert!(server.child.wait()?.success());
    // The user's checkout is untouched.
    assert_eq!(fs::read_to_string(f.repo.join("src/main.rs"))?, SOURCE);
    Ok(())
}

#[test]
fn cli_and_serve_give_the_same_refusal_for_a_bad_report() -> Result {
    let f = fixture("refusal")?;
    let unretained = "0".repeat(64);
    let cases = [
        ("unretained.json", report(&unretained)),
        ("garbage.json", "{\"schema\":\"not-a-report\"}".to_owned()),
    ];
    let mut server = Server::start(&f.state)?;
    server.call(
        "initialize",
        json!({"protocol_version": "2", "capabilities": ["writfold.trial/1"]}),
    )?;
    for (name, bytes) in cases {
        let report_path = f.root.join(name);
        fs::write(&report_path, bytes)?;
        let recipe = f.root.join("r.spl");
        let repo = path(&f.repo)?;
        let printed = cli(
            &[
                "trial",
                "prepare",
                "--repo",
                repo,
                "--file",
                "src/main.rs",
                "--recipe",
                path(&recipe)?,
                "--report",
                path(&report_path)?,
            ],
            &f.state,
        )?;
        assert!(!printed.status.success());
        let stderr = String::from_utf8(printed.stderr)?;
        let refused = server.call(
            "trial/prepare",
            json!({"repo": repo, "file": "src/main.rs", "recipe": path(&recipe)?, "report": path(&report_path)?}),
        )?;
        let data = &refused["error"]["data"];
        assert_eq!(refused["error"]["code"], -32000);
        assert_eq!(
            format!("writfold: {}\n", data["message"].as_str().ok_or("message")?),
            stderr
        );
        assert!(stderr.starts_with("writfold: report refused: "), "{stderr}");
        assert_eq!(data["category"], "malformed");
        // Decided before any write: rejected, not unknown.
        assert_eq!(data["outcome"], "rejected");
        assert_eq!(data["exit_code"], 1);
    }
    // Neither path left a worktree behind.
    assert!(
        !f.state.join("worktrees").exists()
            || fs::read_dir(f.state.join("worktrees"))?.next().is_none()
    );
    server.call("shutdown", json!({}))?;
    assert!(server.child.wait()?.success());
    Ok(())
}
