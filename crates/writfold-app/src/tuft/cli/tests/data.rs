use serde::Deserialize;

#[derive(Deserialize)]
pub(super) struct Output {
    pub program: String,
    pub commit: String,
    pub at: Option<At>,
    pub rows: Vec<Row>,
    pub find: Find,
    #[serde(default)]
    pub cites: Vec<String>,
}

#[derive(Deserialize)]
pub(super) struct At {
    pub requested: String,
    pub commit: String,
}

#[derive(Deserialize)]
pub(super) struct Row {
    pub path: String,
    pub excerpt: Option<Excerpt>,
}

#[derive(Deserialize)]
pub(super) struct Excerpt {
    pub text: String,
}

#[derive(Deserialize)]
pub(super) struct Find {
    pub hits: usize,
    pub coverage: Coverage,
}

#[derive(Deserialize)]
pub(super) struct Coverage {
    pub state: String,
    pub total_files: usize,
}

#[derive(Deserialize)]
pub(super) struct Check {
    pub ok: bool,
    pub diagnostics: Vec<Diagnostic>,
}

#[derive(Deserialize)]
pub(super) struct Diagnostic {
    pub line: usize,
    pub span: Span,
}

#[derive(Deserialize)]
pub(super) struct Span {
    pub lo: usize,
    pub hi: usize,
}

#[derive(Deserialize)]
pub(super) struct Many {
    pub repositories: Vec<Repository>,
    pub overall: Overall,
}

#[derive(Deserialize)]
pub(super) struct Repository {
    pub repo: String,
    pub result: Output,
}

#[derive(Deserialize)]
pub(super) struct Overall {
    pub coverage: String,
    pub hits: usize,
}
