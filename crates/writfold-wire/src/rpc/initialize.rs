//! Strict `schemas.py::Initialize` plus `Engine.request` capability admission.
//! Ordering and duplicate capabilities are retained; this neither advertises
//! method bodies nor fabricates server metadata or a running transport.
use serde_json::Value;

pub const PROTOCOL_VERSION: &str = "2";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Capability {
    Citations,
    Graph,
    Knowledge,
    Authored,
    Notifications,
}

impl Capability {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Citations => "citations",
            Self::Graph => "graph",
            Self::Knowledge => "knowledge",
            Self::Authored => "authored",
            Self::Notifications => "notifications",
        }
    }

    fn from_value(value: &Value) -> Option<Self> {
        match value.as_str()? {
            "citations" => Some(Self::Citations),
            "graph" => Some(Self::Graph),
            "knowledge" => Some(Self::Knowledge),
            "authored" => Some(Self::Authored),
            "notifications" => Some(Self::Notifications),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, thiserror::Error, Eq, PartialEq)]
pub enum ParamsError {
    #[error("params must be an object")]
    ExpectedObject,
    #[error("unknown parameter field")]
    UnknownField,
    #[error("protocol_version must be exactly the string 2")]
    InvalidProtocolVersion,
    #[error("capabilities must be a list of strings")]
    InvalidCapabilities,
    #[error("unsupported requested capability")]
    UnsupportedCapability,
}

impl ParamsError {
    /// Unknown capabilities are an operation-level malformed refusal in the
    /// reference; schema/type/version errors are JSON-RPC Invalid params.
    pub fn code(self) -> i32 {
        match self {
            Self::UnsupportedCapability => -32000,
            _ => -32602,
        }
    }
}

/// Schema-valid initialization parameters, before capability admission.
/// The reference checks already-initialized state between these two stages.
#[derive(Clone, Copy, Debug)]
pub struct InitializeParams<'a> {
    capabilities: &'a [Value],
}

impl<'a> InitializeParams<'a> {
    pub fn parse(params: Option<&'a Value>) -> Result<Self, ParamsError> {
        let object = match params {
            None => return Err(ParamsError::InvalidProtocolVersion),
            Some(value) => value.as_object().ok_or(ParamsError::ExpectedObject)?,
        };
        if object
            .keys()
            .any(|key| !matches!(key.as_str(), "protocol_version" | "capabilities"))
        {
            return Err(ParamsError::UnknownField);
        }
        if object.get("protocol_version").and_then(Value::as_str) != Some(PROTOCOL_VERSION) {
            return Err(ParamsError::InvalidProtocolVersion);
        }
        let capabilities = match object.get("capabilities") {
            None => &[][..],
            Some(Value::Array(values)) => values.as_slice(),
            Some(_) => return Err(ParamsError::InvalidCapabilities),
        };
        // Type validation must finish before checking support: a later non-string
        // is a schema error even when an earlier string names an unknown feature.
        if capabilities.iter().any(|value| !value.is_string()) {
            return Err(ParamsError::InvalidCapabilities);
        }
        Ok(Self { capabilities })
    }

    pub fn admit(self) -> Result<Initialize<'a>, ParamsError> {
        if self
            .capabilities
            .iter()
            .any(|value| Capability::from_value(value).is_none())
        {
            return Err(ParamsError::UnsupportedCapability);
        }
        Ok(Initialize {
            capabilities: self.capabilities,
        })
    }
}

/// Immutable admitted parameters, with no tree clone or second allocation.
/// Ordering and duplicates are retained; absent capabilities mean an empty list.
#[derive(Clone, Copy, Debug)]
pub struct Initialize<'a> {
    capabilities: &'a [Value],
}

impl<'a> Initialize<'a> {
    /// Convenience for stateless consumers. A stateful adapter instead parses
    /// `InitializeParams`, checks double-init, admits capabilities, then commits.
    pub fn parse(params: Option<&'a Value>) -> Result<Self, ParamsError> {
        InitializeParams::parse(params)?.admit()
    }

    pub fn protocol_version(self) -> &'static str {
        PROTOCOL_VERSION
    }

    /// All values were validated at construction and cannot subsequently change.
    /// Iteration preserves exact input order and duplicates without allocation.
    pub fn capabilities(self) -> impl Iterator<Item = Capability> + 'a {
        self.capabilities.iter().filter_map(Capability::from_value)
    }
}
