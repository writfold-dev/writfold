//! Closed parameter schemas for the negotiated `writfold.trial/1` capability.
//!
//! Exactly three methods exist: `trial/show`, `trial/retain` and
//! `trial/prepare`. Approval, apply and recovery are CLI-only: they are not
//! registered here, so a request naming them is Method not found, the same as
//! any other absent method. The operator key decides an approval; an agent
//! session must not approve its own edit.
//!
//! Parameters are strict objects: unknown, missing or mistyped fields refuse.
//! Host paths are explicit absolute parameters. The trial state directory is
//! never a parameter; it comes from server startup.
use super::ParamsError;
use serde_json::{Map, Value};

/// The negotiated capability spelling.
pub const TRIAL_CAPABILITY: &str = "writfold.trial/1";

/// The registered trial methods. Lookup is exact.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TrialMethod {
    Show,
    Retain,
    Prepare,
}

impl TrialMethod {
    pub fn lookup(method: &str) -> Option<Self> {
        match method {
            "trial/show" => Some(Self::Show),
            "trial/retain" => Some(Self::Retain),
            "trial/prepare" => Some(Self::Prepare),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Show => "trial/show",
            Self::Retain => "trial/retain",
            Self::Prepare => "trial/prepare",
        }
    }

    /// Whether the method can write trial state. A failure after dispatch of a
    /// writing method cannot by itself prove that nothing was written.
    pub fn writes(self) -> bool {
        !matches!(self, Self::Show)
    }
}

/// `trial/show {run}`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ShowParams<'a> {
    pub run: &'a str,
}

/// `trial/retain {repo, commit, file, start, end}`: bytes `[start, end)`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RetainParams<'a> {
    pub repo: &'a str,
    pub commit: &'a str,
    pub file: &'a str,
    pub start: u64,
    pub end: u64,
}

/// `trial/prepare {repo, file, recipe, report?}`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PrepareParams<'a> {
    pub repo: &'a str,
    pub file: &'a str,
    pub recipe: &'a str,
    pub report: Option<&'a str>,
}

struct Fields<'a> {
    object: &'a Map<String, Value>,
}

impl<'a> Fields<'a> {
    fn new(
        params: Option<&'a Value>,
        required: &[&str],
        optional: &[&str],
    ) -> Result<Self, ParamsError> {
        let object = params
            .ok_or(ParamsError::ExpectedObject)?
            .as_object()
            .ok_or(ParamsError::ExpectedObject)?;
        if object
            .keys()
            .any(|key| !required.contains(&key.as_str()) && !optional.contains(&key.as_str()))
        {
            return Err(ParamsError::UnknownField);
        }
        if required.iter().any(|key| !object.contains_key(*key)) {
            return Err(ParamsError::MissingField);
        }
        Ok(Self { object })
    }

    fn text(&self, key: &str) -> Result<&'a str, ParamsError> {
        match self.object.get(key) {
            Some(Value::String(value)) if !value.is_empty() && !value.contains('\0') => Ok(value),
            _ => Err(ParamsError::InvalidField),
        }
    }

    /// An absolute host path; relative paths would depend on the server's cwd.
    fn path(&self, key: &str) -> Result<&'a str, ParamsError> {
        let value = self.text(key)?;
        if value.starts_with('/') {
            Ok(value)
        } else {
            Err(ParamsError::InvalidField)
        }
    }

    /// An unsigned 64-bit integer lexical token; fractions/exponents refuse.
    fn integer(&self, key: &str) -> Result<u64, ParamsError> {
        match self.object.get(key) {
            Some(Value::Number(number))
                if number.as_str().bytes().all(|byte| byte.is_ascii_digit()) =>
            {
                number
                    .as_str()
                    .parse()
                    .map_err(|_| ParamsError::InvalidField)
            }
            _ => Err(ParamsError::InvalidField),
        }
    }
}

impl<'a> ShowParams<'a> {
    pub fn parse(params: Option<&'a Value>) -> Result<Self, ParamsError> {
        let fields = Fields::new(params, &["run"], &[])?;
        Ok(Self {
            run: fields.text("run")?,
        })
    }
}

impl<'a> RetainParams<'a> {
    pub fn parse(params: Option<&'a Value>) -> Result<Self, ParamsError> {
        let fields = Fields::new(params, &["repo", "commit", "file", "start", "end"], &[])?;
        Ok(Self {
            repo: fields.path("repo")?,
            commit: fields.text("commit")?,
            file: fields.text("file")?,
            start: fields.integer("start")?,
            end: fields.integer("end")?,
        })
    }
}

impl<'a> PrepareParams<'a> {
    pub fn parse(params: Option<&'a Value>) -> Result<Self, ParamsError> {
        let fields = Fields::new(params, &["repo", "file", "recipe"], &["report"])?;
        let report = match fields.object.get("report") {
            None => None,
            Some(_) => Some(fields.path("report")?),
        };
        Ok(Self {
            repo: fields.path("repo")?,
            file: fields.text("file")?,
            recipe: fields.path("recipe")?,
            report,
        })
    }
}
