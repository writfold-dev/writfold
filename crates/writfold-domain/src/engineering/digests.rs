use super::{EngineeringDigest, FingerprintError, FingerprintKind, codec, sealed};
use std::{fmt, str::FromStr};
use vstd::prelude::*;
#[cfg(verus_keep_ghost)]
use vstd::string::StringSliceAdditionalSpecFns;

macro_rules! digest_types {
    ($($name:ident, $kind:ident, $prefix:literal;)+) => {$(
        verus! {
        /// A nominal engineering fingerprint; raw bytes alone confer no authority.
        #[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
        pub struct $name([u8; 32]);

        impl View for $name {
            type V = Seq<u8>;
            closed spec fn view(&self) -> Seq<u8> { self.0@ }
        }

        impl sealed::Sealed for $name {}

        impl EngineeringDigest for $name {
            const KIND: FingerprintKind = FingerprintKind::$kind;

            fn from_digest(bytes: [u8; 32]) -> (result: Self)
                ensures result@ == bytes@,
            {
                Self(bytes)
            }

            fn as_digest(&self) -> (bytes: &[u8; 32])
                ensures bytes@ == self@,
            {
                &self.0
            }
        }

        impl $name {
            pub closed spec fn accepts(text: Seq<char>) -> bool {
                codec::accepts(vstd::utf8::encode_utf8(text), $prefix.spec_bytes())
            }

            fn parse_checked(text: &str) -> (result: Result<Self, FingerprintError>)
                ensures
                    result.is_ok() <==> Self::accepts(text@),
                    result.is_ok() ==> codec::decoded(
                        text.spec_bytes(), $prefix.spec_bytes(), result.unwrap()@),
            {
                proof {
                    broadcast use vstd::string::group_string_axioms;
                    reveal_strlit($prefix);
                    assert(vstd::string::is_ascii($prefix));
                    assert($prefix.spec_bytes().len() <= 64);
                }
                match codec::parse(text, $prefix.as_bytes()) {
                    Ok(bytes) => Ok(Self(bytes)),
                    Err(error) => Err(error),
                }
            }
        }
        }

        impl FromStr for $name {
            type Err = FingerprintError;
            fn from_str(text: &str) -> Result<Self, Self::Err> {
                Self::parse_checked(text)
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                codec::display($prefix, &self.0, f)
            }
        }
    )+};
}

digest_types! {
    BlobDigest, Blob, "wf1:blob:sha256:";
    PackagePayloadDigest, Package, "wf1:package:sha256:";
    DependencyLockDigest, DependencyLock, "wf1:dependency-lock:sha256:";
    SkillRevision, Skill, "wf1:skill:sha256:";
    ProcedureRevision, Procedure, "wf1:procedure:sha256:";
    EngineeringDesignRef, Design, "wf1:design:sha256:";
    RepoVectorDigest, RepoVector, "wf1:repo-vector:sha256:";
    KnowledgeReadSetDigest, ReadSet, "wf1:read-set:sha256:";
    EnvironmentDigest, Environment, "wf1:environment:sha256:";
    AttemptInputDigest, AttemptInput, "wf1:attempt-input:sha256:";
    EffectIntentDigest, EffectIntent, "wf1:effect-intent:sha256:";
    DispatchStampDigest, DispatchStamp, "wf1:dispatch-stamp:sha256:";
    ReceiptDigest, Receipt, "wf1:receipt:sha256:";
    LedgerEventDigest, LedgerEvent, "wf1:ledger-event:sha256:";
    LearningCandidateDigest, Learning, "wf1:learning:sha256:";
    EngineeringReadCutDigest, ReadCut, "wf1:read-cut:sha256:";
}
