use std::{any::TypeId, collections::HashSet, error::Error, fmt};
use writfold_domain::engineering::*;

const KINDS: [&str; 16] = [
    "blob",
    "package",
    "dependency-lock",
    "skill",
    "procedure",
    "design",
    "repo-vector",
    "read-set",
    "environment",
    "attempt-input",
    "effect-intent",
    "dispatch-stamp",
    "receipt",
    "ledger-event",
    "learning",
    "read-cut",
];
const HEX: &str = "00010f107f80feff0123456789abcdef00112233445566778899aabbccddeeff";
const BYTES: [u8; 32] = [
    0x00, 0x01, 0x0f, 0x10, 0x7f, 0x80, 0xfe, 0xff, 0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef,
    0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0x99, 0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff,
];

type Domain = (&'static str, TypeId, fn(&str) -> bool);
fn domain<D: EngineeringDigest + std::str::FromStr + 'static>() -> Domain {
    (D::KIND.as_str(), TypeId::of::<D>(), |text| {
        text.parse::<D>().is_ok()
    })
}

#[test]
fn all_domains_are_nominal_and_reject_every_other_public_namespace() {
    let domains = [
        domain::<BlobDigest>(),
        domain::<PackagePayloadDigest>(),
        domain::<DependencyLockDigest>(),
        domain::<SkillRevision>(),
        domain::<ProcedureRevision>(),
        domain::<EngineeringDesignRef>(),
        domain::<RepoVectorDigest>(),
        domain::<KnowledgeReadSetDigest>(),
        domain::<EnvironmentDigest>(),
        domain::<AttemptInputDigest>(),
        domain::<EffectIntentDigest>(),
        domain::<DispatchStampDigest>(),
        domain::<ReceiptDigest>(),
        domain::<LedgerEventDigest>(),
        domain::<LearningCandidateDigest>(),
        domain::<EngineeringReadCutDigest>(),
    ];
    let mut nominal_types = HashSet::new();
    for (index, (kind, id, accepts)) in domains.iter().enumerate() {
        assert_eq!(*kind, KINDS[index]);
        assert!(nominal_types.insert(id));
        for candidate in KINDS {
            let text = format!("wf1:{candidate}:sha256:{HEX}");
            assert_eq!(
                accepts(&text),
                *kind == candidate,
                "{kind} accepting {candidate}"
            );
        }
    }
}

#[test]
fn registry_is_closed_and_exact() -> Result<(), Box<dyn Error>> {
    let mut kinds = HashSet::new();
    for text in KINDS {
        let kind: FingerprintKind = text.parse()?;
        assert_eq!(kind.as_str(), text);
        assert!(kinds.insert(kind));
        for changed in [text.to_uppercase(), format!(" {text}"), format!("{text}\n")] {
            assert!(changed.parse::<FingerprintKind>().is_err());
        }
    }
    for unknown in [
        "",
        "sha256",
        "read_set",
        "BlobDigest",
        "unknown",
        "blób",
        "blob\0",
    ] {
        assert!(unknown.parse::<FingerprintKind>().is_err());
    }
    Ok(())
}

#[test]
fn independent_hex_vector_decodes_nibble_and_byte_boundaries() -> Result<(), Box<dyn Error>> {
    let text = format!("wf1:blob:sha256:{HEX}");
    let digest: BlobDigest = text.parse()?;
    assert_eq!(digest.as_digest(), &BYTES);
    assert_eq!(BlobDigest::from_digest(BYTES).to_string(), text);
    for (byte, hex) in [(0, '0'), (255, 'f')] {
        let expected = format!("wf1:blob:sha256:{}", hex.to_string().repeat(64));
        assert_eq!(expected.parse::<BlobDigest>()?.as_digest(), &[byte; 32]);
        assert_eq!(BlobDigest::from_digest([byte; 32]).to_string(), expected);
    }
    Ok(())
}

#[test]
fn exact_framing_rejects_normalization_and_algorithm_substitution() -> Result<(), Box<dyn Error>> {
    let valid = format!("wf1:blob:sha256:{HEX}");
    for text in [
        String::new(),
        HEX.to_owned(),
        format!("sha256:{HEX}"),
        format!("wf0:blob:sha256:{HEX}"),
        format!("wf2:blob:sha256:{HEX}"),
        format!("writfold-engineering/1:blob:sha256:{HEX}"),
        format!("wf1:Blob:sha256:{HEX}"),
        format!("wf1:blob:SHA256:{HEX}"),
        format!("wf1:blob:sha512:{HEX}"),
        format!("wf1:blob:sha256::{HEX}"),
        format!(" {valid}"),
        format!("{valid}\n"),
        format!("{valid}\0"),
        format!("{valid}0"),
        valid[..valid.len() - 1].to_owned(),
        format!("wf1:blób:sha256:{HEX}"),
    ] {
        assert!(text.parse::<BlobDigest>().is_err(), "accepted {text:?}");
    }
    for position in 0..64 {
        for invalid in [b'A', b'F', b'G', b'g', b'/', b':', b' ', b'\n', 0, 127] {
            let mut text = valid.as_bytes().to_vec();
            text["wf1:blob:sha256:".len() + position] = invalid;
            let text = std::str::from_utf8(&text)?;
            assert!(text.parse::<BlobDigest>().is_err(), "accepted {text:?}");
        }
    }
    assert!(
        format!("wf1:blob:sha256:é{}", &HEX[2..])
            .parse::<BlobDigest>()
            .is_err()
    );
    Ok(())
}

struct Buffer {
    bytes: [u8; 128],
    len: usize,
    capacity: usize,
}
impl fmt::Write for Buffer {
    fn write_str(&mut self, text: &str) -> fmt::Result {
        if text.len() > self.capacity - self.len {
            return Err(fmt::Error);
        }
        self.bytes[self.len..self.len + text.len()].copy_from_slice(text.as_bytes());
        self.len += text.len();
        Ok(())
    }
}

#[test]
fn streaming_display_uses_caller_storage_and_propagates_capacity_failure()
-> Result<(), Box<dyn Error>> {
    let digest = BlobDigest::from_digest(BYTES);
    let mut output = Buffer {
        bytes: [0; 128],
        len: 0,
        capacity: 128,
    };
    fmt::write(&mut output, format_args!("{digest}"))?;
    assert_eq!(
        std::str::from_utf8(&output.bytes[..output.len])?,
        "wf1:blob:sha256:00010f107f80feff0123456789abcdef00112233445566778899aabbccddeeff"
    );
    let mut short = Buffer {
        bytes: [0; 128],
        len: 0,
        capacity: output.len - 1,
    };
    assert!(fmt::write(&mut short, format_args!("{digest}")).is_err());
    assert_eq!(short.len, short.capacity);
    assert_eq!(&short.bytes[..short.len], &output.bytes[..short.len]);
    Ok(())
}

#[test]
fn equality_hash_and_order_use_all_digest_bytes() -> Result<(), Box<dyn Error>> {
    let zero = BlobDigest::from_digest([0; 32]);
    let mut last = [0; 32];
    last[31] = 1;
    let last = BlobDigest::from_digest(last);
    let mut first = [0; 32];
    first[0] = 1;
    let first = BlobDigest::from_digest(first);
    assert!(zero < last && last < first);
    let equal: BlobDigest = zero.to_string().parse()?;
    let mut distinct = HashSet::new();
    assert!(distinct.insert(zero));
    assert!(!distinct.insert(equal));
    assert!(distinct.insert(last));
    assert!(distinct.insert(first));
    Ok(())
}
