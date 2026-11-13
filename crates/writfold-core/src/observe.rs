//! Observations behind `writfold kb observe`: one record of looking at an
//! outside source during a run, with how the look went. Only an `ok`
//! look carries what was seen as evidence; every other status records
//! that someone looked and could not tell (or could tell only part), so
//! it is shown but never grounds a claim. Nothing here reads a file or a
//! citation; the caller says whether the embedded citation is fresh.

/// How one look at a source went.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Status {
    /// The source answered in full.
    Ok,
    /// The source answered in part.
    Partial,
    /// The source answered, with nothing for the question.
    NoData,
    /// The thing looked for does not exist there.
    NotFound,
    /// The source did not answer in time.
    Timeout,
    /// The source refused access.
    Denied,
    /// The look failed for another reason.
    Error,
}

/// Every status, in the order they are documented.
pub const ALL: [Status; 7] = [
    Status::Ok,
    Status::Partial,
    Status::NoData,
    Status::NotFound,
    Status::Timeout,
    Status::Denied,
    Status::Error,
];

impl Status {
    pub fn name(self) -> &'static str {
        match self {
            Status::Ok => "ok",
            Status::Partial => "partial",
            Status::NoData => "no-data",
            Status::NotFound => "not-found",
            Status::Timeout => "timeout",
            Status::Denied => "denied",
            Status::Error => "error",
        }
    }

    pub fn parse(text: &str) -> Option<Self> {
        match text {
            "ok" => Some(Status::Ok),
            "partial" => Some(Status::Partial),
            "no-data" => Some(Status::NoData),
            "not-found" => Some(Status::NotFound),
            "timeout" => Some(Status::Timeout),
            "denied" => Some(Status::Denied),
            "error" => Some(Status::Error),
            _ => None,
        }
    }
}

/// Whether an observation of this status records what was seen as a
/// citation: `ok` and `partial` do (exactly one token); the others record
/// only that the look did not answer.
pub fn embeds(status: Status) -> bool {
    match status {
        Status::Ok | Status::Partial => true,
        Status::NoData | Status::NotFound | Status::Timeout | Status::Denied | Status::Error => {
            false
        }
    }
}

/// Whether an observation grounds evidence: only an `ok` look whose
/// embedded citations are all fresh (and there is at least one).
pub fn grounds(status: Status, cites_fresh: bool) -> bool {
    match status {
        Status::Ok => cites_fresh,
        Status::Partial
        | Status::NoData
        | Status::NotFound
        | Status::Timeout
        | Status::Denied
        | Status::Error => false,
    }
}

#[cfg(test)]
mod tests {
    use super::{ALL, Status, embeds, grounds};

    /// Over every status and freshness: grounded exactly for a fresh `ok`.
    #[test]
    fn only_a_fresh_ok_grounds() {
        for status in ALL {
            for fresh in [false, true] {
                assert_eq!(
                    grounds(status, fresh),
                    status == Status::Ok && fresh,
                    "{status:?} {fresh}"
                );
            }
            assert_eq!(
                embeds(status),
                matches!(status, Status::Ok | Status::Partial),
                "{status:?}"
            );
        }
    }

    #[test]
    fn names_round_trip() {
        for status in ALL {
            assert_eq!(Status::parse(status.name()), Some(status));
        }
        assert_eq!(Status::parse("fresh"), None);
        assert_eq!(Status::parse("OK"), None);
    }
}
