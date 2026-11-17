//! A step's `posted:<mode>` word: whether a post record about the present
//! bytes of the step's deciding (or bound) candidate exists. It stands
//! beside `deliver:<mode>` on the same step and only there. `[open]` until
//! a record exists, then `[attested]`, never `[satisfied]`
//! ([`writfold_core::delivery::posted`]): a post record is a statement;
//! Writfold sent nothing and saw no post.
use super::delivery::{Deliveries, line};
use super::fold::Step;
use super::judge::Gate;
use crate::deliver::post;
use crate::deliver::step::POSTED;
use crate::session::Session;

impl Deliveries {
    /// The `posted:<mode>` gate of step `s`.
    pub fn posted_gate(&self, session: &Session<'_>, s: &Step, mode: &str) -> Gate {
        let root = &session.ws.root;
        let c = self.decides(s, mode);
        let current = c.and_then(|c| post::current(&post::all(root, c, &self.run), &c.digest()));
        let standing = writfold_core::delivery::posted(current.is_some());
        let command = match (c, &current) {
            (Some(c), None) => Some(crate::deliver::next::post(root, c)),
            _ => None,
        };
        let candidate = current.as_ref().map(|p| {
            line(
                "posted",
                format!(
                    "by {}: {} (a statement; Writfold sent nothing and saw no post)",
                    p.by, p.reference
                ),
                Some(&p.path),
            )
        });
        Gate {
            accept: None,
            id: format!("{POSTED}{mode}"),
            standing: Some(standing),
            evidence: current.as_ref().map(|p| p.path.clone()),
            definition: crate::deliver::candidate::sha(
                format!("writfold-delivery-posted/1\n{mode}").as_bytes(),
            ),
            content: c.map_or_else(String::new, |c| c.digest()),
            read: None,
            attested: true,
            run_record: false,
            command,
            class: String::new(),
            counts: None,
            items: Vec::new(),
            delivery: Some(super::delivery::Delivered {
                summary: match current {
                    Some(_) => "post recorded".into(),
                    None => "no post record".into(),
                },
                candidate,
                detail: None,
                why: None,
                posted: None,
            }),
        }
    }
}
