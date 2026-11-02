use writfold_sources::citations::*;

#[test]
fn ordered_fields_preserve_extensions_quotes_and_exact_slices()
-> Result<(), Box<dyn std::error::Error>> {
    let body = " kind=query q=\"service=foo | count()\" result=\"三 rows\" x_extension=✓ seen=old";
    let fields = parse_cite_body(body)?;
    assert_eq!(
        fields.iter().collect::<Vec<_>>(),
        [
            ("kind", "query"),
            ("q", "service=foo | count()"),
            ("result", "三 rows"),
            ("x_extension", "✓"),
            ("seen", "old")
        ]
    );
    assert_eq!(
        &body[fields.span("result").ok_or("missing span")?],
        "result=\"三 rows\""
    );
    assert_eq!(
        parse_cite_body("q=\"kind=web kind=code\"")?.get("q"),
        Some("kind=web kind=code")
    );
    assert!(parse_cite_body("kind=web kind=code").is_err());
    assert!(parse_cite_body("名=one 名=\"two\"").is_err());
    assert_eq!(
        parse_cite_body("junk a=\"unterminated x=2")?
            .iter()
            .collect::<Vec<_>>(),
        [("a", "\"unterminated"), ("x", "2")]
    );
    assert_eq!(
        parse_cite_body("x=\"\";y=ok")?.iter().collect::<Vec<_>>(),
        [("x", ""), ("y", "ok")]
    );
    assert_eq!(parse_cite_body("a=⟧ junk !x=ok")?.get("x"), Some("ok"));
    Ok(())
}

#[test]
fn scanner_retains_unterminated_adjacent_and_multiline_tokens()
-> Result<(), Box<dyn std::error::Error>> {
    let text = "Café\r\n⟦cite broken\u{2028}\r⟦cite kind=web⟧⟦cite\nq=\"三\"⟧\n⟦cite tail";
    let tokens = iter_cite_tokens(text).collect::<Vec<_>>();
    assert_eq!(tokens.len(), 4);
    assert_eq!(
        tokens.iter().map(CiteToken::line_no).collect::<Vec<_>>(),
        [2, 2, 2, 4]
    );
    assert_eq!(
        tokens.iter().map(CiteToken::complete).collect::<Vec<_>>(),
        [false, true, true, false]
    );
    assert_eq!(tokens[0].raw(), "⟦cite broken\u{2028}\r");
    assert_eq!(tokens[2].body(), "\nq=\"三\"");
    for token in tokens {
        assert_eq!(&text[token.span()], token.raw());
        assert_eq!(
            token.line_no(),
            text[..token.span().start]
                .bytes()
                .filter(|&byte| byte == b'\n')
                .count()
                + 1
        );
    }
    let complete =
        find_cite_lines("⟦cite broken ⟦cite x=1⟧⟦cite x=2⟧").collect::<Result<Vec<_>, _>>()?;
    assert_eq!(complete.len(), 2);
    assert_eq!(complete[0].0.body(), " broken ⟦cite x=1");
    assert_eq!(complete[0].1.get("x"), Some("1"));
    assert_eq!(find_cite_lines("⟦cite never closes").count(), 0);
    assert_eq!(
        iter_cite_tokens("⟦cite⟦cite⟧")
            .map(|token| token.complete())
            .collect::<Vec<_>>(),
        [false, true]
    );
    Ok(())
}

#[test]
fn python_unicode15_word_whitespace_and_illustrative_boundaries()
-> Result<(), Box<dyn std::error::Error>> {
    assert!(is_illustrative("\u{1c}illus kind=web"));
    assert!(is_illustrative("illus\u{345}")); // combining marks are not Python \w
    assert!(!is_illustrative("illus²"));
    assert!(!is_illustrative("illus_"));
    assert!(!is_illustrative(" kind=usr quote=\"illus\""));
    assert!(is_illustrative("illus\u{1c89}")); // assigned only after Unicode 15
    assert!(!is_illustrative("illus\u{31350}")); // Unicode 15 CJK extension H
    let fields = parse_cite_body("²=number \u{31350}=new15 a\u{345}b=tail \u{1c89}=not15")?;
    assert_eq!(
        fields.iter().collect::<Vec<_>>(),
        [("²", "number"), ("\u{31350}", "new15"), ("b", "tail")]
    );
    assert_eq!(
        parse_cite_body("a=x\u{1f}b=y")?.iter().collect::<Vec<_>>(),
        [("a", "x"), ("b", "y")]
    );
    assert_eq!(
        format_cite(&[("q", "x\u{1c}y")], &[])?,
        "⟦cite q=\"x\u{1c}y\"⟧"
    );
    assert!(format_cite(&[("a\u{345}", "x")], &[]).is_err());
    Ok(())
}

#[test]
fn replacement_and_removal_preserve_historical_binding() -> Result<(), Box<dyn std::error::Error>> {
    let raw = "⟦cite  kind=code\nq=\"quoted seen=old\" rev=old blob=oldblob span=oldspan L=1-2 seen=2020 x=µ⟧";
    let replaced = replace_field(raw, "x", "two words")?;
    assert_eq!(replaced, raw.replace("x=µ", "x=\"two words\""));
    let marked = replace_field(raw, "check_status", "moved")?;
    let original = parse_cite_body(
        raw.strip_prefix("⟦cite")
            .and_then(|v| v.strip_suffix('⟧'))
            .ok_or("bad fixture")?,
    )?;
    let marked_token = iter_cite_tokens(&marked).next().ok_or("missing token")?;
    let fields = marked_token.fields()?;
    for key in ["rev", "blob", "span", "L", "seen"] {
        assert_eq!(fields.get(key), original.get(key));
    }
    assert_eq!(fields.get("checked_at"), None);
    assert_eq!(replace_field("⟦cite x=last⟧", "x", "new")?, "⟦cite x=new⟧");
    assert_eq!(remove_field("⟦cite x=last⟧", "x")?, "⟦cite⟧");
    assert_eq!(remove_field(raw, "missing")?, raw);
    assert_eq!(remove_field("⟦cite x=1\u{1c}y=2 ⟧", "y")?, "⟦cite x=1 ⟧");
    assert_eq!(
        replace_field("⟦cite x=1 \u{1c}⟧", "y", "")?,
        "⟦cite x=1 y=\"\"⟧"
    );
    for invalid in ["plain", "⟦cite x=1", " ⟦cite x=1⟧", "⟦cite x=1 x=2⟧"] {
        assert!(replace_field(invalid, "x", "new").is_err());
        assert!(remove_field(invalid, "x").is_err());
    }
    // Python fullmatch backtracks through an internal closer; preserve that
    // historical editing grammar, distinct from the occurrence iterator.
    assert_eq!(
        replace_field("⟦cite x=1⟧⟦cite y=2⟧", "y", "3")?,
        "⟦cite x=1⟧⟦cite y=3⟧"
    );
    Ok(())
}

#[test]
fn formatting_order_omissions_and_representation_errors() -> Result<(), Box<dyn std::error::Error>>
{
    assert_eq!(
        format_cite(
            &[("x", "µ"), ("kind", "web"), ("q", "curl -I"), ("empty", "")],
            &["kind", "q", "kind"]
        )?,
        "⟦cite kind=web q=\"curl -I\" x=µ⟧"
    );
    assert_eq!(format_cite(&[], &[])?, "⟦cite ⟧");
    assert_eq!(format_cite(&[("x", "a\"b")], &[])?, "⟦cite x=a\"b⟧");
    for value in ["embedded \" quote", "closing⟧delimiter", "\"start"] {
        assert!(format_cite(&[("q", value)], &[]).is_err());
    }
    assert!(format_cite(&[("x", "a"), ("x", "b")], &[]).is_err());
    Ok(())
}

#[test]
fn unbounded_decimal_ranges_and_boundaries() -> Result<(), Box<dyn std::error::Error>> {
    for (raw, first, last) in [
        ("4-4", "4", "4"),
        ("0001-02", "1", "2"),
        ("١-９", "1", "9"),
        ("𝟙-𝟚", "1", "2"),
    ] {
        let range = parse_range(raw)?;
        assert_eq!(range.first().as_decimal(), first);
        assert_eq!(range.last().as_decimal(), last);
    }
    let huge = "9".repeat(5000);
    let range = parse_range(&format!("1-{huge}"))?;
    assert_eq!(range.first().to_usize(), Some(1));
    assert_eq!(range.last().as_decimal(), huge);
    assert_eq!(range.last().to_usize(), None);
    assert!(parse_range("18446744073709551616-18446744073709551617").is_ok());
    for invalid in [
        "0-1",
        "4-3",
        "1",
        "1-2\n",
        "1-2\u{2028}",
        "²-3",
        "+1-2",
        "-1-2",
        "1-",
        "-2",
        "1-2-3",
        "٠-1",
    ] {
        assert!(parse_range(invalid).is_err(), "{invalid:?}");
    }
    assert!(parse_range(&format!("{huge}-1")).is_err());
    Ok(())
}

#[test]
fn normalized_hash_and_unpinned_warnings_do_not_resolve_sources() {
    assert_eq!(normalize_span(&[]), "e3b0c44298fc1c14");
    assert_eq!(normalize_span(&["abc"]), "ba7816bf8f01cfea");
    assert_eq!(
        normalize_span(&["  x", "", "\u{1c}y\u{1f}"]),
        normalize_span(&["x", "y"])
    );
    assert_ne!(normalize_span(&["x y"]), normalize_span(&["x", "y"]));
    let text = "三@main:a @HEAD:b @master:c @Main:no @mainish:no";
    let warnings = find_unpinned(text).collect::<Vec<_>>();
    assert_eq!(
        warnings.iter().map(|(_, raw)| *raw).collect::<Vec<_>>(),
        ["@main:", "@HEAD:", "@master:"]
    );
    for (offset, raw) in warnings {
        assert_eq!(&text[offset..offset + raw.len()], raw);
    }
}
