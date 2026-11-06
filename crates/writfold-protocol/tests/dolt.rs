use writfold_domain::{DoltToolGeneration, SchemaEpoch, dolt::DoltReadPin};
use writfold_protocol::dolt::{ReadRefusal, admit_read};

type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;

fn pin(change: Option<(&str, &str)>) -> Result<DoltReadPin> {
    let value = |key, original: &str| -> String {
        match change {
            Some((changed, replacement)) if key == changed => replacement.to_owned(),
            _ => original.to_owned(),
        }
    };
    Ok(DoltReadPin::new(
        value("realm", "realm-a").try_into()?,
        value("kb", "knowledge base").try_into()?,
        value("database", "corpus-a").try_into()?,
        value("commit", "00000000000000000000000000000001").try_into()?,
        SchemaEpoch::new(
            value("epoch", "epoch-1").try_into()?,
            value("schema", &"1".repeat(64)).try_into()?,
            value("query", &"2".repeat(64)).try_into()?,
        ),
        value("profile", "dolt-corpus-read-v1").try_into()?,
        value("producer", &"a".repeat(64)).try_into()?,
    ))
}

#[test]
fn complete_pin_and_actual_tool_are_both_required() -> Result {
    let expected = pin(None)?;
    assert_eq!(
        admit_read(&expected, &expected, expected.producer()),
        Ok(())
    );
    for (field, replacement, refusal) in [
        ("realm", "realm-b", ReadRefusal::Realm),
        ("kb", "another knowledge base", ReadRefusal::Kb),
        ("database", "corpus-b", ReadRefusal::Database),
        (
            "commit",
            "00000000000000000000000000000002",
            ReadRefusal::Commit,
        ),
        ("epoch", "epoch-2", ReadRefusal::Epoch),
        ("schema", &"3".repeat(64), ReadRefusal::Epoch),
        ("query", &"4".repeat(64), ReadRefusal::Epoch),
        ("profile", "dolt-corpus-read-v2", ReadRefusal::Profile),
        ("producer", &"b".repeat(64), ReadRefusal::Producer),
    ] {
        let observed = pin(Some((field, replacement)))?;
        assert_eq!(
            admit_read(&expected, &observed, expected.producer()),
            Err(refusal)
        );
    }
    let other_tool: DoltToolGeneration = "c".repeat(64).parse()?;
    assert_eq!(
        admit_read(&expected, &expected, &other_tool),
        Err(ReadRefusal::Producer)
    );
    Ok(())
}

#[test]
fn equal_but_unsupported_profiles_do_not_admit() -> Result {
    let unsupported = pin(Some(("profile", "dolt-corpus-read-v2")))?;
    assert_eq!(
        admit_read(&unsupported, &unsupported, unsupported.producer()),
        Err(ReadRefusal::Profile)
    );
    Ok(())
}
