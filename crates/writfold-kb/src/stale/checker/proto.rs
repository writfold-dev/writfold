//! The `writfold-checker/1` wire format: one JSON object per line. Writfold
//! writes a `describe` request, then one `check` (or `make`) request per
//! token, then closes stdin; the checker answers `describe` first, then one
//! reply per request id, in any order. Parsing here is strict and pure:
//! anything not exactly as specified is reported, never guessed at.
use serde_json::{Map, Value, json};
use writfold_core::verdict::{Described, Said};

/// The protocol version Writfold speaks.
pub const VERSION: u64 = 1;
/// Its name, as the documentation and `checker list` give it.
pub const NAME: &str = "writfold-checker/1";

/// The first request of every exchange.
pub fn describe() -> String {
    json!({"v": VERSION, "op": "describe"}).to_string()
}

/// One request about a token (`check`) or for one (`make`), with the token
/// fields as written (`check`) or the caller's key=value pairs (`make`).
pub fn request(op: &str, id: &str, kind: &str, fields: &Value) -> String {
    json!({"v": VERSION, "op": op, "id": id, "kind": kind, "fields": fields}).to_string()
}

/// A `describe` reply.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Description {
    pub protocol: bool,
    pub name: String,
    pub version: String,
    pub kinds: Vec<String>,
    pub ops: Vec<String>,
    /// The token format versions it reads; absent means `[1]`.
    pub token_versions: Vec<u64>,
}

impl Description {
    /// The facts the handshake kernel decides on.
    pub fn described(&self, kind: &str, op: &str) -> Described {
        Described {
            protocol: self.protocol,
            lists_kind: self.kinds.iter().any(|k| k == kind),
            has_op: self.ops.iter().any(|o| o == op),
        }
    }
}

fn object(line: &str) -> Option<Map<String, Value>> {
    match serde_json::from_str(line) {
        Ok(Value::Object(map)) => Some(map),
        _ => None,
    }
}

fn words(map: &Map<String, Value>, key: &str) -> Vec<String> {
    map.get(key)
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default()
}

fn text(map: &Map<String, Value>, key: &str) -> Option<String> {
    map.get(key).and_then(Value::as_str).map(str::to_owned)
}

/// Read the `describe` reply; `None` when the line is not a JSON object.
pub fn description(line: &str) -> Option<Description> {
    let map = object(line)?;
    let versions = match map.get("token_versions") {
        None => vec![1],
        Some(list) => list
            .as_array()
            .map(|l| l.iter().filter_map(Value::as_u64).collect())
            .unwrap_or_default(),
    };
    Some(Description {
        protocol: map.get("v").and_then(Value::as_u64) == Some(VERSION),
        name: text(&map, "name").unwrap_or_default(),
        version: text(&map, "version").unwrap_or_default(),
        kinds: words(&map, "kinds"),
        ops: words(&map, "ops"),
        token_versions: versions,
    })
}

/// Where a `moved` reply says the cited content sits now.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Moved {
    pub path: Option<String>,
    pub first: u32,
    pub last: u32,
}

/// One reply to a request id.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Reply {
    pub id: String,
    /// The status, or why it cannot be read.
    pub said: Result<Said, String>,
    pub at: Option<String>,
    pub moved: Option<Moved>,
    pub detail: Option<String>,
    /// A `make` reply's fields.
    pub fields: Option<Map<String, Value>>,
}

/// Read one reply line. `Err` when the line belongs to no request (not a
/// JSON object, another protocol version, no string `id`): the whole
/// exchange is then unusable.
pub fn reply(line: &str) -> Result<Reply, String> {
    let map = object(line).ok_or("a line that is not a JSON object")?;
    if map.get("v").and_then(Value::as_u64) != Some(VERSION) {
        return Err(format!("a reply without \"v\":{VERSION}"));
    }
    let id = text(&map, "id").ok_or("a reply without a string id")?;
    let moved = moved(&map);
    let said = match map.get("status").and_then(Value::as_str) {
        Some("fresh") => Ok(Said::Fresh),
        Some("moved") if moved.is_some() => Ok(Said::Moved),
        Some("moved") => Err("moved without a valid range".to_owned()),
        Some("changed") => Ok(Said::Changed),
        Some("gone") => Ok(Said::Gone),
        Some("unverifiable") => Ok(Said::Unverifiable),
        Some(other) => Err(format!("unknown status {other:?}")),
        None => Err("no status".to_owned()),
    };
    let fields = map.get("fields").and_then(Value::as_object).cloned();
    Ok(Reply {
        id,
        said,
        at: text(&map, "at"),
        moved,
        detail: text(&map, "detail"),
        fields,
    })
}

fn moved(map: &Map<String, Value>) -> Option<Moved> {
    let moved = map.get("moved")?.as_object()?;
    let number = |key: &str| {
        moved
            .get(key)
            .and_then(Value::as_u64)
            .and_then(|n| u32::try_from(n).ok())
    };
    let (first, last) = (number("first")?, number("last")?);
    if first == 0 || last < first {
        return None;
    }
    Some(Moved {
        path: text(moved, "path"),
        first,
        last,
    })
}

/// A token's format version: `v=` (absent means 1); `None` when unreadable.
pub fn token_version(fields: &Value) -> Option<u64> {
    match fields.get("v").and_then(Value::as_str) {
        None => Some(1),
        Some(v) => v.parse().ok(),
    }
}

#[cfg(test)]
mod tests {
    use super::{description, reply, token_version};
    use serde_json::json;
    use writfold_core::verdict::Said;

    #[test]
    fn replies_are_read_strictly() {
        let ok =
            reply(r#"{"v":1,"id":"0","status":"moved","moved":{"path":"b","first":3,"last":9}}"#);
        assert!(ok.is_ok_and(|r| r.said == Ok(Said::Moved) && r.moved.is_some()));
        let bad_range = reply(r#"{"v":1,"id":"0","status":"moved","moved":{"first":9,"last":3}}"#);
        assert!(bad_range.is_ok_and(|r| r.said.is_err()));
        let unknown = reply(r#"{"v":1,"id":"0","status":"stale"}"#);
        assert!(unknown.is_ok_and(|r| r.said == Err("unknown status \"stale\"".into())));
        assert!(reply("garbage").is_err());
        assert!(reply(r#"{"v":2,"id":"0","status":"fresh"}"#).is_err());
        assert!(reply(r#"{"v":1,"id":0,"status":"fresh"}"#).is_err());
        let d = description(r#"{"v":1,"kinds":["src"],"ops":["check"]}"#);
        assert!(d.is_some_and(|d| d.protocol && d.token_versions == [1]));
        assert!(description(r#"{"v":2}"#).is_some_and(|d| !d.protocol));
        assert_eq!(token_version(&json!({})), Some(1));
        assert_eq!(token_version(&json!({"v": "2"})), Some(2));
        assert_eq!(token_version(&json!({"v": "x"})), None);
    }
}
