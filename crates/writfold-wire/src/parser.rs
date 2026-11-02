use crate::{Budget, Error, Limits};
use serde_json::{Map, Number, Value};
use std::str::FromStr;

pub(crate) fn parse(bytes: &[u8], limits: Limits) -> Result<Value, Error> {
    if bytes.len() > limits.bytes {
        return Err(Error::Limit {
            budget: Budget::Bytes,
            offset: limits.bytes,
        });
    }
    let text = std::str::from_utf8(bytes).map_err(|error| Error::Unicode {
        offset: error.valid_up_to(),
    })?;
    let mut parser = Parser {
        text,
        offset: 0,
        nodes: 0,
        limits,
        frames: Vec::new(),
        root: None,
    };
    parser.value()?;
    while parser.root.is_none() {
        parser.step()?;
    }
    parser.space();
    if parser.offset != bytes.len() {
        return Err(Error::TrailingInput {
            offset: parser.offset,
        });
    }
    parser
        .root
        .take()
        .ok_or_else(|| parser.syntax("expected a JSON value"))
}

/// Destruction must be iterative too, including completed children in partial
/// containers on every parse-error path. Values are moved, never deep-cloned.
pub(crate) fn drop_value(value: Value) {
    let mut pending = vec![value];
    while let Some(value) = pending.pop() {
        match value {
            Value::Array(values) => pending.extend(values),
            Value::Object(values) => pending.extend(values.into_iter().map(|(_, value)| value)),
            _ => {}
        }
    }
}

#[derive(Clone, Copy)]
enum State {
    First,
    Next,
    Comma,
}

struct Frame {
    values: Value,
    state: State,
    key: Option<String>,
}

impl Frame {
    fn close(&self) -> u8 {
        if self.values.is_array() { b']' } else { b'}' }
    }
    fn len(&self) -> usize {
        match &self.values {
            Value::Array(values) => values.len(),
            Value::Object(values) => values.len(),
            _ => 0,
        }
    }
}

struct Parser<'a> {
    text: &'a str,
    offset: usize,
    nodes: usize,
    limits: Limits,
    frames: Vec<Frame>,
    root: Option<Value>,
}

impl Drop for Parser<'_> {
    fn drop(&mut self) {
        if let Some(value) = self.root.take() {
            drop_value(value);
        }
        for frame in self.frames.drain(..) {
            drop_value(frame.values);
        }
    }
}

impl Parser<'_> {
    fn peek(&self) -> Option<u8> {
        self.text.as_bytes().get(self.offset).copied()
    }

    fn space(&mut self) {
        while matches!(self.peek(), Some(b' ' | b'\n' | b'\r' | b'\t')) {
            self.offset += 1;
        }
    }

    fn syntax(&self, reason: &str) -> Error {
        Error::Syntax {
            offset: self.offset,
            reason: reason.to_owned(),
        }
    }

    fn limit(&self, budget: Budget) -> Error {
        Error::Limit {
            budget,
            offset: self.offset,
        }
    }

    fn value(&mut self) -> Result<(), Error> {
        self.space();
        if self.nodes >= self.limits.nodes {
            return Err(self.limit(Budget::Nodes));
        }
        self.nodes += 1;
        let value = match self.peek() {
            Some(b'{' | b'[') => return self.open(),
            Some(b'"') => self.string().map(Value::String),
            Some(b'-' | b'0'..=b'9') => self.number(),
            Some(b't') => self.literal("true", Value::Bool(true)),
            Some(b'f') => self.literal("false", Value::Bool(false)),
            Some(b'n') => self.literal("null", Value::Null),
            Some(b'N' | b'I') => Err(Error::UnsupportedNumber {
                offset: self.offset,
            }),
            _ => Err(self.syntax("expected a JSON value")),
        }?;
        self.complete(value)
    }

    fn complete(&mut self, value: Value) -> Result<(), Error> {
        match self.frames.last_mut() {
            Some(Frame {
                values: Value::Array(values),
                ..
            }) => values.push(value),
            Some(Frame {
                values: Value::Object(values),
                key,
                ..
            }) => {
                if let Some(key) = key.take() {
                    values.insert(key, value);
                } else {
                    drop_value(value);
                    return Err(self.syntax("missing object key"));
                }
            }
            Some(_) => {
                drop_value(value);
                return Err(self.syntax("invalid container"));
            }
            None => self.root = Some(value),
        }
        Ok(())
    }

    fn open(&mut self) -> Result<(), Error> {
        if self.frames.len() >= self.limits.depth {
            return Err(self.limit(Budget::Depth));
        }
        let values = if self.peek() == Some(b'[') {
            Value::Array(Vec::new())
        } else {
            Value::Object(Map::new())
        };
        self.offset += 1;
        self.frames.push(Frame {
            values,
            state: State::First,
            key: None,
        });
        Ok(())
    }

    fn step(&mut self) -> Result<(), Error> {
        self.space();
        let frame = self
            .frames
            .last()
            .ok_or_else(|| self.syntax("expected container"))?;
        let close = frame.close();
        match frame.state {
            State::Comma => match self.peek() {
                Some(byte) if byte == close => return self.close(),
                Some(b',') => {
                    self.offset += 1;
                    if let Some(frame) = self.frames.last_mut() {
                        frame.state = State::Next;
                    }
                    return Ok(());
                }
                _ => return Err(self.syntax("expected comma or container end")),
            },
            State::First if self.peek() == Some(close) => return self.close(),
            _ => {}
        }
        if frame.len() >= self.limits.container_entries {
            return Err(self.limit(Budget::ContainerEntries));
        }
        if close == b'}' {
            self.key()?;
        }
        if let Some(frame) = self.frames.last_mut() {
            frame.state = State::Comma;
        }
        self.value()
    }

    fn close(&mut self) -> Result<(), Error> {
        self.offset += 1;
        let frame = self
            .frames
            .pop()
            .ok_or_else(|| self.syntax("unexpected container end"))?;
        self.complete(frame.values)
    }

    fn key(&mut self) -> Result<(), Error> {
        if self.peek() != Some(b'"') {
            return Err(self.syntax("expected object key"));
        }
        let offset = self.offset;
        let key = self.string()?;
        if self.frames.last().is_some_and(|frame| {
            frame
                .values
                .as_object()
                .is_some_and(|values| values.contains_key(&key))
        }) {
            return Err(Error::DuplicateKey { key, offset });
        }
        self.space();
        if self.peek() != Some(b':') {
            return Err(self.syntax("expected colon"));
        }
        self.offset += 1;
        if let Some(frame) = self.frames.last_mut() {
            frame.key = Some(key);
        }
        Ok(())
    }

    fn literal(&mut self, token: &str, value: Value) -> Result<Value, Error> {
        if !self.text[self.offset..].starts_with(token) {
            return Err(self.syntax("invalid literal"));
        }
        self.offset += token.len();
        Ok(value)
    }

    fn string(&mut self) -> Result<String, Error> {
        let start = self.offset;
        self.offset += 1;
        let mut escaped = false;
        while let Some(byte) = self.peek() {
            self.offset += 1;
            if byte == b'"' && !escaped {
                return serde_json::from_str(&self.text[start..self.offset]).map_err(|error| {
                    Error::Syntax {
                        offset: start,
                        reason: error.to_string(),
                    }
                });
            }
            escaped = byte == b'\\' && !escaped;
        }
        Err(self.syntax("unterminated string"))
    }

    fn number(&mut self) -> Result<Value, Error> {
        let start = self.offset;
        while let Some(byte) = self.peek() {
            if matches!(byte, b',' | b']' | b'}' | b' ' | b'\n' | b'\r' | b'\t') {
                break;
            }
            self.offset += 1;
        }
        let token = &self.text[start..self.offset];
        // Python parses -0 as integer zero, and emits 0 on canonicalization.
        let normalized = if token == "-0" { "0" } else { token };
        Number::from_str(normalized)
            .map(Value::Number)
            .map_err(|error| Error::Syntax {
                offset: start,
                reason: error.to_string(),
            })
    }
}
