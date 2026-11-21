//! The checked values accept exactly their grammar and say what they are.
use writfold_domain::text::whitespace;
use writfold_domain::{
    ByteCount, ContentDigest, DesignRevision, GitObjectFormat, GitObjectId,
    ProposalMaterialRevision,
};

#[test]
fn git_object_ids_take_their_format_from_their_width() {
    let sha1 = "a".repeat(40);
    let sha256 = "0".repeat(64);
    let id = GitObjectId::try_from(sha1.clone()).map(|i| i.format());
    assert_eq!(id, Ok(GitObjectFormat::Sha1));
    let id = GitObjectId::try_from(sha256.clone()).map(|i| i.format());
    assert_eq!(id, Ok(GitObjectFormat::Sha256));
    for bad in ["A".repeat(40), "g".repeat(40), "a".repeat(39), String::new()] {
        let refused = GitObjectId::try_from(bad);
        assert!(refused.is_err_and(|e| e.to_string() == "invalid GitObjectId"));
    }
    assert!(GitObjectId::new(GitObjectFormat::Sha256, sha1).is_err());
    assert!(GitObjectId::new(GitObjectFormat::Sha256, sha256).is_ok());
}

#[test]
fn digests_revisions_and_counts() {
    assert!(ContentDigest::try_from("f".repeat(64)).is_ok());
    assert!(ContentDigest::try_from(" ".repeat(64)).is_err());
    assert!("x".parse::<DesignRevision>().is_ok());
    assert!(String::new().parse::<ProposalMaterialRevision>().is_err());
    assert_eq!(ByteCount::new(7).map(ByteCount::get), Ok(7));
    assert!(whitespace('\u{3000}') && whitespace(' ') && !whitespace('x'));
}
