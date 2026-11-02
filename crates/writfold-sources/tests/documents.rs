use writfold_sources::documents::{
    parse_document, parse_document_required, render_document, unfenced_lines,
};
use writfold_sources::frontmatter::{
    parse_frontmatter, read_frontmatter, split_commas_respecting_brackets, split_list_value,
};

#[test]
fn edits_preserve_unknown_material_crlf_and_history() -> Result<(), Box<dyn std::error::Error>> {
    let raw = "---\r\nid: x\r\nstatus : proposed  \r\ncustom: opaque # comment\r\n---\r\n\r\n# Title\r\n\r\n## Context\r\n\r\nkeep  spaces\r\n\r\n## Unknown\r\n```markdown\r\n## Context\r\n```\r\n⟦cite rev=old blob=original span=hash L=1-2 seen=then⟧\r\n";
    let document = parse_document(raw)?;
    assert_eq!(document.newline(), "\r\n");
    assert_eq!(
        document.sections().map(|(key, _)| key).collect::<Vec<_>>(),
        ["Context", "Unknown"]
    );
    let updated = document.update_fields(&[("status", Some("accepted"))])?;
    assert_eq!(
        updated.text(),
        raw.replace("status : proposed", "status : accepted")
    );
    assert_eq!(document.as_bytes(), raw.as_bytes());
    let replaced = document.update_section("Context", "new\nline")?;
    assert_eq!(replaced.text(), raw.replace("keep  spaces", "new\r\nline"));
    Ok(())
}

#[test]
fn exact_utf8_spans_and_ordered_extensions() -> Result<(), Box<dyn std::error::Error>> {
    let raw = "---\nid: λ\nsecond : \u{1f}三  \n---\n# Café\n## Δ ###\n\n雪\n\n## Unknown\nbytes\n";
    let doc = parse_document(raw)?;
    let field_span = doc.field_span("second").ok_or("field span missing")?;
    assert_eq!(&raw[field_span], "三");
    let section_span = doc.section_span("Δ").ok_or("section span missing")?;
    assert_eq!(&raw[section_span], "\n雪\n\n");
    let changed = doc.update_fields(&[
        ("z_extension", Some("last")),
        ("id", None),
        ("a_extension", Some("later")),
    ])?;
    assert_eq!(
        changed.text(),
        raw.replace("id: λ", "id: ")
            .replace("---\n#", "z_extension: last\na_extension: later\n---\n#")
    );
    assert_eq!(
        changed
            .frontmatter()
            .map(|(key, _)| key)
            .collect::<Vec<_>>(),
        ["id", "second", "z_extension", "a_extension"]
    );
    assert!(
        doc.update_fields(&[("new", None), ("new", Some("ambiguous"))])
            .is_err()
    );
    Ok(())
}

#[test]
fn append_keeps_every_authored_byte_and_blank_section_boundaries()
-> Result<(), Box<dyn std::error::Error>> {
    let raw = "---\r\nid: x\r\n---\r\n# T\r\n## Decision log\r\n\r\n- old  text\r\n\r\n## Unknown\r\n\r\nleave alone\r\n";
    let document = parse_document(raw)?;
    assert_eq!(
        document.append_section("Decision log", "- new")?.text(),
        raw.replace("- old  text", "- old  text\r\n- new")
    );
    let empty = parse_document("---\n---\n## A\n\n## B\n")?;
    assert_eq!(
        empty.update_section("A", " x ")?.text(),
        "---\n---\n## A\n\nx\n\n## B\n"
    );
    assert_eq!(
        empty.append_section("A", " x ")?.text(),
        "---\n---\n## A\n\nx\n## B\n"
    );
    for tail in ["", "\n", "\n\n"] {
        let doc = parse_document(format!("---\n---{tail}"))?;
        assert_eq!(
            doc.update_section("Added", " x ")?.text(),
            "---\n---\n\n## Added\n\nx\n"
        );
    }
    Ok(())
}

#[test]
fn fences_width_indent_and_heading_grammar() -> Result<(), Box<dyn std::error::Error>> {
    let raw = "---\n---\n## Context\n````md\n## Fake\n```\n## Also fake\n````\n~~~\n## Hidden\n~~~\n## Actual ###  \nreal\n";
    let doc = parse_document(raw)?;
    assert_eq!(
        doc.sections().map(|(key, _)| key).collect::<Vec<_>>(),
        ["Context", "Actual"]
    );
    assert!(!doc.unfenced_lines().any(|line| line == "## Fake"));
    assert_eq!(unfenced_lines("```bad`info\n## Visible\n").count(), 2);
    assert_eq!(unfenced_lines("    ```\n## Visible\n").count(), 2);
    assert_eq!(
        unfenced_lines("   ~~~info\n## Hidden\n  ~~~~ \u{1f}\n## Visible\n")
            .map(|(_, line)| line)
            .collect::<Vec<_>>(),
        ["## Visible\n"]
    );
    assert_eq!(parse_document("---\n---\n##  \n")?.section(""), Some(""));
    assert_eq!(parse_document("---\n---\n## \n")?.sections().count(), 0);
    Ok(())
}

#[test]
fn drafts_and_ambiguity_are_separate_from_required_admission()
-> Result<(), Box<dyn std::error::Error>> {
    let raw = "---\ntitle: unfinished\n---\n## Research\n";
    let draft = parse_document(raw)?;
    assert_eq!(draft.field("title"), Some("unfinished"));
    assert!(draft.require(&["id"], &[]).is_err());
    assert!(parse_document_required(raw, &[], &["Decision log"]).is_err());
    assert!(draft.require(&["title"], &["Research"]).is_ok());
    for broken in [
        "",
        "no frontmatter",
        "---\nid: x",
        "---\nid: x\nid: y\n---\n",
        "---\n---\n## Context\na\n## Context\nb\n",
    ] {
        assert!(parse_document(broken).is_err());
    }
    assert!(read_frontmatter("---\nid: x\nid: y\n---\n").is_err());
    assert!(
        draft
            .update_fields(&[("title", Some("line\nbreak"))])
            .is_err()
    );
    assert!(draft.update_section("new\rheading", "body").is_err());
    Ok(())
}

#[test]
fn python_unicode_splitlines_preserve_historical_documents()
-> Result<(), Box<dyn std::error::Error>> {
    for separator in [
        "\r", "\r\n", "\n", "\u{b}", "\u{c}", "\u{1c}", "\u{1d}", "\u{1e}", "\u{85}", "\u{2028}",
        "\u{2029}",
    ] {
        let raw =
            format!("---{separator}x: 三{separator}---{separator}## 名{separator}body{separator}");
        let document = parse_document(raw.clone())?;
        assert_eq!(document.text(), raw);
        assert_eq!(document.field("x"), Some("三"));
        assert_eq!(document.section("名"), Some("body"));
        assert_eq!(parse_frontmatter(&raw)?.get("x"), Some("三"));
    }
    assert!(parse_document("---\u{1f}x: y\u{1f}---").is_err());
    let lone_cr = parse_document("---\rx: y\r---\r## A\rtext\r")?;
    assert_eq!(lone_cr.body(), "## A\rtext\r");
    assert_eq!(lone_cr.unfenced_lines().count(), 0);
    Ok(())
}

#[test]
fn new_fields_and_headings_refuse_line_injection() -> Result<(), Box<dyn std::error::Error>> {
    let document = parse_document("---\nx: old\n---\n")?;
    for separator in [
        "\r", "\r\n", "\n", "\u{b}", "\u{c}", "\u{1c}", "\u{1d}", "\u{1e}", "\u{85}", "\u{2028}",
        "\u{2029}",
    ] {
        let value = format!("a{separator}injected: b");
        assert!(document.update_fields(&[("x", Some(&value))]).is_err());
        assert!(document.update_fields(&[("new", Some(&value))]).is_err());
        assert!(document.update_fields(&[(&value, Some("safe"))]).is_err());
        assert!(document.update_section(&value, "body").is_err());
        assert!(document.append_section(&value, "body").is_err());
        assert!(render_document(&[("x", Some(&value))], &[], "").is_err());
        assert!(render_document(&[(&value, Some("safe"))], &[], "").is_err());
        assert!(render_document(&[], &[(&value, "body")], "").is_err());
        assert!(render_document(&[], &[], &value).is_err());
    }
    let historical = "---\nx: a\u{2028}injected: b\n---\n";
    let retained = parse_document(historical)?;
    assert_eq!(retained.text(), historical);
    assert_eq!(retained.field("injected"), Some("b"));
    assert_eq!(
        retained.update_fields(&[("other", Some("safe"))])?.text(),
        "---\nx: a\u{2028}injected: b\nother: safe\n---\n"
    );
    Ok(())
}

#[test]
fn render_order_and_citation_aware_lists() -> Result<(), Box<dyn std::error::Error>> {
    let document = render_document(
        &[("z", Some("三")), ("a", None)],
        &[("Unknown", " body "), ("Context", "draft")],
        "Title",
    )?;
    assert_eq!(
        document.text(),
        "---\nz: 三\na: \n---\n\n# Title\n\n## Unknown\n\nbody\n\n## Context\n\ndraft\n"
    );
    assert_eq!(parse_frontmatter("ordinary draft")?.entries().count(), 0);
    assert_eq!(
        split_commas_respecting_brackets("x, \"y, z\""),
        ["x", "\"y, z\""]
    );
    assert_eq!(split_commas_respecting_brackets("a \"b, c"), ["a \"b", "c"]);
    assert_eq!(
        split_commas_respecting_brackets("x, ⟦cite y=\"z, w\"⟧"),
        ["x", "⟦cite y=\"z, w\"⟧"]
    );
    assert_eq!(
        split_commas_respecting_brackets("a, ⟦outer ⟦inner,x⟧,y⟧, z"),
        ["a", "⟦outer ⟦inner,x⟧,y⟧", "z"]
    );
    assert_eq!(split_list_value("\u{1c}[a, ,b]\u{1f}"), ["a", "b"]);
    assert!(split_list_value("a,b").is_empty());
    Ok(())
}
