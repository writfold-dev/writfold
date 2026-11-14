mod checker;

use checker::{RUN, Result, Root, WRITFOLD, world};
use std::process::{Command, Output, Stdio};

const THEMES: [&str; 6] = ["hone", "slate", "carbide", "vellum", "paper", "paper-dark"];

fn invoke(w: &Root, args: &[&str], env: &[(&str, &str)], xdg: bool) -> Result<Output> {
    let mut command = Command::new(WRITFOLD);
    command
        .env_clear()
        .current_dir(w.kb())
        .args(args)
        .env("HOME", w.0.join("home"))
        .env("TERM", "xterm-256color")
        .env("COLORTERM", "truecolor")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Some(path) = std::env::var_os("PATH") {
        command.env("PATH", path);
    }
    if xdg {
        command.env("XDG_CONFIG_HOME", w.0.join("config"));
    }
    for (name, value) in env {
        command.env(name, value);
    }
    Ok(command.output()?)
}

fn config(w: &Root, theme: &str, xdg: bool) -> Result {
    let base = if xdg {
        w.0.join("config")
    } else {
        w.0.join("home/.config")
    };
    let dir = base.join("writfold");
    std::fs::create_dir_all(&dir)?;
    std::fs::write(dir.join("config.toml"), format!("theme = \"{theme}\"\n"))?;
    Ok(())
}

fn frame(w: &Root, extra: &[&str], env: &[(&str, &str)], xdg: bool) -> Result<Output> {
    let mut args = vec!["watch", "--run", RUN];
    args.extend_from_slice(extra);
    invoke(w, &args, env, xdg)
}

fn success(output: &Output) {
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(!output.stdout.is_empty());
    assert!(!output.stdout.contains(&0x1b));
}

fn warning(output: &Output, name: &str, source: &str) -> Result {
    success(output);
    let text = std::str::from_utf8(&output.stderr)?;
    for part in [
        name,
        source,
        "using hone",
        "hone, slate, carbide, vellum, paper, paper-dark",
    ] {
        assert!(text.contains(part), "{text}");
    }
    assert_eq!(text.lines().count(), 1, "{text}");
    Ok(())
}

fn prime(w: &Root) -> Result<Vec<u8>> {
    let output = invoke(w, &["kb", "index", "status"], &[], true)?;
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    success(&frame(w, &["--once"], &[], true)?);
    let output = frame(w, &["--once"], &[], true)?;
    success(&output);
    assert!(output.stderr.is_empty());
    Ok(output.stdout)
}

#[test]
fn once_and_pipe_are_identical_for_every_theme_and_depth() -> Result {
    let w = world("watch-look-plain", &[], false)?;
    let baseline = prime(&w)?;
    for once in [false, true] {
        for theme in THEMES {
            for depth in ["auto", "truecolor", "ansi", "none"] {
                let mut args = vec!["--theme", theme, "--color", depth];
                if once {
                    args.push("--once");
                }
                let output = frame(&w, &args, &[("NO_COLOR", "")], true)?;
                success(&output);
                assert_eq!(output.stdout, baseline, "{theme} {depth} once={once}");
                assert!(output.stderr.is_empty());
            }
        }
    }
    Ok(())
}

#[test]
fn real_config_environment_and_flag_precedence_preserves_plain_stdout() -> Result {
    let w = world("watch-look-precedence", &[], false)?;
    let baseline = prime(&w)?;
    config(&w, "bad-config", true)?;
    config(&w, "bad-home", false)?;
    let output = frame(&w, &["--once"], &[], true)?;
    warning(&output, "bad-config", "config.toml")?;
    assert_eq!(output.stdout, baseline);
    let output = frame(&w, &[], &[], false)?;
    warning(&output, "bad-home", "config.toml")?;
    assert_eq!(output.stdout, baseline);
    for extra in [&[][..], &["--once"][..]] {
        let output = frame(&w, extra, &[("WRITFOLD_THEME", "paper")], true)?;
        success(&output);
        assert!(output.stderr.is_empty());
        assert_eq!(output.stdout, baseline);
        let output = frame(&w, extra, &[("WRITFOLD_THEME", "bad-env")], true)?;
        warning(&output, "bad-env", "WRITFOLD_THEME")?;
        assert_eq!(output.stdout, baseline);
        let mut flags = vec!["--theme", "slate"];
        flags.extend_from_slice(extra);
        let output = frame(&w, &flags, &[("WRITFOLD_THEME", "bad-env")], true)?;
        success(&output);
        assert!(output.stderr.is_empty());
        assert_eq!(output.stdout, baseline);
    }
    config(&w, "vellum", true)?;
    let output = frame(&w, &["--once"], &[], true)?;
    success(&output);
    assert!(output.stderr.is_empty());
    assert_eq!(output.stdout, baseline);
    for extra in [&[][..], &["--once"][..]] {
        let mut flags = vec!["--theme", "bad-flag"];
        flags.extend_from_slice(extra);
        let output = frame(&w, &flags, &[("WRITFOLD_THEME", "paper")], true)?;
        warning(&output, "bad-flag", "--theme")?;
        assert_eq!(output.stdout, baseline);
    }
    Ok(())
}

#[test]
fn invalid_colour_is_a_usage_error_not_a_theme_warning() -> Result {
    let w = world("watch-look-invalid", &[], false)?;
    for once in [false, true] {
        let mut args = vec!["--color", "256"];
        if once {
            args.push("--once");
        }
        let output = frame(&w, &args, &[], true)?;
        assert!(!output.status.success());
        assert!(output.stdout.is_empty());
        let error = std::str::from_utf8(&output.stderr)?;
        for name in ["--color", "auto", "truecolor", "ansi", "none"] {
            assert!(error.contains(name), "{error}");
        }
    }
    Ok(())
}
