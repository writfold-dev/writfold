use sha2::{Digest, Sha256};
use splicery_compile::CompileError;
use splicery_domain::Profile;
use splicery_kernel::KernelError;
use writfold_sources::splicery::{ReplayError, replay};

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn actual_replay_retains_sorted_original_coordinates_and_origins() -> TestResult {
    let recipe = br#"edit "src/f" {
replace one text "world" with "earth"
replace one text "hello" with "hi"
}"#;
    let edit = replay(b"hello world".to_vec(), "src/f", recipe)?;
    assert_eq!(edit.candidate(), b"hi earth");
    assert_eq!(edit.relative_path(), "src/f");
    assert_eq!(edit.edits().len(), 2);
    let first = &edit.edits()[0];
    assert_eq!((first.target.lo(), first.target.hi()), (0, 5));
    assert_eq!(first.replacement, b"hi");
    assert_eq!(
        &recipe[first.origin.start..first.origin.end],
        br#"replace one text "hello" with "hi""#
    );
    let second = &edit.edits()[1];
    assert_eq!((second.target.lo(), second.target.hi()), (6, 11));
    assert_eq!(&first.target.capture_sha256(), edit.source_sha256());
    assert_eq!(
        edit.source_sha256(),
        &<[u8; 32]>::from(Sha256::digest(b"hello world"))
    );
    assert_eq!(
        edit.candidate_sha256(),
        &<[u8; 32]>::from(Sha256::digest(b"hi earth"))
    );
    assert!(!edit.no_change());
    assert_eq!(edit.into_candidate(), b"hi earth");
    Ok(())
}

#[test]
fn rejects_wrong_or_noncanonical_paths_before_lowering() {
    let recipe = br#"edit "f" { delete @missing }"#;
    assert_eq!(
        replay(vec![], "other", recipe).err(),
        Some(ReplayError::WrongPath)
    );
    for path in [
        "", "/f", "../f", "a/../f", "./f", "a//f", "a/", "C:/f", "a\\f", "a\0f",
    ] {
        assert_eq!(
            replay(vec![], path, recipe).err(),
            Some(ReplayError::InvalidPath)
        );
    }
    for path in ["../f", "/f", "./f", "a//f"] {
        let recipe = format!("edit {path:?} {{ delete @missing }}");
        assert_eq!(
            replay(vec![], "f", recipe.as_bytes()).err(),
            Some(ReplayError::InvalidPath)
        );
    }
    assert!(matches!(
        replay(vec![], "f", b"delete @missing"),
        Err(ReplayError::Parse(_))
    ));
}

#[test]
fn refuses_file_operations_and_unresolved_handles() {
    for recipe in [
        &b"create \"f\" with \"new\""[..],
        &b"rename \"f\" to \"g\""[..],
        &b"remove \"f\""[..],
    ] {
        assert_eq!(
            replay(vec![], "f", recipe).err(),
            Some(ReplayError::UnsupportedFileOperation)
        );
    }
    for recipe in [
        &br#"edit "f" { delete @missing }"#[..],
        &br#"edit "f" { each @r in frozen @saved_rows { delete @r } }"#[..],
    ] {
        let result = replay(b"abc".to_vec(), "f", recipe);
        assert!(matches!(result, Err(ReplayError::Lower(_))), "{result:?}");
    }
}

#[test]
fn refuses_no_match_ambiguity_and_overlapping_edits() {
    let recipe = br#"edit "f" { replace one text "old" with "new" }"#;
    for bytes in [&b"stale"[..], &b"old old"[..]] {
        assert!(matches!(
            replay(bytes.to_vec(), "f", recipe),
            Err(ReplayError::Lower(_))
        ));
    }
    let overlap = br#"edit "f" {
replace one text "abc" with "x"
delete one text "bc"
}"#;
    assert_eq!(
        replay(b"abc".to_vec(), "f", overlap).err(),
        Some(ReplayError::Kernel(KernelError::Overlap))
    );
    let unresolved = br#"edit "f" { delete @old_capture }"#;
    assert!(matches!(replay(b"abc".to_vec(), "f", unresolved),
        Err(ReplayError::Lower(error)) if error.kind == CompileError::UnknownBinding));
}

#[test]
fn capture_digest_binds_exact_bytes_not_assumed_freshness() -> TestResult {
    // The API has no expected-capture parameter; the owner must compare this
    // digest to its retained capture. Matching selectors alone prove no freshness.
    let recipe = br#"edit "f" { replace one text "old" with "new" }"#;
    let first = replay(b"old\n".to_vec(), "f", recipe)?;
    let changed = replay(b"old\r\n".to_vec(), "f", recipe)?;
    assert_ne!(first.source_sha256(), changed.source_sha256());
    assert_eq!(changed.candidate(), b"new\r\n");
    Ok(())
}

#[test]
fn preserves_unicode_crlf_raw_and_no_change_semantics() -> TestResult {
    let recipe = "edit \"f\" { replace one text \"café\" with \"茶\" }";
    let edit = replay("café\r\nend\n".as_bytes().to_vec(), "f", recipe.as_bytes())?;
    assert_eq!(edit.candidate(), "茶\r\nend\n".as_bytes());
    assert_eq!(edit.source_profile(), &Profile::Text);
    assert_eq!(edit.edits()[0].target.hi(), 5);
    let unchanged = replay(
        b"same\r\n".to_vec(),
        "f",
        br#"edit "f" { replace one text "same" with "same" }"#,
    )?;
    assert!(unchanged.no_change());
    assert_eq!(unchanged.source_sha256(), unchanged.candidate_sha256());
    let raw = replay(
        vec![0xff, 0, 0xfe],
        "f",
        br#"edit "f" as bytes { replace one hex"00" with hex"80" }"#,
    )?;
    assert_eq!(raw.source_profile(), &Profile::Raw);
    assert_eq!(raw.candidate(), &[0xff, 0x80, 0xfe]);
    assert_eq!(
        replay(vec![0xff], "f", br#"edit "f" { delete range [0,1) }"#).err(),
        Some(ReplayError::InvalidSource)
    );
    Ok(())
}

#[test]
fn equal_output_does_not_erase_recipe_identity() -> TestResult {
    let first = replay(
        b"old".to_vec(),
        "f",
        br#"edit "f" { replace one text "old" with "new" }"#,
    )?;
    let second = replay(
        b"old".to_vec(),
        "f",
        br#"edit "f" { replace range [0,3) with "new" }"#,
    )?;
    assert_eq!(first.candidate_sha256(), second.candidate_sha256());
    assert_ne!(first.recipe_sha256(), second.recipe_sha256());
    Ok(())
}

#[test]
fn retains_pinned_source_and_recipe_limits() {
    let oversized = vec![b'x'; splicery_domain::MAX_INPUT_BYTES + 1];
    assert_eq!(
        replay(oversized, "f", br#"edit "f" { delete range [0,1) }"#).err(),
        Some(ReplayError::InvalidSource)
    );
    let recipe = vec![b' '; splicery_domain::MAX_RECIPE_BYTES + 1];
    assert!(matches!(
        replay(vec![], "f", &recipe),
        Err(ReplayError::Parse(_))
    ));
}

#[test]
fn pinned_lowerer_and_kernel_refuse_foreign_capture_coordinates() -> TestResult {
    use splicery_domain::{
        CaptureBinding, EditTarget, OriginalImage, ReadCapture, RecipeSpan, Splice,
    };
    use splicery_kernel::PreparedBatch;

    let old = OriginalImage::new(b"old".to_vec(), Profile::Text).ok_or("old image")?;
    let current = OriginalImage::new(b"new".to_vec(), Profile::Text).ok_or("current image")?;
    let capture = CaptureBinding {
        name: "captured".to_owned(),
        span: ReadCapture::new(&old, 0, 3).ok_or("capture")?,
    };
    let parsed = splicery_syntax::parse_recipe(br#"edit "f" { delete @captured }"#, None)
        .map_err(|error| format!("{error:?}"))?;
    assert!(
        matches!(splicery_compile::lower(&parsed, &current, &[capture], &[]),
        Err(error) if error.kind == CompileError::OutOfBounds)
    );
    let stale = Splice {
        target: EditTarget::new(&old, 0, 3).ok_or("target")?,
        replacement: vec![],
        origin: RecipeSpan { start: 0, end: 1 },
    };
    assert!(matches!(
        PreparedBatch::new(current, vec![stale]),
        Err(KernelError::OutOfBounds)
    ));
    Ok(())
}
