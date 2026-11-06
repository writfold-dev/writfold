use crate::{Budget, Error, RawJson};
use serde_json::Value;

mod number;

/// Python json.dumps(sort_keys=True, separators=(",", ":"),
/// ensure_ascii=False, allow_nan=False), including Python binary64 float spelling.
/// `max_bytes` bounds the encoded output independently of the input budget.
pub fn canonical(raw: &RawJson, max_bytes: usize) -> Result<Vec<u8>, Error> {
    canonical_prefixed(raw, b"", max_bytes)
}

pub(crate) fn canonical_prefixed(
    raw: &RawJson,
    prefix: &[u8],
    max_bytes: usize,
) -> Result<Vec<u8>, Error> {
    let mut writer = Writer {
        bytes: Vec::new(),
        max_bytes,
    };
    writer.append(prefix)?;
    writer.value(raw.value())?;
    Ok(writer.bytes)
}

pub(crate) struct Writer {
    pub(crate) bytes: Vec<u8>,
    pub(crate) max_bytes: usize,
}

enum Work<'a> {
    Value(&'a Value),
    Array(std::slice::Iter<'a, Value>, bool),
    Object(std::vec::IntoIter<(&'a String, &'a Value)>, bool),
}

impl Writer {
    pub(crate) fn append(&mut self, bytes: &[u8]) -> Result<(), Error> {
        if bytes.len() > self.max_bytes.saturating_sub(self.bytes.len()) {
            return Err(Error::Limit {
                budget: Budget::Bytes,
                offset: self.bytes.len(),
            });
        }
        self.bytes.extend_from_slice(bytes);
        Ok(())
    }

    pub(crate) fn string(&mut self, text: &str) -> Result<(), Error> {
        self.append(b"\"")?;
        let mut start = 0;
        for (offset, byte) in text.bytes().enumerate() {
            let escape: Option<&[u8]> = match byte {
                b'"' => Some(b"\\\""),
                b'\\' => Some(b"\\\\"),
                b'\x08' => Some(b"\\b"),
                b'\x0c' => Some(b"\\f"),
                b'\n' => Some(b"\\n"),
                b'\r' => Some(b"\\r"),
                b'\t' => Some(b"\\t"),
                _ => None,
            };
            if escape.is_some() || byte < 0x20 {
                self.append(&text.as_bytes()[start..offset])?;
                if let Some(escape) = escape {
                    self.append(escape)?;
                } else {
                    const HEX: &[u8; 16] = b"0123456789abcdef";
                    self.append(&[
                        b'\\',
                        b'u',
                        b'0',
                        b'0',
                        HEX[(byte >> 4) as usize],
                        HEX[(byte & 15) as usize],
                    ])?;
                }
                start = offset + 1;
            }
        }
        self.append(&text.as_bytes()[start..])?;
        self.append(b"\"")
    }

    fn value(&mut self, value: &Value) -> Result<(), Error> {
        let mut pending = vec![Work::Value(value)];
        while let Some(work) = pending.pop() {
            match work {
                Work::Value(value) => match value {
                    Value::Null => self.append(b"null")?,
                    Value::Bool(true) => self.append(b"true")?,
                    Value::Bool(false) => self.append(b"false")?,
                    Value::Number(number) => {
                        let token = number.as_str();
                        if token.contains(['.', 'e', 'E']) {
                            let encoded = number::python_float(token, self.bytes.len())?;
                            self.append(encoded.as_bytes())?;
                        } else {
                            self.append(token.as_bytes())?;
                        }
                    }
                    Value::String(text) => self.string(text)?,
                    Value::Array(values) => {
                        self.append(b"[")?;
                        pending.push(Work::Array(values.iter(), true));
                    }
                    Value::Object(values) => {
                        self.append(b"{")?;
                        // Remain deterministic under preserve_order feature unification.
                        let mut entries: Vec<_> = values.iter().collect();
                        entries.sort_unstable_by(|left, right| left.0.cmp(right.0));
                        pending.push(Work::Object(entries.into_iter(), true));
                    }
                },
                Work::Array(mut values, first) => {
                    if let Some(value) = values.next() {
                        if !first {
                            self.append(b",")?;
                        }
                        pending.push(Work::Array(values, false));
                        pending.push(Work::Value(value));
                    } else {
                        self.append(b"]")?;
                    }
                }
                Work::Object(mut values, first) => {
                    if let Some((key, value)) = values.next() {
                        if !first {
                            self.append(b",")?;
                        }
                        self.string(key)?;
                        self.append(b":")?;
                        pending.push(Work::Object(values, false));
                        pending.push(Work::Value(value));
                    } else {
                        self.append(b"}")?;
                    }
                }
            }
        }
        Ok(())
    }
}
