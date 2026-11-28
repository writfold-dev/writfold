//! A route's positional body (#187): `file=@1`, `@2` or `@last` names the
//! command's first, second or last positional argument as the body file,
//! and `flags=` the flags that take a value (`gh gist create --desc X
//! notes.md` with `flags=--desc,-d`). An unlisted flag followed by a word
//! might take that word as its value, so the body is unread, and an unread
//! body under a route is denied, as for a flag.
use super::delivery_body::file_bytes;
use crate::deliver::guarded::Read;
use crate::deliver::route::Route;
use crate::shell::Word;
use std::path::Path;

/// The positional arguments after the route's own words, or why they
/// cannot be told.
fn positionals<'w>(route: &Route, words: &'w [Word]) -> Result<Vec<&'w Word>, String> {
    let n = route.words.len();
    let start = (0..words.len().saturating_sub(n.saturating_sub(1)))
        .find(|&i| {
            words
                .get(i..i + n)
                .is_some_and(|w| w.iter().zip(&route.words).all(|(a, b)| a.text == *b))
        })
        .map_or(n, |i| i + n);
    let mut out = Vec::new();
    let mut rest = words.iter().skip(start).peekable();
    let mut options = true;
    while let Some(word) = rest.next() {
        let text = word.text.as_str();
        if !options || !text.starts_with('-') || text == "-" {
            out.push(word);
        } else if text == "--" {
            options = false;
        } else if text.contains('=') {
            continue;
        } else if route.flags.iter().any(|f| f == text) {
            rest.next();
        } else if rest.peek().is_some_and(|w| !w.text.starts_with('-')) {
            return Err(format!(
                "{text} is not in the route's flags=, so the word after it may be its value; list it in flags= if it takes one"
            ));
        }
    }
    Ok(out)
}

/// The body under a positional route, or `None` when the route reads flags.
pub fn read(route: &Route, words: &[Word], cwd: &Path, elsewhere: bool) -> Option<Read> {
    let at = route.file.iter().find(|f| f.starts_with('@'))?;
    let found = match positionals(route, words) {
        Ok(found) => found,
        Err(why) => return Some(Read::Unread(why)),
    };
    let picked = match at.as_str() {
        "@last" => found.last().copied(),
        n => (n[1..].parse::<usize>().ok())
            .and_then(|k| k.checked_sub(1))
            .and_then(|k| found.get(k).copied()),
    };
    Some(match picked {
        Some(value) => file_bytes(value, cwd, elsewhere),
        None => Read::Unread(format!("the command has no positional argument {at}")),
    })
}
