//! Closed parameter schemas for the negotiated `writfold.trial/1` capability.
//!
//! Exactly six methods exist: `trial/show`, `trial/retain`, `trial/prepare`,
//! `trial/run` (evidence to a prepared edit, stopping before approval), and the
//! read-only `trial/evidence-list` and `trial/evidence-show`. Approval, apply
//! and recovery are CLI-only: they are not
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
    Run,
    EvidenceList,
    EvidenceShow,
}

const NAMES: [(TrialMethod, &str); 6] = [
    (TrialMethod::Show, "trial/show"),
    (TrialMethod::Retain, "trial/retain"),
    (TrialMethod::Prepare, "trial/prepare"),
    (TrialMethod::Run, "trial/run"),
    (TrialMethod::EvidenceList, "trial/evidence-list"),
    (TrialMethod::EvidenceShow, "trial/evidence-show"),
];

impl TrialMethod {
    pub fn lookup(method: &str) -> Option<Self> {
        NAMES
            .iter()
            .find(|(_, name)| *name == method)
            .map(|(m, _)| *m)
    }

    pub fn as_str(self) -> &'static str {
        NAMES
            .iter()
            .find(|(m, _)| *m == self)
            .map_or("", |(_, name)| name)
    }

    /// Whether the method can write trial state. A failure after dispatch of a
    /// writing method cannot by itself prove that nothing was written.
    pub fn writes(self) -> bool {
        !matches!(self, Self::Show | Self::EvidenceList | Self::EvidenceShow)
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

/// `trial/run {repo, file, recipe, tuft?, weft?, weft_bin?}`: `weft` and
/// `weft_bin` are given together or not at all.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RunParams<'a> {
    pub repo: &'a str,
    pub file: &'a str,
    pub recipe: &'a str,
    pub tuft: Option<&'a str>,
    /// The Weft review source and the Weft binary to run it with.
    pub weft: Option<(&'a str, &'a str)>,
}

/// `trial/evidence-list {path?, commit?, limit?}`: prefix filters and a page
/// size; the CLI's `--out` has no RPC form.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EvidenceListParams<'a> {
    pub path: Option<&'a str>,
    pub commit: Option<&'a str>,
    pub limit: Option<u64>,
}

/// `trial/evidence-show {binding, context_lines?, repo?}`: `binding` is a full
/// digest, a unique 8+ hex prefix or a local label.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EvidenceShowParams<'a> {
    pub binding: &'a str,
    pub context_lines: Option<u64>,
    pub repo: Option<&'a str>,
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

    /// An optional field: absent is `None`, present must be valid.
    fn optional<T>(
        &self,
        key: &str,
        read: impl Fn(&Self, &str) -> Result<T, ParamsError>,
    ) -> Result<Option<T>, ParamsError> {
        match self.object.get(key) {
            None => Ok(None),
            Some(_) => read(self, key).map(Some),
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
        Ok(Self {
            repo: fields.path("repo")?,
            file: fields.text("file")?,
            recipe: fields.path("recipe")?,
            report: fields.optional("report", Fields::path)?,
        })
    }
}

impl<'a> RunParams<'a> {
    pub fn parse(params: Option<&'a Value>) -> Result<Self, ParamsError> {
        let optional = ["tuft", "weft", "weft_bin"];
        let fields = Fields::new(params, &["repo", "file", "recipe"], &optional)?;
        let weft = match (
            fields.optional("weft", Fields::path)?,
            fields.optional("weft_bin", Fields::path)?,
        ) {
            (Some(review), Some(bin)) => Some((review, bin)),
            (None, None) => None,
            _ => return Err(ParamsError::MissingField),
        };
        Ok(Self {
            repo: fields.path("repo")?,
            file: fields.text("file")?,
            recipe: fields.path("recipe")?,
            tuft: fields.optional("tuft", Fields::path)?,
            weft,
        })
    }
}

impl<'a> EvidenceListParams<'a> {
    pub fn parse(params: Option<&'a Value>) -> Result<Self, ParamsError> {
        let fields = Fields::new(params, &[], &["path", "commit", "limit"])?;
        Ok(Self {
            path: fields.optional("path", Fields::text)?,
            commit: fields.optional("commit", Fields::text)?,
            limit: fields.optional("limit", Fields::integer)?,
        })
    }
}

impl<'a> EvidenceShowParams<'a> {
    pub fn parse(params: Option<&'a Value>) -> Result<Self, ParamsError> {
        let fields = Fields::new(params, &["binding"], &["context_lines", "repo"])?;
        Ok(Self {
            binding: fields.text("binding")?,
            context_lines: fields.optional("context_lines", Fields::integer)?,
            repo: fields.optional("repo", Fields::path)?,
        })
    }
}
