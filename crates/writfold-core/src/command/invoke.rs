//! From simple commands to invocations: the leading words a shell or a
//! wrapper adds are skipped, the command's name is compared by its last
//! path segment, `git`'s global options are dropped, and the directory
//! each command acts in is followed through `cd`, groups and `git -C`.
use super::read::{Piece, Sep, pieces};
use super::{Dir, Invocation, Why, Word};
use alloc::string::String;
use alloc::vec::Vec;

/// Shell words that may begin a simple command and are skipped.
const KEYWORDS: [&str; 10] = [
    "if", "then", "else", "elif", "do", "while", "until", "!", "{", "time",
];
/// Wrappers skipped when the next word does not begin with `-`.
const WRAPPERS: [&str; 6] = ["command", "exec", "nohup", "builtin", "env", "sudo"];
/// `git` global options that take a value.
const VALUED: [&str; 7] = [
    "-C",
    "-c",
    "--git-dir",
    "--work-tree",
    "--namespace",
    "--super-prefix",
    "--attr-source",
];

/// The variable name of a `NAME=value` word, if it is one.
fn assigned(text: &str) -> Option<&str> {
    let (name, _) = text.split_once('=')?;
    let ok = !name.is_empty() && name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_');
    ok.then_some(name)
}

/// `.` and `..` resolved by name; the result is absolute.
fn normal(path: &str) -> String {
    let mut parts: Vec<&str> = Vec::new();
    for part in path.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                parts.pop();
            }
            other => parts.push(other),
        }
    }
    let mut out = String::new();
    for part in &parts {
        out.push('/');
        out.push_str(part);
    }
    if out.is_empty() {
        out.push('/');
    }
    out
}

fn join(base: &str, path: &str) -> String {
    match path.starts_with('/') {
        true => normal(path),
        false => {
            let mut whole = String::from(base);
            whole.push('/');
            whole.push_str(path);
            normal(&whole)
        }
    }
}

/// What follows `$HOME` or `${HOME}` when that is the word's only
/// expansion and it is followed by nothing or `/`.
fn after_home(text: &str) -> Option<&str> {
    let rest = text
        .strip_prefix("${HOME}")
        .or_else(|| text.strip_prefix("$HOME"))?;
    let clean = (rest.is_empty() || rest.starts_with('/')) && !rest.contains(['$', '`']);
    clean.then_some(rest)
}

fn home_join(base: &str, home: Option<&str>, rest: &str) -> Dir {
    match home {
        Some(home) => {
            let mut path = String::from(home);
            path.push_str(rest);
            Dir::At(join(base, &path))
        }
        None => Dir::Unresolved(Why::NoHome),
    }
}

/// The directory `word` names, from `base`; never guessed.
pub fn resolve(base: &Dir, word: &Word, home: Option<&str>) -> Dir {
    let Dir::At(base) = base else {
        return base.clone();
    };
    let text = word.text.as_str();
    if word.expands {
        return match after_home(text) {
            Some(_) if word.glob => Dir::Unresolved(Why::Glob),
            Some(rest) => home_join(base, home, rest),
            None => Dir::Unresolved(Why::Variable),
        };
    }
    if word.glob {
        return Dir::Unresolved(Why::Glob);
    }
    if word.tilde {
        return match text.strip_prefix('~') {
            Some(rest) if rest.is_empty() || rest.starts_with('/') => home_join(base, home, rest),
            _ => Dir::Unresolved(Why::OtherUser),
        };
    }
    Dir::At(join(base, text))
}

/// The directory after a segment that changes it (`cd`, `pushd`,
/// `popd`), or `None` when the segment does not.
fn changed(words: &[Word], dir: &Dir, home: Option<&str>) -> Option<Dir> {
    let start = words
        .iter()
        .take_while(|w| KEYWORDS.contains(&w.text.as_str()) && w.text != "time")
        .count();
    let words = &words[start..];
    let first = words.first()?;
    let new = match (first.text.as_str(), words.get(1), words.len()) {
        ("pushd" | "popd", _, _) => Dir::Unresolved(Why::Previous),
        ("cd", None, _) => match home {
            Some(home) => Dir::At(normal(home)),
            None => Dir::Unresolved(Why::NoHome),
        },
        ("cd", Some(target), 2) if !target.text.starts_with('-') => resolve(dir, target, home),
        ("cd", _, _) => Dir::Unresolved(Why::Previous),
        _ => return None,
    };
    Some(match dir {
        Dir::Unresolved(_) => dir.clone(),
        Dir::At(_) => new,
    })
}

/// The index of the command's name after the skipped leading words, and
/// whether a `GIT_DIR` or `GIT_WORK_TREE` assignment was among them.
fn skipped(words: &[Word]) -> (usize, bool) {
    let mut i = 0;
    let mut git_dir = false;
    while let Some(word) = words.get(i) {
        let text = word.text.as_str();
        if let Some(name) = assigned(text) {
            git_dir |= name == "GIT_DIR" || name == "GIT_WORK_TREE";
        } else if !KEYWORDS.contains(&text) {
            let next = words.get(i + 1);
            let wrapped =
                WRAPPERS.contains(&text) && next.is_some_and(|n| !n.text.starts_with('-'));
            if !wrapped {
                break;
            }
        }
        i += 1;
    }
    (i, git_dir)
}

/// A command name compared by its last path segment.
fn name(text: &str) -> &str {
    match text.rsplit('/').next() {
        Some(last) if !last.is_empty() => last,
        _ => text,
    }
}

/// `git`'s global options from `words[1..]`: the index of the subcommand,
/// and the directory after `-C` and `--work-tree`.
fn git_options(words: &[Word], mut dir: Dir, home: Option<&str>) -> (usize, Dir) {
    let mut k = 1;
    let (mut git_dir, mut work_tree) = (false, false);
    while let Some(word) = words.get(k) {
        let text = word.text.as_str();
        if !text.starts_with('-') {
            break;
        }
        let (option, value, step) = match text.split_once('=') {
            Some((option, value)) if option.starts_with("--") => {
                let value = Word {
                    text: String::from(value),
                    ..word.clone()
                };
                (option, Some(value), 1)
            }
            _ if VALUED.contains(&text) => (text, words.get(k + 1).cloned(), 2),
            _ => (text, None, 1),
        };
        match (option, value) {
            ("-C" | "--work-tree", Some(value)) => {
                work_tree |= option == "--work-tree";
                dir = resolve(&dir, &value, home);
            }
            ("--git-dir", _) => git_dir = true,
            _ => {}
        }
        k += step;
    }
    if git_dir && !work_tree {
        dir = Dir::Unresolved(Why::GitDir);
    }
    (k.min(words.len()), dir)
}

/// One simple command as an invocation at `dir`.
fn invocation(words: &[Word], dir: &Dir, home: Option<&str>) -> Invocation {
    let (start, git_dir) = skipped(words);
    let words = &words[start..];
    let mut dir = match git_dir {
        true => Dir::Unresolved(Why::GitDir),
        false => dir.clone(),
    };
    let mut out: Vec<String> = Vec::new();
    let Some(first) = words.first() else {
        return Invocation { words: out, dir };
    };
    out.push(String::from(name(&first.text)));
    let mut rest = 1;
    if out[0] == "git" {
        let (k, at) = git_options(words, dir, home);
        rest = k;
        dir = at;
    }
    out.extend(words[rest..].iter().map(|w| w.text.clone()));
    Invocation { words: out, dir }
}

/// Every simple command of `line` as an invocation with its directory,
/// starting at `cwd` (absolute); `home` is `HOME`, if known.
pub fn invocations(line: &str, cwd: &str, home: Option<&str>) -> Vec<Invocation> {
    let all = pieces(line);
    let mut out = Vec::new();
    let mut dir = Dir::At(normal(cwd));
    let mut outside: Vec<Dir> = Vec::new();
    let mut before = Sep::Newline;
    for (i, piece) in all.iter().enumerate() {
        match piece {
            Piece::Sep(sep) => {
                match sep {
                    Sep::Open => outside.push(dir.clone()),
                    Sep::Close => {
                        if let Some(d) = outside.pop() {
                            dir = d;
                        }
                    }
                    _ => {}
                }
                before = *sep;
            }
            Piece::Words(words) => {
                let after = match all.get(i + 1) {
                    Some(Piece::Sep(sep)) => Some(*sep),
                    _ => None,
                };
                let alone =
                    before != Sep::Pipe && !matches!(after, Some(Sep::Pipe | Sep::Background));
                let next = changed(words, &dir, home);
                out.push(invocation(words, &dir, home));
                if let (Some(next), true) = (next, alone) {
                    dir = next;
                }
            }
        }
    }
    out
}
