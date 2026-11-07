use super::{NativeError, Result, equal_text, natural, object, parse, take_commit, text};
use crate::Limits;
use serde_json::Value;
use writfold_domain::{GraphCommitId, RowLimit};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ScalarKind {
    Text,
    Integer,
    Float,
    Boolean,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Column<'a> {
    pub name: &'a str,
    pub kind: ScalarKind,
    pub nullable: bool,
}

/// A complete, ordered projection supplied by an immutable query profile.
/// Construction checks representation only, not native schema authenticity.
#[derive(Clone, Copy, Debug)]
pub struct Projection<'a> {
    columns: &'a [Column<'a>],
}

impl<'a> Projection<'a> {
    pub fn new(columns: &'a [Column<'a>]) -> Result<Self> {
        if columns.is_empty() {
            return Err(NativeError::Shape("empty projection"));
        }
        for (index, column) in columns.iter().enumerate() {
            if column.name.is_empty()
                || columns[..index]
                    .iter()
                    .any(|prior| prior.name == column.name)
            {
                return Err(NativeError::Shape("projection names"));
            }
        }
        Ok(Self { columns })
    }
}

#[derive(Clone, Copy, Debug)]
pub enum QueryTarget<'a> {
    Snapshot(&'a GraphCommitId),
    Branch(&'a str),
}

#[derive(Debug, PartialEq)]
pub enum Cell {
    Text(String),
    Integer(i64),
    Float(f64),
    Boolean(bool),
    Null,
}

/// Fully checked scalar cells in the requested projection's order. Missing
/// declared-nullable cells and explicit JSON null have one semantic value.
/// Native row order is preserved; completeness/order across pages is a caller
/// obligation, not inferred from row_count or a successful decode.
#[derive(Debug)]
pub struct QueryRows {
    commit: GraphCommitId,
    width: usize,
    cells: Vec<Cell>,
}

impl QueryRows {
    pub fn commit(&self) -> &GraphCommitId {
        &self.commit
    }

    pub fn rows(&self) -> std::slice::ChunksExact<'_, Cell> {
        self.cells.chunks_exact(self.width)
    }
}

fn matches_cell(value: Option<&Value>, column: &Column<'_>) -> bool {
    let Some(value) = value.filter(|value| !value.is_null()) else {
        return column.nullable;
    };
    match column.kind {
        ScalarKind::Text => value.is_string(),
        ScalarKind::Integer => value.as_i64().is_some(),
        ScalarKind::Float => value.is_f64() && value.as_f64().is_some_and(f64::is_finite),
        ScalarKind::Boolean => value.is_boolean(),
    }
}

fn target(map: &serde_json::Map<String, Value>, expected: QueryTarget<'_>) -> Result<()> {
    let target = object(&map["target"], &["branch", "snapshot"], &[])?;
    match expected {
        QueryTarget::Snapshot(pin) => {
            equal_text(target, "snapshot", pin.as_str())?;
            equal_text(map, "graph_commit_id", pin.as_str())?;
            if !target["branch"].is_null() {
                return Err(NativeError::Binding("snapshot has branch"));
            }
        }
        QueryTarget::Branch(branch) => {
            equal_text(target, "branch", branch)?;
            if !target["snapshot"].is_null() {
                return Err(NativeError::Binding("branch has snapshot"));
            }
        }
    }
    Ok(())
}

fn cell(value: Option<Value>, kind: ScalarKind) -> Result<Cell> {
    match value {
        None | Some(Value::Null) => Ok(Cell::Null),
        Some(Value::String(value)) if kind == ScalarKind::Text => Ok(Cell::Text(value)),
        Some(Value::Bool(value)) if kind == ScalarKind::Boolean => Ok(Cell::Boolean(value)),
        Some(Value::Number(value)) if kind == ScalarKind::Integer => value
            .as_i64()
            .map(Cell::Integer)
            .ok_or(NativeError::Shape("integer cell")),
        Some(Value::Number(value)) if kind == ScalarKind::Float => value
            .as_f64()
            .map(Cell::Float)
            .ok_or(NativeError::Shape("float cell")),
        _ => Err(NativeError::Shape("scalar cell")),
    }
}

/// Decode only the exact non-ranked stored-query response profile. The complete
/// shallow tree is validated before moving any values out of RawJson, preserving
/// iterative cleanup even for adversarial deep values in late rows or metadata.
pub fn decode_query(
    status: u16,
    bytes: &[u8],
    limits: Limits,
    max_rows: RowLimit,
    expected_query: &str,
    expected_target: QueryTarget<'_>,
    projection: Projection<'_>,
) -> Result<QueryRows> {
    if status != 200 {
        return Err(NativeError::Binding("query status"));
    }
    let mut raw = parse(bytes, limits)?;
    let map = object(
        raw.value(),
        &[
            "query_name",
            "target",
            "row_count",
            "columns",
            "rows",
            "graph_commit_id",
        ],
        &["page", "ranking"],
    )?;
    equal_text(map, "query_name", expected_query)?;
    text(map, "graph_commit_id")?;
    target(map, expected_target)?;
    let columns = map["columns"]
        .as_array()
        .ok_or(NativeError::Shape("columns"))?;
    if columns.len() != projection.columns.len()
        || columns
            .iter()
            .zip(projection.columns)
            .any(|(actual, expected)| actual.as_str() != Some(expected.name))
    {
        return Err(NativeError::Binding("complete ordered projection"));
    }
    let rows = map["rows"].as_array().ok_or(NativeError::Shape("rows"))?;
    let count = u64::try_from(rows.len()).map_err(|_| NativeError::Budget)?;
    if count > max_rows.get() {
        return Err(NativeError::Budget);
    }
    if natural(map, "row_count")? != count {
        return Err(NativeError::Binding("row count"));
    }
    let total = rows
        .len()
        .checked_mul(columns.len())
        .ok_or(NativeError::Budget)?;
    if total > limits.nodes() {
        return Err(NativeError::Budget);
    }
    validate_rows(rows, projection)?;
    let commit = take_commit(&mut raw.value["graph_commit_id"])?;
    let mut cells = Vec::new();
    cells
        .try_reserve_exact(total)
        .map_err(|_| NativeError::Budget)?;
    let Value::Array(rows) = std::mem::take(&mut raw.value["rows"]) else {
        return Err(NativeError::Shape("rows"));
    };
    for row in rows {
        let Value::Object(mut row) = row else {
            return Err(NativeError::Shape("row"));
        };
        for column in projection.columns {
            cells.push(cell(row.remove(column.name), column.kind)?);
        }
    }
    Ok(QueryRows {
        commit,
        width: projection.columns.len(),
        cells,
    })
}

fn validate_rows(rows: &[Value], projection: Projection<'_>) -> Result<()> {
    for row in rows {
        let row = row.as_object().ok_or(NativeError::Shape("row"))?;
        let mut recognized = 0;
        for column in projection.columns {
            let value = row.get(column.name);
            if !matches_cell(value, column) {
                return Err(NativeError::Binding("row fields or scalar types"));
            }
            recognized += usize::from(value.is_some());
        }
        if recognized != row.len() {
            return Err(NativeError::Binding("row fields or scalar types"));
        }
    }
    Ok(())
}
