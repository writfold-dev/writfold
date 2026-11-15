use super::{Dir, Invocation, Why, command, invocations, matches, segments};
use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

const CWD: &str = "/w/a";
const HOME: Option<&str> = Some("/home/u");

fn all(line: &str) -> Vec<Invocation> {
    invocations(line, CWD, HOME)
}

fn words(text: &str) -> Vec<String> {
    text.split_whitespace().map(ToString::to_string).collect()
}

/// The directories of the invocations `rule` matches in `line`.
fn hits(line: &str, rule: &str) -> Vec<Dir> {
    let rule = words(rule);
    all(line)
        .into_iter()
        .filter(|i| matches(&rule, &i.words))
        .map(|i| i.dir)
        .collect()
}

fn refused(line: &str) -> bool {
    !hits(line, "git commit").is_empty() || !hits(line, "git push").is_empty()
}

fn at(path: &str) -> Dir {
    Dir::At(path.to_string())
}

#[test]
fn rule_1_segments_quotes_and_comments() {
    let texts: Vec<Vec<String>> =
        segments("cd x && GH=1 gh pr comment 3 --body 'a b' ; echo \"q \\\"z\\\"\" # c")
            .into_iter()
            .map(|s| s.into_iter().map(|w| w.text).collect())
            .collect();
    let mut comment = words("GH=1 gh pr comment 3 --body");
    comment.push("a b".to_string());
    let echo = Vec::from(["echo".to_string(), "q \"z\"".to_string()]);
    assert_eq!(texts, [words("cd x"), comment, echo]);
    let s = segments("gh pr comment --body \"$(cat f)\" x");
    assert!(s[0][4].expands && !s[0][5].expands);
    assert_eq!(command(&segments("A=1 B_2=x gh")[0]).len(), 1);
    // `2>&1` and `&>` are redirections, not a background separator.
    assert_eq!(segments("git commit 2>&1 &>/dev/null").len(), 1);
    assert_eq!(segments("a && b || c | d & e").len(), 5);
    let flags = &segments("ls ~/x '~' a*b \"*\"")[0];
    let seen: Vec<(bool, bool)> = flags.iter().map(|w| (w.tilde, w.glob)).collect();
    assert_eq!(
        seen,
        [
            (false, false),
            (true, false),
            (false, false),
            (false, true),
            (false, false)
        ]
    );
}

#[test]
fn the_specification_table() {
    let pass = [
        "cat README.md",
        "grep -r \"git commit\" .",
        "echo \"git commit\"",
        "ssh host \"git commit -m x\"",
        "ssh host 'cd r && git push'",
        "cat > notes.md <<EOF\ngit commit -m x\nEOF",
        "git commit --help",
        "bash -c \"git commit\"",
        "xargs git commit",
        "x=`git commit`",
        "python -c 'import os; os.system(\"git commit\")'",
        "git commit-tree HEAD^{tree}",
        "sudo -u x git commit",
        "command -v git commit",
    ];
    for line in pass {
        assert!(!refused(line), "refused: {line}");
    }
    let deny = [
        "git commit -m x",
        "FOO=1 git commit",
        "cd /tmp && git commit",
        "true || git push",
        "echo $(git commit)",
        "git -C ~/other commit",
        "git --no-pager commit",
        "git -c user.name=x commit",
        "/usr/bin/git commit",
        "command git commit",
        "time git commit",
        "sudo git commit",
        "env A=1 git commit",
        "if git commit; then echo y; fi",
        "{ git commit; }",
        "! git commit",
        "exec git push",
        "nohup git push &",
        "git commit -m --help",
        "git --git-dir=.git --work-tree=. commit",
    ];
    for line in deny {
        assert!(refused(line), "passed: {line}");
    }
    assert!(!hits("git push --dry-run", "git push").is_empty());
}

#[test]
fn rule_2_here_documents() {
    let line = "cat > f <<EOF\ngit commit -m x\nEOF\ngit push";
    assert_eq!(hits(line, "git push").len(), 1);
    assert!(hits(line, "git commit").is_empty());
    // Ending at the delimiter line, not at a line that only holds it.
    assert!(hits("cat <<EOF\nEOF x\ngit commit\n", "git commit").is_empty());
    // A quoted delimiter, `<<-` with tabs, two on one line, in order.
    assert!(refused("cat <<'END'\ngit commit\nEND\ngit commit"));
    assert!(refused("cat <<-E\n\tgit push\n\tE\ngit commit"));
    let two = "cat <<A - <<B\ngit push\nA\ngit push\nB\ngit commit";
    assert_eq!(hits(two, "git push").len(), 0);
    assert_eq!(hits(two, "git commit").len(), 1);
    // `<<<` is a word; an unterminated body consumes the rest.
    assert!(refused("cat <<< x; git commit"));
    assert!(!refused("cat <<EOF\ngit commit\ngit push"));
    // A commit message in a quoted here-document is one word.
    assert!(refused("git commit -m \"$(cat <<'EOF'\nfix\nEOF\n)\""));
}

#[test]
fn rule_3_skipped_words_stop_at_an_option() {
    for line in [
        "sudo -u x git commit",
        "env -i git commit",
        "nohup -p git push",
    ] {
        assert!(!refused(line), "{line}");
    }
    // A wrapper with an option is left as it is: a rule naming it matches.
    assert_eq!(hits("sudo -u x git commit", "sudo").len(), 1);
    assert!(hits("sudo git commit", "sudo").is_empty());
    for line in [
        "while git push; do :; done",
        "until git push; do :; done",
        "builtin git commit",
    ] {
        assert!(refused(line), "{line}");
    }
}

#[test]
fn rule_5_git_options_and_rule_7_directories() {
    assert_eq!(hits("git -C b commit", "git commit"), [at("/w/a/b")]);
    assert_eq!(
        hits("git -C b -C ../c commit", "git commit"),
        [at("/w/a/c")]
    );
    assert_eq!(hits("git --work-tree=/x commit", "git commit"), [at("/x")]);
    assert_eq!(hits("cd /w/b && git commit", "git commit"), [at("/w/b")]);
    assert_eq!(hits("cd ../b; git commit", "git commit"), [at("/w/b")]);
    assert_eq!(
        hits("(cd /w/b && true); git commit", "git commit"),
        [at("/w/a")]
    );
    assert_eq!(hits("(cd /w/b && git commit)", "git commit"), [at("/w/b")]);
    assert_eq!(
        hits("echo $(cd /x); git commit", "git commit"),
        [at("/w/a")]
    );
    // A `cd` in a pipeline or the background changes nothing.
    assert_eq!(hits("cd /x | true; git commit", "git commit"), [at("/w/a")]);
    assert_eq!(hits("cd /x & git commit", "git commit"), [at("/w/a")]);
    assert_eq!(hits("cd && git commit", "git commit"), [at("/home/u")]);
    assert_eq!(
        invocations("cd && git commit", CWD, None)[1].dir,
        Dir::Unresolved(Why::NoHome)
    );
}

#[test]
fn rule_8_home_and_rule_9_never_guessed() {
    assert_eq!(hits("git -C ~/b commit", "git commit"), [at("/home/u/b")]);
    assert_eq!(
        hits("git -C \"$HOME/b\" commit", "git commit"),
        [at("/home/u/b")]
    );
    assert_eq!(hits("git -C ${HOME} commit", "git commit"), [at("/home/u")]);
    assert_eq!(hits("git -C '~/b' commit", "git commit"), [at("/w/a/~/b")]);
    assert_eq!(
        hits("git -C '$HOME' commit", "git commit"),
        [at("/w/a/$HOME")]
    );
    let unresolved = [
        ("git -C \"$R\" commit", Why::Variable),
        ("git -C $HOME$X commit", Why::Variable),
        ("git -C $HOMER commit", Why::Variable),
        ("git -C b* commit", Why::Glob),
        ("git -C ~bob/b commit", Why::OtherUser),
        ("cd - && git commit", Why::Previous),
        ("cd -P /x && git commit", Why::Previous),
        ("pushd /x && git commit", Why::Previous),
        ("GIT_DIR=x git commit", Why::GitDir),
        ("git --git-dir=x commit", Why::GitDir),
        ("cd $R && cd /abs && git commit", Why::Variable),
        ("cd $R; cd; git commit", Why::Variable),
    ];
    for (line, why) in unresolved {
        assert_eq!(hits(line, "git commit"), [Dir::Unresolved(why)], "{line}");
    }
}

/// A fixed pseudo-random corpus: no new crate, the same strings each run.
fn corpus() -> Vec<String> {
    let mut seed: u64 = 0x5eed_2026_1115;
    let mut out: Vec<String> = [
        "git commit",
        "x; git commit",
        "$(git push)",
        "a\ngit commit\nb",
        "`git commit`",
        "' ; git commit ; '",
        "\" && git push",
        "\\\ngit commit",
        "EOF",
        "<<EOF\ngit push",
    ]
    .into_iter()
    .map(String::from)
    .collect();
    for _ in 0..400 {
        let mut s = String::new();
        let len = (seed % 40) as usize;
        for _ in 0..len {
            seed = seed
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            let pick = (seed >> 33) % 100;
            let c = match pick {
                0..=4 => '\n',
                5..=9 => ' ',
                _ => char::from(32 + ((seed >> 40) % 95) as u8),
            };
            s.push(c);
        }
        if seed.is_multiple_of(3) {
            s.push_str("\ngit commit -m x\n");
        }
        out.push(s);
    }
    out
}

#[test]
fn quoted_text_and_here_document_bodies_are_never_commands() {
    for s in corpus() {
        let printable: String = s.chars().filter(|c| *c != '\n').collect();
        let single = format!("echo '{}'", printable.replace('\'', ""));
        let mut escaped = String::new();
        for c in printable.chars() {
            if matches!(c, '"' | '\\' | '$' | '`') {
                escaped.push('\\');
            }
            escaped.push(c);
        }
        let double = format!("cat \"{escaped}\"");
        let body = format!("cat <<'WF_END_7'\n{s}\nWF_END_7");
        let dashed = format!("cat <<-WF_END_7 >f\n{s}\n\tWF_END_7");
        for line in [single, double, body, dashed] {
            let named: Vec<Invocation> = all(&line)
                .into_iter()
                .filter(|i| i.words.first().is_some_and(|w| w == "git"))
                .collect();
            assert!(named.is_empty(), "{line:?}: {named:?}");
        }
    }
}
