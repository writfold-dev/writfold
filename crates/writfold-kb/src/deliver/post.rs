//! A post record (`deliver posted`): a statement that the payload was
//! posted, with where (`--ref`, recorded as given and never fetched) and
//! by whom. Writfold sent nothing and saw no post. One create-new file
//! `posts/post-<time>-<hex>.json` in the candidate's directory, schema
//! `writfold-delivery-post/1`; several may exist. A record is about the
//! present exactly when its content digest is the candidate's present one
//! ([`writfold_core::gate::about`]); after an edit it is stale and counts
//! for nothing. A record made while the candidate was not ready keeps
//! `!unready-when-recorded` and still counts as a post: it says what
//! happened.
use super::candidate::Candidate;
use super::record;
use serde_json::Value;
use std::path::Path;

pub const SCHEMA: &str = "writfold-delivery-post/1";
pub const DIR: &str = "posts";
pub const PREFIX: &str = "post-";
/// The longest `--ref` kept.
pub const MAX_REF: usize = 2000;

/// One valid post record of a candidate.
#[derive(Clone, Debug)]
pub struct Post {
    pub path: String,
    pub by: String,
    pub reference: String,
    pub content: String,
    pub ready: bool,
    pub seq: u64,
}

impl Post {
    /// Whether it is about the candidate's present bytes.
    pub fn current(&self, content: &str) -> bool {
        writfold_core::gate::about(b"", self.content.as_bytes(), b"", content.as_bytes())
    }
}

pub fn dir(c: &Candidate) -> String {
    format!("{}/{DIR}", c.dir)
}

fn read(value: &Value, path: String) -> Option<Post> {
    Some(Post {
        path,
        by: record::text(value, "by").ok()?,
        reference: record::text(value, "ref").ok()?,
        content: record::text(value, "content").ok()?,
        ready: value.get("ready").and_then(Value::as_bool)?,
        seq: value.get("seq").and_then(Value::as_u64)?,
    })
}

/// The candidate's valid post records of `run`, by `(seq, name)`.
pub fn all(root: &Path, c: &Candidate, run: &str) -> Vec<Post> {
    let mut out: Vec<(u64, String, Post)> = record::files(root, &dir(c), PREFIX)
        .into_iter()
        .filter_map(|(name, path, text)| {
            let value = record::open(&text?, SCHEMA).ok()?;
            let ours = record::text(&value, "candidate").ok()? == c.id
                && record::text(&value, "run").ok()? == run;
            let post = read(&value, path).filter(|_| ours)?;
            Some((post.seq, name, post))
        })
        .collect();
    out.sort_by(|a, b| (a.0, &a.1).cmp(&(b.0, &b.1)));
    out.into_iter().map(|(_, _, p)| p).collect()
}

/// The newest record about the candidate's present bytes.
pub fn current(posts: &[Post], content: &str) -> Option<Post> {
    posts.iter().rev().find(|p| p.current(content)).cloned()
}

/// The `posted:` line of a candidate: the newest current record, else the
/// newest stale one.
pub fn line(posts: &[Post], content: &str) -> Option<crate::emit::More> {
    let (post, status) = match current(posts, content) {
        Some(p) => (p, "attested"),
        None => (posts.last()?.clone(), "stale"),
    };
    let mut text = match status {
        "stale" => format!(
            "[stale] by {}: {} (the candidate was edited after this record)",
            post.by, post.reference
        ),
        _ => format!(
            "by {}: {} (a statement; Writfold sent nothing and saw no post)",
            post.by, post.reference
        ),
    };
    let mut flags = Vec::new();
    if !post.ready {
        flags.push("unready-when-recorded".to_owned());
        text.push_str(" (recorded while the candidate was not ready)");
    }
    Some(crate::emit::More {
        label: "posted",
        text,
        path: Some(post.path),
        line: Some(1),
        status: Some(status.into()),
        flags,
        ..crate::emit::More::default()
    })
}
