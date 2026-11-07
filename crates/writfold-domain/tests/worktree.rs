use writfold_domain::worktree::*;

type TestResult = std::result::Result<(), Box<dyn std::error::Error>>;

#[test]
fn scope_references_reject_noncanonical_and_oversized_tokens() {
    for invalid in ["", " ", "_first", "a/b", "é", "a\nb"] {
        assert!(invalid.parse::<RepositoryId>().is_err());
        assert!(invalid.parse::<WorktreeId>().is_err());
        assert!(invalid.parse::<BindingRevision>().is_err());
    }
    assert!("a".repeat(4096).parse::<BindingRevision>().is_ok());
    assert!("a".repeat(4097).parse::<BindingRevision>().is_err());
    assert!("repo:Ab_1.2-3".parse::<RepositoryId>().is_ok());
}

#[test]
fn logical_effect_id_is_a_checked_non_digest_identity() -> TestResult {
    let id: EffectId = "effect:Ab_1.2-3".parse()?;
    assert_eq!(id.to_string(), "effect:Ab_1.2-3");
    for invalid in [
        "", " ", " effect", "effect ", "_effect", "éffect", "a/b", "a\0b",
    ] {
        assert!(invalid.parse::<EffectId>().is_err());
    }
    assert!("a".repeat(256).parse::<EffectId>().is_ok());
    assert!("a".repeat(257).parse::<EffectId>().is_err());
    Ok(())
}

#[test]
fn six_nominal_digest_namespaces_do_not_interchange() -> TestResult {
    let hash = "00".repeat(32);
    let recipe: RecipeDigest = format!("wfwt1:recipe:sha256:{hash}").parse()?;
    assert_eq!(recipe.to_string(), format!("wfwt1:recipe:sha256:{hash}"));
    assert!(recipe.to_string().parse::<CaptureDigest>().is_err());
    assert!(recipe.to_string().parse::<CandidateDigest>().is_err());
    assert!(recipe.to_string().parse::<PreparedEditDigest>().is_err());
    assert!(recipe.to_string().parse::<RequestKeyDigest>().is_err());
    assert!(recipe.to_string().parse::<ObservationDigest>().is_err());
    assert!(
        format!("wf1:recipe:sha256:{hash}")
            .parse::<RecipeDigest>()
            .is_err()
    );
    assert!(
        format!("wfwt1:recipe:sha256:{}", "AA".repeat(32))
            .parse::<RecipeDigest>()
            .is_err()
    );
    Ok(())
}
#[test]
fn contained_literal_path_and_posix_policy() -> TestResult {
    let scope = Scope::new(
        "realm".parse()?,
        "run".parse()?,
        "repo".parse()?,
        "tree".parse()?,
        "binding".parse()?,
    )?;
    let time = PosixTimestamp::new(-1, 999_999_999)?;
    let identity = PosixIdentity::new(1, 2);
    let metadata = PosixMetadata::new(identity, 0o100644, 3, 4, 1, time, time)?;
    for path in ["", "/x", "a/../x", "./x", "a//x", "a/", "a\\b", "a\0b"] {
        assert!(Capture::new(scope.clone(), path.into(), vec![], metadata.clone()).is_err());
    }
    assert!(Capture::new(scope, "é/文.txt".into(), vec![0, 255], metadata).is_ok());
    assert!(PosixMetadata::new(identity, 0o120644, 3, 4, 1, time, time).is_err());
    assert!(PosixMetadata::new(identity, 0o100644, 3, 4, 2, time, time).is_err());
    assert!(PosixTimestamp::new(0, 1_000_000_000).is_err());
    assert_eq!(time.seconds(), -1);
    assert_eq!(time.nanoseconds(), 999_999_999);
    Ok(())
}
#[test]
fn normalized_original_byte_splices_are_not_character_selectors() -> TestResult {
    let r = RecipeDigest::from_digest([1; 32]);
    let c = CaptureDigest::from_digest([2; 32]);
    let n = CandidateDigest::from_digest([3; 32]);
    let artifact = Artifact::new("artifact/1".into(), [4; 32])?;
    let origin = RecipeSpan::new(0, 1)?;
    let make = |splices| {
        PreparedEdit::new(
            r,
            c,
            n,
            artifact.clone(),
            artifact.clone(),
            splices,
            vec![ReadRange::new(0, "α\r\n".as_bytes().to_vec())?],
        )
    };
    let p = make(vec![Splice::new(0, 2, "β".as_bytes().to_vec(), origin)?])?;
    assert!(
        p.validate_bytes("α\r\n".as_bytes(), "β\r\n".as_bytes())
            .is_ok()
    );
    assert!(
        p.validate_bytes("α\r\n".as_bytes(), "β\n".as_bytes())
            .is_err()
    );
    assert!(
        p.validate_bytes("α\n".as_bytes(), "β\r\n".as_bytes())
            .is_err()
    );
    assert!(
        make(vec![
            Splice::new(0, 2, vec![1], origin)?,
            Splice::new(2, 3, vec![2], origin)?
        ])
        .is_ok()
    );
    assert!(
        make(vec![
            Splice::new(0, 2, vec![1], origin)?,
            Splice::new(2, 2, vec![2], origin)?
        ])
        .is_err()
    );
    assert!(
        make(vec![
            Splice::new(0, 0, vec![1], origin)?,
            Splice::new(0, 2, vec![2], origin)?
        ])
        .is_err()
    );
    assert!(Splice::new(3, 2, vec![1], origin).is_err());
    assert!(Splice::new(0, 0, vec![], origin).is_ok());
    assert!(p.validate_origins(b"x").is_ok());
    assert!(p.validate_origins(b"").is_err());
    assert!(ReadRange::new(u64::MAX, vec![1]).is_err());
    Ok(())
}

#[test]
fn insertion_into_existing_empty_capture_is_content_replacement() -> TestResult {
    let artifact = Artifact::new("artifact/1".into(), [0; 32])?;
    let prepared = PreparedEdit::new(
        RecipeDigest::from_digest([1; 32]),
        CaptureDigest::from_digest([2; 32]),
        CandidateDigest::from_digest([3; 32]),
        artifact.clone(),
        artifact,
        vec![Splice::new(0, 0, vec![255, 0], RecipeSpan::new(0, 0)?)?],
        vec![],
    )?;
    assert!(prepared.validate_bytes(&[], &[255, 0]).is_ok());
    assert!(prepared.validate_bytes(&[], &[0, 255]).is_err());
    Ok(())
}
