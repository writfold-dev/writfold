//! `cursor`: Tuft's resume cursor, a serialized continuation of one paged
//! view.
//!
//! Canonical text (version 2): `v2.<P>.<C>.<OFFSET>.<K>`, where `P` and `C`
//! are the first 8 lowercase hex digits of the program digest and the
//! commit, `OFFSET` is the first row to show (decimal, no leading zeros) and
//! `K` is the first 8 hex digits of SHA-256 over
//! `tuft-cursor/2\n<program>\n<commit>\n<offset>\n<view digest>` with the
//! full program digest and commit. The view digest covers every compact row,
//! so a cursor binds its program, commit, offset and result.
//!
//! The check is not a secret: a cursor is a continuation, not a capability.
//! It turns a hand-edited, foreign or stale cursor into a typed refusal
//! instead of a silently wrong page.
//!
//! The revision-9 `NoticeCursor` is a different value: a borrowed,
//! in-process position over a `NoticeQuery` with no serialized form. This
//! cursor crosses processes as text, so it carries its binding itself.
use crate::digest_hex;
use std::fmt;

const VERSION: &str = "v2";
const DOMAIN: &str = "tuft-cursor/2";
const PREFIX: usize = 8;

/// Why a cursor does not resume this run.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CursorError {
    /// Not the canonical `v2.P.C.OFFSET.K` text.
    Malformed,
    /// Another cursor version, including the earlier unversioned
    /// `TAG.OFFSET` form.
    UnsupportedVersion,
    /// Issued at another commit: the cursor is stale.
    OtherCommit,
    /// Issued for a different program (or budget, which is in the program).
    ForeignProgram,
    /// The check does not match: the offset was edited or the result changed.
    Tampered,
    /// The offset is at or beyond the last row.
    OutOfRange,
}

impl fmt::Display for CursorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Malformed => "cursor is malformed (expected v2.PROGRAM.COMMIT.OFFSET.CHECK)",
            Self::UnsupportedVersion => "cursor version is not supported (expected v2)",
            Self::OtherCommit => "cursor was issued at a different commit",
            Self::ForeignProgram => "cursor was issued for a different program",
            Self::Tampered => "cursor check failed: its offset was edited or the result changed",
            Self::OutOfRange => "cursor offset is beyond the view",
        })
    }
}

impl std::error::Error for CursorError {}

/// What a cursor binds: the full program digest, the full commit and the
/// digest of the view's compact rows.
#[derive(Clone, Copy, Debug)]
pub struct Binding<'a> {
    pub program: &'a str,
    pub commit: &'a str,
    pub view: &'a str,
}

impl Binding<'_> {
    fn check(&self, offset: usize) -> String {
        let material = format!(
            "{DOMAIN}\n{}\n{}\n{offset}\n{}",
            self.program, self.commit, self.view
        );
        short(&digest_hex(material.as_bytes())).to_owned()
    }
}

/// A parsed or issued resume cursor. Its only constructors are [`Cursor::issue`]
/// and [`Cursor::parse`]; `Display` is the canonical text.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Cursor {
    program: String,
    commit: String,
    offset: usize,
    check: String,
}

fn short(hex: &str) -> &str {
    hex.get(..PREFIX).unwrap_or(hex)
}

fn is_hex(part: &str, len: usize) -> bool {
    part.len() == len && part.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
}

impl Cursor {
    /// The cursor that resumes `binding`'s view at row `offset`.
    pub fn issue(binding: &Binding<'_>, offset: usize) -> Self {
        Self {
            program: short(binding.program).to_owned(),
            commit: short(binding.commit).to_owned(),
            offset,
            check: binding.check(offset),
        }
    }

    /// Parse canonical text; anything that does not print back byte for
    /// byte is malformed.
    pub fn parse(text: &str) -> Result<Self, CursorError> {
        let parts: Vec<&str> = text.split('.').collect();
        let [version, program, commit, offset, check] = parts.as_slice() else {
            return Err(match parts.as_slice() {
                [tag, offset] if is_hex(tag, 16) && offset.parse::<usize>().is_ok() => {
                    CursorError::UnsupportedVersion
                }
                _ => CursorError::Malformed,
            });
        };
        if *version != VERSION {
            let numbered = version
                .strip_prefix('v')
                .is_some_and(|n| n.parse::<u32>().is_ok());
            return Err(if numbered {
                CursorError::UnsupportedVersion
            } else {
                CursorError::Malformed
            });
        }
        if ![program, commit, check].iter().all(|p| is_hex(p, PREFIX)) {
            return Err(CursorError::Malformed);
        }
        let cursor = Self {
            program: (*program).to_owned(),
            commit: (*commit).to_owned(),
            offset: offset.parse().map_err(|_| CursorError::Malformed)?,
            check: (*check).to_owned(),
        };
        if cursor.to_string() != text {
            return Err(CursorError::Malformed);
        }
        Ok(cursor)
    }

    pub fn offset(&self) -> usize {
        self.offset
    }

    /// The row this cursor resumes at in `binding`'s view of `rows` rows.
    pub fn resume(&self, binding: &Binding<'_>, rows: usize) -> Result<usize, CursorError> {
        if self.commit != short(binding.commit) {
            return Err(CursorError::OtherCommit);
        }
        if self.program != short(binding.program) {
            return Err(CursorError::ForeignProgram);
        }
        if self.check != binding.check(self.offset) {
            return Err(CursorError::Tampered);
        }
        if self.offset >= rows {
            return Err(CursorError::OutOfRange);
        }
        Ok(self.offset)
    }
}

impl fmt::Display for Cursor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{VERSION}.{}.{}.{}.{}",
            self.program, self.commit, self.offset, self.check
        )
    }
}
