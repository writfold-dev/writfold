//! The resume cursor's canonical encoding and its typed refusals: a round
//! trip, a foreign program, another commit, malformed or older text, an
//! edited offset or result, and an offset past the end.
use tuft::cursor::{Binding, Cursor, CursorError};

type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;

const PROGRAM: &str = "9558f72e5876aa00112233445566778899aabbccddeeff00112233445566778899";
const COMMIT: &str = "e56621f0a1b2c3d4e5f60718293a4b5c6d7e8f90";
const VIEW: &str = "0f0e0d0c0b0a09080706050403020100ffeeddccbbaa99887766554433221100";

fn binding<'a>(program: &'a str, commit: &'a str, view: &'a str) -> Binding<'a> {
    Binding {
        program,
        commit,
        view,
    }
}

#[test]
fn an_issued_cursor_round_trips_and_resumes() -> Result {
    let here = binding(PROGRAM, COMMIT, VIEW);
    let issued = Cursor::issue(&here, 40);
    let text = issued.to_string();
    assert!(text.starts_with("v2.9558f72e.e56621f0.40."), "{text}");
    assert_eq!(text.len(), "v2.9558f72e.e56621f0.40.".len() + 8);
    let parsed = Cursor::parse(&text)?;
    assert_eq!(parsed, issued);
    assert_eq!(parsed.offset(), 40);
    assert_eq!(parsed.resume(&here, 41), Ok(40));
    Ok(())
}

#[test]
fn a_cursor_refuses_another_program_commit_or_result() -> Result {
    let issued = Cursor::parse(&Cursor::issue(&binding(PROGRAM, COMMIT, VIEW), 3).to_string())?;
    let program = PROGRAM.replacen('9', "8", 1);
    let commit = COMMIT.replacen('e', "f", 1);
    let view = VIEW.replacen('0', "1", 1);
    let refused = |b: Binding<'_>| issued.resume(&b, 10);
    assert_eq!(
        refused(binding(&program, COMMIT, VIEW)),
        Err(CursorError::ForeignProgram)
    );
    assert_eq!(
        refused(binding(PROGRAM, &commit, VIEW)),
        Err(CursorError::OtherCommit)
    );
    // A new commit is also a new program (the pin is in the source): the
    // commit is reported, because that is what changed.
    assert_eq!(
        refused(binding(&program, &commit, VIEW)),
        Err(CursorError::OtherCommit)
    );
    assert_eq!(
        refused(binding(PROGRAM, COMMIT, &view)),
        Err(CursorError::Tampered)
    );
    Ok(())
}

#[test]
fn an_edited_offset_or_an_offset_past_the_end_refuses() -> Result {
    let here = binding(PROGRAM, COMMIT, VIEW);
    let text = Cursor::issue(&here, 3).to_string();
    let edited = Cursor::parse(&text.replacen(".3.", ".4.", 1))?;
    assert_eq!(edited.resume(&here, 10), Err(CursorError::Tampered));
    for rows in [0, 3] {
        assert_eq!(
            Cursor::issue(&here, 3).resume(&here, rows),
            Err(CursorError::OutOfRange)
        );
    }
    Ok(())
}

#[test]
fn malformed_and_older_cursors_refuse_with_a_type() -> Result {
    let text = Cursor::issue(&binding(PROGRAM, COMMIT, VIEW), 3).to_string();
    let trailing = format!("{text}.");
    let leading = format!(" {text}");
    for bad in [
        "",
        "zz",
        "v2",
        "v2.9558f72e.e56621f0.3",
        "v2.9558F72E.e56621f0.3.00000000",
        "v2.9558f72e.e56621f0.03.00000000",
        "v2.9558f72e.e56621f0.+3.00000000",
        "v2.9558f72e.e56621f0.-1.00000000",
        "v2.9558f72e5.e56621f0.3.00000000",
        "x2.9558f72e.e56621f0.3.00000000",
        &trailing,
        &leading,
    ] {
        assert_eq!(Cursor::parse(bad), Err(CursorError::Malformed), "{bad:?}");
    }
    for older in ["0123456789abcdef.12", "v1.9558f72e.e56621f0.3.00000000"] {
        assert_eq!(
            Cursor::parse(older),
            Err(CursorError::UnsupportedVersion),
            "{older}"
        );
    }
    Ok(())
}
