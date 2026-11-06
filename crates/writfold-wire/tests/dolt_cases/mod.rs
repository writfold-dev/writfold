use super::*;
#[test]
fn exact_typed_rows_preserve_null_empty_unicode_crlf_and_injection_text() -> Result {
    let a = node("a");
    let mut z = node("z");
    z["home_is_null"] = json!(false);
    let mut unicode = node("é");
    unicode["home_is_null"] = json!(false);
    unicode["home_path"] = json!("docs/雪\r\n.md");
    let decoded = dolt::decode_nodes(&rows(vec![a, z, unicode])?, limits()?, RowLimit::new(3)?)?;
    assert_eq!(decoded.len(), 3);
    for (row, id, home) in [
        (&decoded[0], "a", None),
        (&decoded[1], "z", Some("")),
        (&decoded[2], "é", Some("docs/雪\r\n.md")),
    ] {
        assert_eq!(row.id().as_str(), id);
        assert_eq!(row.kind().as_str(), "document");
        assert_eq!(row.label(), "é\r\n雪'); DROP TABLE wf_nodes; --");
        assert_eq!(row.home_path(), home);
    }
    let decoded = dolt::decode_edges(
        &rows(vec![edge("a"), edge("é")])?,
        limits()?,
        RowLimit::new(2)?,
    )?;
    assert_eq!(decoded.len(), 2);
    for (row, id) in decoded.iter().zip(["a", "é"]) {
        assert_eq!(row.id().as_str(), id);
        assert_eq!(row.source().as_str(), "a");
        assert_eq!(row.target().as_str(), "z");
        assert_eq!(row.kind().as_str(), "related");
    }
    Ok(())
}

#[test]
fn collections_require_ordered_unique_bounded_rows() -> Result {
    for empty in [b"{}".as_slice(), b"{\"rows\":[]}".as_slice()] {
        assert_eq!(
            dolt::decode_nodes(empty, limits()?, RowLimit::new(1)?)?,
            vec![]
        );
        assert_eq!(
            dolt::decode_edges(empty, limits()?, RowLimit::new(1)?)?,
            vec![]
        );
    }
    for ids in [["a", "a"], ["z", "a"], ["é", "z"]] {
        assert_eq!(
            dolt::decode_nodes(&rows(ids.map(node).to_vec())?, limits()?, RowLimit::new(2)?),
            Err(Error::RowOrder { row: 1 })
        );
        assert_eq!(
            dolt::decode_edges(&rows(ids.map(edge).to_vec())?, limits()?, RowLimit::new(2)?),
            Err(Error::RowOrder { row: 1 })
        );
    }
    assert_eq!(
        dolt::decode_nodes(
            &rows(vec![node("a"), node("b")])?,
            limits()?,
            RowLimit::new(1)?
        ),
        Err(Error::RowLimit)
    );
    assert_eq!(
        dolt::decode_edges(
            &rows(vec![edge("a"), edge("b")])?,
            limits()?,
            RowLimit::new(1)?
        ),
        Err(Error::RowLimit)
    );
    Ok(())
}

#[test]
fn native_rows_require_boolean_null_flags_and_nominal_fields() -> Result {
    for flag in [
        json!(0),
        json!(1),
        json!(1.0),
        json!("false"),
        Value::Null,
        json!([]),
    ] {
        let mut row = node("a");
        row["home_is_null"] = flag;
        assert_eq!(
            dolt::decode_nodes(&rows(vec![row])?, limits()?, RowLimit::new(1)?),
            Err(Error::InvalidField {
                field: "home_is_null"
            })
        );
    }
    let mut row = node("a");
    row["home_path"] = json!("not empty");
    assert_eq!(
        dolt::decode_nodes(&rows(vec![row])?, limits()?, RowLimit::new(1)?),
        Err(Error::NullPayload)
    );
    for field in ["id", "kind"] {
        let mut row = node("a");
        row[field] = json!("");
        assert_eq!(
            dolt::decode_nodes(&rows(vec![row])?, limits()?, RowLimit::new(1)?),
            Err(Error::InvalidIdentity { field })
        );
    }
    for field in ["id", "source", "target", "kind"] {
        let mut row = edge("a");
        row[field] = json!("");
        assert_eq!(
            dolt::decode_edges(&rows(vec![row])?, limits()?, RowLimit::new(1)?),
            Err(Error::InvalidIdentity { field })
        );
    }
    for field in ["label", "home_path"] {
        let mut row = node("a");
        row[field] = json!({"unsupported":[[]]});
        assert_eq!(
            dolt::decode_nodes(&rows(vec![row])?, limits()?, RowLimit::new(1)?),
            Err(Error::InvalidField { field })
        );
    }
    for bytes in [
        b"[]".as_slice(),
        b"{\"rows\":{}}",
        b"{\"rows\":[],\"extra\":0}",
        b"{\"rows\":[[]]}",
    ] {
        assert_eq!(
            dolt::decode_nodes(bytes, limits()?, RowLimit::new(1)?),
            Err(Error::InvalidShape)
        );
        assert_eq!(
            dolt::decode_edges(bytes, limits()?, RowLimit::new(1)?),
            Err(Error::InvalidShape)
        );
    }
    Ok(())
}
