//! Typed access to a file's entries that reports what is wrong. There is
//! no catch-all: an absent key, a value of the wrong type and a number
//! out of range are three faults with three codes, and a required key is
//! read through its own function, so absence is never a default by
//! accident. Every accessor pushes its fault onto the sink and returns
//! nothing, so a later check that needs the value does not run and no
//! fault is reported twice. A [`Slot::Broken`] entry is silent.
use super::toml::{self, Doc, Slot, Spanned, Text, Value};
use super::{Sink, why};
use writfold_core::diag::{Diagnostic, Report, Span};

/// What reading one key found.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Field<T> {
    Absent,
    /// Broken, or of the wrong kind; already reported.
    Bad,
    Got(T),
}

impl<T> Field<T> {
    pub fn got(self) -> Option<T> {
        match self {
            Field::Got(v) => Some(v),
            _ => None,
        }
    }

    /// The value, or `default` when absent; `None` when bad.
    pub fn or(self, default: T) -> Option<T> {
        match self {
            Field::Got(v) => Some(v),
            Field::Absent => Some(default),
            Field::Bad => None,
        }
    }
}

/// One file being checked: its entries and its sink.
pub struct Check<'t> {
    pub doc: Doc,
    pub sink: Sink<'t>,
    /// `a gate definition`, `a guard rule`, `a plan template`.
    pub what: &'static str,
}

impl<'t> Check<'t> {
    /// Read `text` (shown as `file`), reporting syntax faults.
    pub fn read(file: &str, text: &'t str, what: &'static str) -> Self {
        let mut sink = Sink::new(file, text);
        let doc = toml::read(text, &mut sink);
        Check { doc, sink, what }
    }

    /// The report, or `Ok` when nothing is an error.
    pub fn finish<T>(self, value: Option<T>) -> Result<T, Report> {
        match (self.sink.errors(), value) {
            (0, Some(value)) => Ok(value),
            _ => Err(self.sink.report()),
        }
    }

    /// The compact form of every error so far (for callers that keep a
    /// one-line error).
    pub fn why(report: &Report) -> String {
        why(report)
    }

    pub fn error(
        &mut self,
        code: &'static str,
        span: Option<Span>,
        message: String,
    ) -> &mut Diagnostic {
        self.sink.error(code, span, message)
    }

    fn slot(&self, key: &str) -> Field<&Spanned<Value>> {
        match self.doc.get(key).map(|e| &e.value) {
            None => Field::Absent,
            Some(Slot::Broken) => Field::Bad,
            Some(Slot::Value(v)) => Field::Got(v),
        }
    }

    pub fn key_span(&self, key: &str) -> Option<Span> {
        self.doc.get(key).map(|e| e.key.span)
    }

    fn wrong(
        &mut self,
        key: &str,
        v: &Spanned<Value>,
        want: &str,
        label: String,
    ) -> &mut Diagnostic {
        let why = format!("`{key}` must be {want}, and this is {}", v.value.kind());
        let d = self.sink.error("key.type", Some(v.span), why);
        d.label = label;
        d
    }

    /// `key` is absent and required: reported unless a skipped line may
    /// hold it, or an unknown key was said to mean it.
    pub fn missing(&mut self, key: &str, needs: &str, add: &str) {
        if self.sink.recovered || self.sink.suggested.iter().any(|k| k == key) {
            return;
        }
        let d = self
            .sink
            .error("key.missing", None, format!("{needs} needs `{key}`"));
        d.help(format!("add a line: {add}"));
    }

    pub fn text(&mut self, key: &str) -> Field<Spanned<Text>> {
        match self.slot(key) {
            Field::Absent => Field::Absent,
            Field::Bad => Field::Bad,
            Field::Got(v) => match &v.value {
                Value::Str(t) => Field::Got(Spanned {
                    value: t.clone(),
                    span: v.span,
                }),
                _ => {
                    let v = v.clone();
                    self.wrong(key, &v, "a string", "expected a string".into());
                    Field::Bad
                }
            },
        }
    }

    pub fn need_text(&mut self, key: &str, add: &str) -> Option<Spanned<Text>> {
        let what = self.what;
        self.need_text_for(key, what, add)
    }

    pub fn need_text_for(&mut self, key: &str, needs: &str, add: &str) -> Option<Spanned<Text>> {
        match self.text(key) {
            Field::Absent => {
                self.missing(key, needs, add);
                None
            }
            other => other.got(),
        }
    }

    pub fn int(&mut self, key: &str, (low, high): (i64, i64), typical: i64) -> Field<Spanned<i64>> {
        let v = match self.slot(key) {
            Field::Got(v) => v.clone(),
            Field::Absent => return Field::Absent,
            Field::Bad => return Field::Bad,
        };
        let label = format!("expected an integer from {low} to {high}");
        match &v.value {
            Value::Int(n) if (low..=high).contains(n) => Field::Got(Spanned {
                value: *n,
                span: v.span,
            }),
            Value::Int(n) => {
                let why = format!("`{key}` must be from {low} to {high}, and this is {n}");
                self.sink
                    .error("key.range", Some(v.span), why)
                    .label("out of range");
                Field::Bad
            }
            Value::Str(t) => {
                let help = match t.text.trim().parse::<i64>() {
                    Ok(n) => format!("write it without quotes: {key} = {n}"),
                    Err(_) => format!("write a number without quotes: {key} = {typical}"),
                };
                self.wrong(key, &v, "an integer", label).help(help);
                Field::Bad
            }
            Value::List(_) => {
                self.wrong(key, &v, "an integer", label);
                Field::Bad
            }
        }
    }

    pub fn need_int(
        &mut self,
        key: &str,
        range: (i64, i64),
        typical: i64,
        add: &str,
    ) -> Option<Spanned<i64>> {
        match self.int(key, range, typical) {
            Field::Absent => {
                let what = self.what;
                self.missing(key, what, add);
                None
            }
            other => other.got(),
        }
    }

    pub fn list(&mut self, key: &str) -> Field<Spanned<Vec<Spanned<Text>>>> {
        let v = match self.slot(key) {
            Field::Got(v) => v.clone(),
            Field::Absent => return Field::Absent,
            Field::Bad => return Field::Bad,
        };
        match v.value {
            Value::List(items) => Field::Got(Spanned {
                value: items,
                span: v.span,
            }),
            Value::Str(ref t) => {
                let help = format!(
                    "write an array: {key} = [{}]",
                    serde_json::Value::from(t.text.as_str())
                );
                self.wrong(key, &v, "an array of strings", "expected an array".into())
                    .help(help);
                Field::Bad
            }
            Value::Int(_) => {
                self.wrong(key, &v, "an array of strings", "expected an array".into());
                Field::Bad
            }
        }
    }

    /// A required array with at least one item.
    pub fn need_list(&mut self, key: &str, needs: &str, add: &str) -> Option<Vec<Spanned<Text>>> {
        match self.list(key) {
            Field::Absent => {
                self.missing(key, needs, add);
                None
            }
            Field::Bad => None,
            Field::Got(v) if v.value.is_empty() => {
                let why = format!("`{key}` needs at least one item");
                self.sink
                    .error("key.range", Some(v.span), why)
                    .label("empty");
                None
            }
            Field::Got(v) => Some(v.value),
        }
    }
}
