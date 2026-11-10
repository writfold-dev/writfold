//! The default compact view against a real repository: rows are the exact
//! source lines in `path:line: text` form, formats share pages and cursors,
//! and `--out` writes a new file and prints only the trailer.
use super::tests::{Fixture, envelope, fixture, pipeline, tuft};
use crate::trial::Result;
use std::fs;
use tuft::emit::Format;

fn lines_program(f: &Fixture, fields: &str, budget: u64) -> String {
    format!(
        "@h := find text \"let a\" at \"{}\" limit 50;\n@e := retain @h around lines 0;\n\
@v := project @e [{fields}];\nemit @v budget_tokens {budget};\n",
        f.commit
    )
}

#[test]
fn compact_rows_are_the_exact_source_lines() -> Result<()> {
    let f = fixture()?;
    let program = lines_program(&f, "path, line, excerpt(max_chars: 300)", 10_000);
    let output = tuft(&f, &program, None, Format::Compact)?;
    let (rows, trailer) = output.rsplit_once('\n').ok_or("no trailer")?;
    let digest = tuft::digest_hex(program.as_bytes());
    assert_eq!(
        trailer,
        format!(
            "# coverage=complete files=7/7 hits=8 shown=8 omitted=0 cursor=none program={}",
            &digest[..12]
        )
    );
    assert!(rows.starts_with("docs/notes.md:1: let a = café;\nsrc/lib.rs:1: let a = 1;\n"));
    for row in rows.lines() {
        let (path, rest) = row.split_once(':').ok_or("path")?;
        let (line, text) = rest.split_once(": ").ok_or("line")?;
        let file = fs::read_to_string(f.repo.join(path))?;
        let index: usize = line.parse()?;
        assert_eq!(file.lines().nth(index - 1), Some(text), "{row}");
    }
    Ok(())
}

#[test]
fn bindings_and_ranges_print_only_when_projected() -> Result<()> {
    let f = fixture()?;
    let plain = tuft(
        &f,
        &lines_program(&f, "path, line, excerpt(max_chars: 99)", 900),
        None,
        Format::Compact,
    )?;
    assert!(!plain.contains(".."), "{plain}");
    let program = lines_program(
        &f,
        "path, line, range, binding, excerpt(max_chars: 99)",
        900,
    );
    let full = tuft(&f, &program, None, Format::Compact)?;
    let first = full.lines().next().ok_or("no rows")?;
    let fields: Vec<&str> = first.splitn(5, ':').collect();
    assert_eq!(fields.first().copied(), Some("docs/notes.md"));
    assert_eq!(fields.get(2).copied(), Some("0..15"));
    assert_eq!(fields.get(3).map(|b| b.len()), Some(64));
    assert_eq!(fields.get(4).copied(), Some(" let a = café;"));
    Ok(())
}

#[test]
fn compact_and_verbose_pages_share_rows_and_cursors() -> Result<()> {
    let f = fixture()?;
    let program = pipeline(&f, 60);
    let compact = tuft(&f, &program, None, Format::Compact)?;
    let verbose = tuft(&f, &program, None, Format::Verbose)?;
    let cursor = envelope(&compact, "cursor").ok_or("cursor")?;
    assert_ne!(cursor, "none", "{compact}");
    assert_eq!(envelope(&verbose, "cursor"), Some(cursor));
    assert_eq!(envelope(&compact, "shown"), envelope(&verbose, "shown"));
    assert_eq!(envelope(&compact, "omitted"), envelope(&verbose, "omitted"));
    let next = tuft(&f, &program, Some(cursor), Format::Verbose)?;
    assert_eq!(envelope(&next, "from"), envelope(&compact, "shown"));
    Ok(())
}

#[test]
fn out_writes_a_new_file_prints_the_trailer_and_never_overwrites() -> Result<()> {
    let f = fixture()?;
    let program_path = f.root.join("q.tuft");
    let program = lines_program(&f, "path, line, excerpt(max_chars: 300)", 10_000);
    fs::write(&program_path, &program)?;
    let out = f.root.join("answer.txt");
    let args: Vec<String> = [
        "run",
        &program_path.to_string_lossy(),
        "--repo",
        &f.repo.to_string_lossy(),
        "--state",
        &f.state.to_string_lossy(),
        "--out",
        &out.to_string_lossy(),
    ]
    .iter()
    .map(|a| a.to_string())
    .collect();
    let printed = super::cli::main(&args)?;
    let whole = tuft(&f, &program, None, Format::Compact)?;
    let (rows, trailer) = whole.rsplit_once('\n').ok_or("no trailer")?;
    assert_eq!(printed, trailer);
    assert_eq!(fs::read_to_string(&out)?, format!("{rows}\n"));
    fs::write(&out, "kept")?;
    let refused = super::cli::main(&args).err().ok_or("overwrote")?;
    assert!(
        refused.to_string().contains("refusing to overwrite"),
        "{refused}"
    );
    assert_eq!(fs::read_to_string(&out)?, "kept");
    Ok(())
}
