//! How a guard reads a POSIX shell command line, without running or
//! expanding anything: which simple commands it holds, what each one is
//! once a shell's and a wrapper's leading words are set aside, and which
//! directory each acts in.
//!
//! 1. Segments: words split on blanks; quotes and escapes removed; `;`,
//!    `&`, `|`, newline, `(` and `)` end a simple command; `#` starts a
//!    comment. Text inside quotes is one word and never a command.
//! 2. Here-documents: after `<<` or `<<-` and its delimiter, the lines up
//!    to the delimiter line are data. `<<<` is a word.
//! 3. Leading words skipped: assignments; `if`, `then`, `else`, `elif`,
//!    `do`, `while`, `until`, `!`, `{` and `time`; `command`, `exec`,
//!    `nohup`, `builtin`, `env` and `sudo` when the next word does not
//!    begin with `-`.
//! 4. A command name holding `/` is compared by its last segment.
//! 5. `git`'s global options are dropped, with their values.
//! 6. `--help` as the only word after a matched prefix does not match
//!    ([`matches`]).
//! 7. The directory starts at the working directory; `cd DIR` (alone, not
//!    in a pipeline or the background) sets it for the rest of its group;
//!    `git -C DIR` and `--work-tree DIR` for that invocation only.
//! 8. `~`, `~/`, `$HOME` and `${HOME}` are the home directory.
//! 9. Anything else is never guessed: the directory is
//!    [`Dir::Unresolved`], with the reason.
//!
//! Not followed: `bash -c` and `eval` strings, programs that run their
//! arguments (`xargs`, `find -exec`), backquotes, aliases, functions and
//! scripts, and `ssh`'s remote command. For a guard these are misses.
//!
//! Whether a rule's prefix names a command ([`named`], behind [`matches`])
//! is translated and proved (`proofs/Writfold/Command.lean`); the reader
//! is a parser and is tested.
#[cfg(test)]
mod corpus;
mod invoke;
mod read;
#[cfg(test)]
mod tests;

use alloc::string::String;
use alloc::vec::Vec;
pub use invoke::{invocations, resolve};
pub use read::{Piece, Sep, pieces};

/// One word of a command, with its quotes removed.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Word {
    pub text: String,
    /// The word holds an expansion (`$…` or a backquote): its value is not
    /// its text.
    pub expands: bool,
    /// It begins with an unquoted `~`.
    pub tilde: bool,
    /// It holds an unquoted `*`, `?` or `[`.
    pub glob: bool,
}

/// Why a directory is not known.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Why {
    /// A word holding an expansion other than a leading `$HOME`.
    Variable,
    /// An unquoted `*`, `?` or `[`.
    Glob,
    /// `~name`.
    OtherUser,
    /// The home directory is needed and unknown.
    NoHome,
    /// After `cd -`, `pushd`, `popd`, or `cd` with an option.
    Previous,
    /// `--git-dir` without `--work-tree`, or `GIT_DIR`/`GIT_WORK_TREE`.
    GitDir,
    /// More distinct directories than a caller judges.
    TooMany,
}

impl Why {
    /// A short phrase naming the reason.
    pub fn text(self) -> &'static str {
        match self {
            Why::Variable => "a variable",
            Why::Glob => "a pattern",
            Why::OtherUser => "another user's home directory",
            Why::NoHome => "the home directory, which is not known here",
            Why::Previous => "an earlier directory (cd -, pushd, popd or a cd option)",
            Why::GitDir => "a Git directory set apart from its work tree",
            Why::TooMany => "more directories than one call judges",
        }
    }
}

/// The directory an invocation acts in.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Dir {
    /// An absolute path, `.` and `..` resolved by name.
    At(String),
    Unresolved(Why),
}

/// One simple command, as a rule compares it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Invocation {
    /// The command's name (its last path segment), then its arguments;
    /// for `git`, the subcommand follows with the global options dropped.
    pub words: Vec<String>,
    pub dir: Dir,
}

/// Whether `prefix` names `words`: a whole-word prefix, unless the only
/// word after it is `--help` ([`named`] over their bytes).
pub fn matches(prefix: &[String], words: &[String]) -> bool {
    let bytes =
        |all: &[String]| -> Vec<Vec<u8>> { all.iter().map(|w| w.as_bytes().to_vec()).collect() };
    named(&bytes(prefix), &bytes(words))
}

/// Whether a word is exactly `--help`.
pub fn is_help(word: &[u8]) -> bool {
    word.len() == 6
        && word[0] == b'-'
        && word[1] == b'-'
        && word[2] == b'h'
        && word[3] == b'e'
        && word[4] == b'l'
        && word[5] == b'p'
}

/// Whether `prefix` names `words`: each word of `prefix` is the word of
/// `words` at the same place, and `words` is not `prefix` followed by
/// `--help` alone. Translated and proved (`Command.lean`).
pub fn named(prefix: &[Vec<u8>], words: &[Vec<u8>]) -> bool {
    if prefix.len() > words.len() {
        return false;
    }
    let mut i = 0;
    while i < prefix.len() {
        if !crate::bytes::same(&prefix[i], &words[i]) {
            return false;
        }
        i += 1;
    }
    !(words.len() - prefix.len() == 1 && is_help(&words[prefix.len()]))
}

/// The simple commands of `line`, each as its words (rules 1 and 2).
pub fn segments(line: &str) -> Vec<Vec<Word>> {
    pieces(line)
        .into_iter()
        .filter_map(|piece| match piece {
            Piece::Words(words) => Some(words),
            Piece::Sep(_) => None,
        })
        .collect()
}

/// The words after leading `NAME=value` assignments.
pub fn command(words: &[Word]) -> &[Word] {
    let skip = words
        .iter()
        .take_while(|w| {
            w.text.split_once('=').is_some_and(|(name, _)| {
                !name.is_empty() && name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
            })
        })
        .count();
    &words[skip..]
}
