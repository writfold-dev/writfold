//! A small reader of POSIX shell command lines, enough for the guard hook
//! to find a publishing command and its body argument: words split on
//! blanks, single and double quotes, backslash escapes, comments, and the
//! command separators `;`, `&`, `|` and newlines. It never runs or expands
//! anything; a word holding `$` or a backquote outside single quotes is
//! marked, because its value is only known to the shell.

/// One word of a command, with its quotes removed.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Word {
    pub text: String,
    /// The word holds an expansion (`$…` or a backquote): its value is not
    /// its text.
    pub expands: bool,
}

#[derive(Default)]
struct Reader {
    segments: Vec<Vec<Word>>,
    words: Vec<Word>,
    word: Option<Word>,
}

impl Reader {
    fn push(&mut self, c: char) {
        self.word.get_or_insert_with(Word::default).text.push(c);
    }

    fn expands(&mut self) {
        self.word.get_or_insert_with(Word::default).expands = true;
    }

    fn end_word(&mut self) {
        if let Some(word) = self.word.take() {
            self.words.push(word);
        }
    }

    fn end_segment(&mut self) {
        self.end_word();
        if !self.words.is_empty() {
            self.segments.push(std::mem::take(&mut self.words));
        }
    }
}

/// The simple commands of `line`, each as its words.
pub fn segments(line: &str) -> Vec<Vec<Word>> {
    let mut r = Reader::default();
    let mut chars = line.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            ' ' | '\t' => r.end_word(),
            '\n' | ';' | '&' | '|' | '(' | ')' => r.end_segment(),
            '#' if r.word.is_none() => {
                while chars.peek().is_some_and(|c| *c != '\n') {
                    chars.next();
                }
            }
            '\\' => match chars.next() {
                Some('\n') | None => {}
                Some(next) => r.push(next),
            },
            '\'' => {
                r.word.get_or_insert_with(Word::default);
                for next in chars.by_ref() {
                    if next == '\'' {
                        break;
                    }
                    r.push(next);
                }
            }
            '"' => {
                r.word.get_or_insert_with(Word::default);
                while let Some(next) = chars.next() {
                    match next {
                        '"' => break,
                        '\\' => match chars.peek().copied() {
                            Some(e @ ('"' | '\\' | '$' | '`')) => {
                                chars.next();
                                r.push(e);
                            }
                            Some('\n') => {
                                chars.next();
                            }
                            _ => r.push('\\'),
                        },
                        '$' | '`' => {
                            r.expands();
                            r.push(next);
                        }
                        other => r.push(other),
                    }
                }
            }
            '$' | '`' => {
                r.expands();
                r.push(c);
            }
            other => r.push(other),
        }
    }
    r.end_segment();
    r.segments
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

#[cfg(test)]
mod tests {
    use super::{command, segments};

    fn texts(line: &str) -> Vec<Vec<String>> {
        segments(line)
            .into_iter()
            .map(|s| s.into_iter().map(|w| w.text).collect())
            .collect()
    }

    #[test]
    fn quotes_separators_and_comments() {
        assert_eq!(
            texts("cd x && GH=1 gh pr comment 3 --body 'a b' ; echo \"q \\\"z\\\"\" # c"),
            vec![
                vec!["cd", "x"],
                vec!["GH=1", "gh", "pr", "comment", "3", "--body", "a b"],
                vec!["echo", "q \"z\""],
            ]
        );
        assert_eq!(texts("a\\ b ''"), vec![vec!["a b", ""]]);
        let s = segments("gh pr comment --body \"$(cat f)\" x");
        assert!(s[0][4].expands && !s[0][5].expands);
        assert_eq!(command(&s[0])[0].text, "gh");
        let assigned = segments("A=1 B_2=x gh");
        assert_eq!(command(&assigned[0]).len(), 1);
    }
}
