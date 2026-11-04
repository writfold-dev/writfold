use super::FingerprintError;
use std::str::FromStr;
use vstd::prelude::*;

macro_rules! fingerprint_kinds {
    ($($kind:ident => $text:literal),+ $(,)?) => {
        verus! {
        /// The sixteen domains in the revision-5 fingerprint registry.
        #[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
        pub enum FingerprintKind { $($kind),+ }

        impl FingerprintKind {
            pub closed spec fn spelling(self) -> Seq<char> {
                match self { $(Self::$kind => $text@),+ }
            }

            pub fn as_str(&self) -> (text: &'static str)
                ensures text@ == self.spelling(),
            {
                match self { $(Self::$kind => $text),+ }
            }

            pub closed spec fn accepts(text: Seq<char>) -> bool {
                false $(|| text == $text@)+
            }

            fn parse_checked(text: &str) -> (result: Result<Self, FingerprintError>)
                ensures
                    result.is_ok() <==> Self::accepts(text@),
                    result.is_ok() ==> result.unwrap().spelling() == text@,
            {
                $(if text.eq($text) { return Ok(Self::$kind); })+
                Err(FingerprintError)
            }
        }
        }
    };
}

fingerprint_kinds! {
    AttemptInput => "attempt-input",
    Blob => "blob",
    DependencyLock => "dependency-lock",
    Design => "design",
    EffectIntent => "effect-intent",
    Environment => "environment",
    Learning => "learning",
    LedgerEvent => "ledger-event",
    Package => "package",
    Procedure => "procedure",
    ReadCut => "read-cut",
    ReadSet => "read-set",
    Receipt => "receipt",
    RepoVector => "repo-vector",
    Skill => "skill",
    DispatchStamp => "dispatch-stamp",
}

impl FromStr for FingerprintKind {
    type Err = FingerprintError;
    fn from_str(text: &str) -> Result<Self, Self::Err> {
        Self::parse_checked(text)
    }
}
