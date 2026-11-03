//! Pure protocol-2 request boundary, from `engine/protocol.py::Engine.request`.
//! This is not a dispatcher, server, method implementation or IO capability.
//! Parse one bounded frame, then borrow its validated request; the retained
//! `RawJson` owns every value and destroys even rejected deep trees iteratively.
//! Missing IDs are notifications only after envelope validation succeeds.

mod initialize;
pub use initialize::{
    Capability, Initialize, InitializeParams, PROTOCOL_VERSION, ParamsError, StorageProfileVersion,
};

use crate::{Error, Limits, RawJson};
use serde_json::Value;

/// Reference transport frame ceiling, including any framing whitespace/newline.
pub const MAX_REQUEST_BYTES: usize = 65_536;

#[derive(Debug, thiserror::Error, Eq, PartialEq)]
pub enum FrameError {
    #[error(transparent)]
    Json(#[from] Error),
    #[error("nonfinite JSON floating-point number")]
    NonfiniteNumber,
}

/// Owns a duplicate-aware UTF-8 JSON frame. Caller limits may be stricter than
/// the reference byte ceiling; depth/entry/node budgets have no hidden defaults.
#[derive(Debug)]
pub struct Frame {
    raw: RawJson,
}

impl Frame {
    pub fn parse(bytes: &[u8], limits: Limits) -> Result<Self, FrameError> {
        let limits = Limits::new(
            limits.bytes().min(MAX_REQUEST_BYTES),
            limits.depth(),
            limits.container_entries(),
            limits.nodes(),
        )?;
        let raw = RawJson::parse(bytes, limits)?;
        // Python decode_frame rejects overflowing floats anywhere, not merely
        // in envelope fields. Integer tokens remain arbitrary precision.
        let mut pending = vec![raw.value()];
        while let Some(value) = pending.pop() {
            match value {
                Value::Array(values) => pending.extend(values),
                Value::Object(values) => pending.extend(values.values()),
                Value::Number(number)
                    if !integer_token(number.as_str())
                        && !number.as_str().parse::<f64>().is_ok_and(f64::is_finite) =>
                {
                    return Err(FrameError::NonfiniteNumber);
                }
                _ => {}
            }
        }
        Ok(Self { raw })
    }

    pub fn request(&self) -> Result<Request<'_>, InvalidRequest<'_>> {
        let invalid = |reason, response_id| InvalidRequest {
            reason,
            response_id,
        };
        let object = self
            .raw
            .value()
            .as_object()
            .ok_or_else(|| invalid(EnvelopeError::ExpectedObject, RequestId::Null))?;
        let id = match object.get("id") {
            None => RequestId::Absent,
            Some(Value::Null) => RequestId::Null,
            Some(Value::String(value)) => RequestId::String(value),
            Some(Value::Number(value)) if integer_token(value.as_str()) => {
                RequestId::Integer(IntegerId {
                    decimal: value.as_str(),
                })
            }
            _ => return Err(invalid(EnvelopeError::InvalidId, RequestId::Null)),
        };
        let response_id = match id {
            RequestId::Absent => RequestId::Null,
            id => id,
        };
        if object
            .keys()
            .any(|key| !matches!(key.as_str(), "jsonrpc" | "id" | "method" | "params"))
        {
            return Err(invalid(EnvelopeError::UnknownField, response_id));
        }
        if object.get("jsonrpc").and_then(Value::as_str) != Some("2.0") {
            return Err(invalid(EnvelopeError::InvalidVersion, response_id));
        }
        let method = object
            .get("method")
            .and_then(Value::as_str)
            .ok_or_else(|| invalid(EnvelopeError::InvalidMethod, response_id))?;
        Ok(Request {
            id,
            method,
            params: object.get("params"),
        })
    }
}

fn integer_token(token: &str) -> bool {
    !token.bytes().any(|byte| matches!(byte, b'.' | b'e' | b'E'))
}

/// An integer JSON token, never narrowed through a machine integer or float.
/// Like Python's decoder, integral-looking float tokens (`1.0`, `1e0`) are
/// not integer IDs. The underlying parser normalizes integer `-0` to `0`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct IntegerId<'a> {
    decimal: &'a str,
}

impl<'a> IntegerId<'a> {
    pub fn as_decimal(self) -> &'a str {
        self.decimal
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RequestId<'a> {
    Absent,
    Null,
    String(&'a str),
    Integer(IntegerId<'a>),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EnvelopeError {
    ExpectedObject,
    InvalidId,
    UnknownField,
    InvalidVersion,
    InvalidMethod,
}

/// Invalid envelopes always receive an Invalid Request response, including
/// those without an ID. A valid ID survives other envelope errors; a bad ID
/// or non-object root yields null, matching the reference's validation order.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InvalidRequest<'a> {
    reason: EnvelopeError,
    response_id: RequestId<'a>,
}

impl<'a> InvalidRequest<'a> {
    pub fn reason(self) -> EnvelopeError {
        self.reason
    }

    pub fn response_id(self) -> RequestId<'a> {
        self.response_id
    }

    pub fn code(self) -> i32 {
        -32600
    }
}

/// A valid envelope, not a known or implemented method. Any string method,
/// including an empty or unknown name, reaches the future adapter's lookup.
/// For notifications that adapter must suppress method/params/state failures.
#[derive(Clone, Copy, Debug)]
pub struct Request<'a> {
    id: RequestId<'a>,
    method: &'a str,
    params: Option<&'a Value>,
}

impl<'a> Request<'a> {
    pub fn id(self) -> RequestId<'a> {
        self.id
    }

    pub fn is_notification(self) -> bool {
        matches!(self.id, RequestId::Absent)
    }

    pub fn method(self) -> &'a str {
        self.method
    }

    /// A borrowed raw boundary value; absent params differ from explicit null.
    /// Method-specific validation must precede passing data to the domain.
    pub fn params(self) -> Option<&'a Value> {
        self.params
    }

    /// Validate the reference's closed empty `Params` schema for shutdown.
    /// Does not invoke shutdown or inspect initialization state.
    pub fn shutdown_params(self) -> Result<(), ParamsError> {
        match self.params {
            None => Ok(()),
            Some(Value::Object(object)) if object.is_empty() => Ok(()),
            Some(Value::Object(_)) => Err(ParamsError::UnknownField),
            Some(_) => Err(ParamsError::ExpectedObject),
        }
    }
}
