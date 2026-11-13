//! Consumer-visible checking, pinning and symbol syntax.
use tuft::ir::{MatcherPlan, SymbolKind};
use tuft::{Code, Span};

type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;
const COMMIT: &str = "0123456789abcdef0123456789abcdef01234567";

fn source(at: &str) -> String {
    format!("@h := find text \"x\" at \"{at}\";\n@v := project @h [path];\nemit @v;\n")
}

#[test]
fn stored_programs_require_pins_but_one_shots_accept_refs() -> Result {
    for reference in ["HEAD", "main", "v1.0", "0123456", "feature/search"] {
        let source = source(reference);
        let errors = tuft::compile(source.as_bytes())
            .err()
            .ok_or("accepted ref")?;
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].code, Code::Pin);
        assert!(errors[0].message.contains("git rev-parse"));
        let plan = tuft::compile_unpinned(source.as_bytes()).map_err(|d| format!("{d:?}"))?;
        assert_eq!(plan.find.commit, reference);
        let pinned = tuft::pin(source.as_bytes(), COMMIT).map_err(|d| format!("{d:?}"))?;
        let plan = tuft::compile(&pinned).map_err(|d| format!("{d:?}"))?;
        assert_eq!(plan.find.commit, COMMIT);
        assert_eq!(plan.program, tuft::digest_hex(&pinned));
    }
    assert!(tuft::compile_unpinned(source("").as_bytes()).is_err());
    Ok(())
}

#[test]
fn pin_replaces_only_the_at_literal_and_keeps_existing_pins_exact() -> Result {
    let prefix = "# at \"HEAD\"; Unicode λ\r\n@h := find text \"at \\\"HEAD\\\"\" in \"HEAD/\" at ";
    let suffix = "; # HEAD\r\n@v := project @h [path]; emit @v;\r\n";
    let source = format!("{prefix}\"HEAD\"{suffix}");
    let pinned = tuft::pin(source.as_bytes(), COMMIT).map_err(|d| format!("{d:?}"))?;
    assert_eq!(pinned, format!("{prefix}\"{COMMIT}\"{suffix}").as_bytes());
    assert_eq!(tuft::pin(&pinned, COMMIT), Ok(pinned.clone()));
    for bad in ["HEAD", "ABCDEF", "", "bad\"; emit @h;"] {
        assert!(tuft::pin(source.as_bytes(), bad).is_err());
    }
    let sha256 = "a".repeat(64);
    assert!(tuft::pin(source.as_bytes(), &sha256).is_ok());
    Ok(())
}

#[test]
fn parsing_reports_independent_statement_errors_in_source_order() -> Result {
    let source = b"@a := fetch \"x\";\n@b := project @a [size];\nemit ;\n";
    let errors = tuft::compile(source).err().ok_or("accepted syntax")?;
    let spans = [Span::new(6, 11), Span::new(35, 39), Span::new(47, 48)];
    assert_eq!(errors.len(), spans.len());
    for (error, span) in errors.iter().zip(spans) {
        assert_eq!(error.code, Code::Syntax);
        assert_eq!(error.span, span);
    }
    Ok(())
}

#[test]
fn lexical_errors_do_not_hide_later_parse_errors_or_split_quoted_semicolons() -> Result {
    let source = b"@a := find text \"bad\\q;still string\" at \"HEAD\";\n@b := fetch \";\";\n@c := find text \"x\" at \"HEAD\" limit 007;\nemit ;";
    let errors = tuft::compile(source).err().ok_or("accepted errors")?;
    assert_eq!(
        errors.iter().map(|d| d.code).collect::<Vec<_>>(),
        [Code::Literal, Code::Syntax, Code::Number, Code::Syntax]
    );
    for pair in errors.windows(2) {
        assert!(pair[0].span < pair[1].span);
    }
    assert!(errors.iter().all(|error| error.span.hi <= source.len()));
    let trailing_escape = b"@h := find text \"x\\";
    let errors = tuft::compile(trailing_escape)
        .err()
        .ok_or("accepted escape")?;
    assert_eq!(errors.len(), 1);
    assert!(errors[0].span.hi <= trailing_escape.len());
    Ok(())
}

#[test]
fn symbols_lower_with_kind_name_scope_and_pin() -> Result {
    for (word, kind) in [("def", SymbolKind::Def), ("refs", SymbolKind::Refs)] {
        let source = format!(
            "@h := find {word} \"Widget::open\" in \"src/**/*.rs\" at \"{COMMIT}\"; @v := project @h [path, line]; emit @v;"
        );
        let plan = tuft::compile(source.as_bytes()).map_err(|d| format!("{d:?}"))?;
        assert!(
            matches!(&plan.find.matcher, MatcherPlan::Symbol {kind: actual, name}
            if *actual == kind && name == "Widget::open")
        );
        assert_eq!(plan.find.matcher.describe(), (word, "Widget::open"));
        assert_eq!(plan.find.scope, "src/**/*.rs");
        let empty = source.replace("Widget::open", "");
        let errors = tuft::compile(empty.as_bytes())
            .err()
            .ok_or("accepted empty name")?;
        assert_eq!(errors[0].code, Code::Needle);
    }
    Ok(())
}

#[test]
fn retained_field_diagnostic_names_both_insertion_and_projection() -> Result {
    let source = source(COMMIT).replace("[path]", "[binding]");
    let errors = tuft::compile(source.as_bytes())
        .err()
        .ok_or("accepted bare binding")?;
    assert_eq!(errors[0].code, Code::Type);
    assert!(errors[0].message.contains("@e := retain @h;"));
    assert!(errors[0].message.contains("@v := project"));
    assert!(errors[0].message.contains("project @e"));
    Ok(())
}

#[test]
fn encoding_and_character_errors_keep_every_byte_span() -> Result {
    let source = b"# NUL in comment \0\n@a := find text \"x\0\" at \"HEAD\";\n$; %; emit ;";
    let errors = tuft::compile(source)
        .err()
        .ok_or("accepted malformed source")?;
    assert_eq!(
        errors.iter().map(|error| error.code).collect::<Vec<_>>(),
        [
            Code::Encoding,
            Code::Encoding,
            Code::Character,
            Code::Character,
            Code::Syntax
        ]
    );
    for error in &errors {
        assert_eq!(error.span.hi, error.span.lo + 1);
        assert!(error.span.hi <= source.len());
    }
    Ok(())
}
