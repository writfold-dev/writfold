//! Which repository a gate judges when no `--repo` is given: the one
//! enclosing the working directory, the run's recorded worktree, or the
//! KB's own. The caller finds the repositories and says what each is; this
//! module only decides.
//!
//! The recorded worktree is for a KB kept in its own repository, where the
//! working directory is the KB clone and the code is elsewhere. It is never
//! used when it is another checkout of the KB's own repository (the same
//! Git common directory, or a clone holding this KB at the same prefix):
//! that checkout's work tree and commit are another clone's, not the one
//! the command runs in.
//!
//! [`pick`] is translated and proved (`proofs/Writfold/Subject.lean`).

/// Where the subject comes from.
pub enum Pick {
    /// The repository enclosing the working directory.
    Here,
    /// The run's recorded worktree.
    Recorded,
    /// The KB's own repository.
    Kb,
    /// None: there is no subject.
    Nothing,
}

/// The subject: the repository enclosing the working directory when it is
/// not the KB's own; else the recorded worktree when it is a repository
/// and not another checkout of the KB's; else the enclosing repository;
/// else, when the recorded worktree was a checkout of the KB's repository,
/// the KB's own; else nothing.
///
/// `here`: the working directory is in a repository; `here_is_kb`: that
/// repository is the KB's own work tree. `recorded`: the run's recorded
/// worktree exists here and is in a repository; `recorded_is_kb`: that
/// repository is a checkout of the KB's repository.
pub fn pick(here: bool, here_is_kb: bool, recorded: bool, recorded_is_kb: bool) -> Pick {
    if here && !here_is_kb {
        return Pick::Here;
    }
    if recorded && !recorded_is_kb {
        return Pick::Recorded;
    }
    if here {
        return Pick::Here;
    }
    if recorded {
        return Pick::Kb;
    }
    Pick::Nothing
}

#[cfg(test)]
mod tests {
    use super::{Pick, pick};

    fn word(p: Pick) -> &'static str {
        match p {
            Pick::Here => "here",
            Pick::Recorded => "recorded",
            Pick::Kb => "kb",
            Pick::Nothing => "nothing",
        }
    }

    #[test]
    fn every_input_is_decided_by_the_table() {
        for bits in 0u8..16 {
            let [h, hk, r, rk] = [bits & 1, bits & 2, bits & 4, bits & 8].map(|b| b != 0);
            let got = word(pick(h, hk, r, rk));
            let want = if h && !hk {
                "here"
            } else if r && !rk {
                "recorded"
            } else if h {
                "here"
            } else if r {
                "kb"
            } else {
                "nothing"
            };
            assert_eq!(got, want, "{h} {hk} {r} {rk}");
            // Another checkout of the KB's repository is never judged.
            assert!(!(got == "recorded" && rk));
        }
    }
}
