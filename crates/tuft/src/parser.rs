//! Recursive-descent parser for the grammar in `docs/tuft/language.md`.
//! It stops at the first syntax error; the checker reports the rest.
use crate::ast::{
    Binding, Direction, Emit, Field, FieldKind, Find, Matcher, Name, Number, Operation, Program,
    Project, SortField, SortKey, Text,
};
use crate::diag::{Code, Diagnostic, Span};
use crate::lexer::{Tok, Token};

pub fn parse(tokens: &[Token]) -> Result<Program, Diagnostic> {
    let mut parser = Parser { tokens, at: 0 };
    let mut bindings = Vec::new();
    loop {
        match &parser.current().tok {
            Tok::Handle(_) => bindings.push(parser.binding()?),
            Tok::Word(w) if w == "emit" => break,
            _ => return Err(parser.unexpected("a binding `@name := ...` or `emit`")),
        }
    }
    let emit = parser.emit()?;
    if parser.current().tok != Tok::Eof {
        return Err(parser.unexpected("end of program: `emit` must be the last statement"));
    }
    Ok(Program { bindings, emit })
}

struct Parser<'a> {
    tokens: &'a [Token],
    at: usize,
}

const EOF: Token = Token {
    tok: Tok::Eof,
    span: Span::new(0, 0),
};

impl Parser<'_> {
    fn current(&self) -> &Token {
        self.tokens.get(self.at).unwrap_or(&EOF)
    }

    fn bump(&mut self) -> Token {
        let token = self.current().clone();
        self.at += 1;
        token
    }

    fn unexpected(&self, wanted: &str) -> Diagnostic {
        let found = self.current();
        Diagnostic::new(
            Code::Syntax,
            found.span,
            format!("expected {wanted}, found {}", found.tok.describe()),
        )
    }

    fn keyword(&mut self, word: &str) -> Result<Span, Diagnostic> {
        match &self.current().tok {
            Tok::Word(w) if w == word => Ok(self.bump().span),
            _ => Err(self.unexpected(&format!("`{word}`"))),
        }
    }

    fn at_keyword(&self, word: &str) -> bool {
        matches!(&self.current().tok, Tok::Word(w) if w == word)
    }

    fn punct(&mut self, tok: Tok) -> Result<Span, Diagnostic> {
        if self.current().tok == tok {
            Ok(self.bump().span)
        } else {
            Err(self.unexpected(&tok.describe()))
        }
    }

    fn handle(&mut self) -> Result<Name, Diagnostic> {
        match self.current().tok.clone() {
            Tok::Handle(text) => Ok(Name {
                text,
                span: self.bump().span,
            }),
            _ => Err(self.unexpected("a binding reference `@name`")),
        }
    }

    fn text(&mut self) -> Result<Text, Diagnostic> {
        match self.current().tok.clone() {
            Tok::Str(value) => Ok(Text {
                value,
                span: self.bump().span,
            }),
            _ => Err(self.unexpected("a string")),
        }
    }

    fn number(&mut self) -> Result<Number, Diagnostic> {
        match self.current().tok {
            Tok::Nat(value) => Ok(Number {
                value,
                span: self.bump().span,
            }),
            _ => Err(self.unexpected("a number")),
        }
    }

    fn optional_number(&mut self, word: &str) -> Result<Option<Number>, Diagnostic> {
        if self.at_keyword(word) {
            self.bump();
            return self.number().map(Some);
        }
        Ok(None)
    }

    fn binding(&mut self) -> Result<Binding, Diagnostic> {
        let name = self.handle()?;
        self.punct(Tok::Bind)?;
        let operation = match &self.current().tok {
            Tok::Word(w) if w == "find" => Operation::Find(self.find()?),
            Tok::Word(w) if w == "retain" => self.retain()?,
            Tok::Word(w) if w == "project" => Operation::Project(self.project()?),
            Tok::Word(w) if w == "rank" => {
                self.bump();
                let source = self.handle()?;
                self.keyword("by")?;
                Operation::Rank {
                    source,
                    by: self.text()?,
                }
            }
            Tok::Word(w) if w == "follow" => {
                self.bump();
                let source = self.handle()?;
                self.keyword("via")?;
                Operation::Follow {
                    source,
                    via: self.text()?,
                }
            }
            _ => return Err(self.unexpected("`find`, `retain`, `project`, `rank` or `follow`")),
        };
        let end = self.punct(Tok::Semi)?;
        Ok(Binding {
            span: name.span.to(end),
            name,
            operation,
        })
    }

    fn find(&mut self) -> Result<Find, Diagnostic> {
        self.keyword("find")?;
        let lo = self.current().span;
        let matcher = match self.current().tok.clone() {
            Tok::Word(w) if w == "text" || w == "similar" => {
                self.bump();
                let needle = self.text()?.value;
                if w == "text" {
                    Matcher::Text(needle)
                } else {
                    Matcher::Similar(needle)
                }
            }
            Tok::Rx(source) => {
                self.bump();
                Matcher::Rx(source)
            }
            _ => return Err(self.unexpected("`text \"...\"`, `rx\"...\"` or `similar \"...\"`")),
        };
        let matcher_span = lo.to(self
            .tokens
            .get(self.at.wrapping_sub(1))
            .map_or(lo, |t| t.span));
        let scope = if self.at_keyword("in") {
            self.bump();
            Some(self.text()?)
        } else {
            None
        };
        self.keyword("at")?;
        let at = self.text()?;
        let limit = self.optional_number("limit")?;
        Ok(Find {
            matcher,
            matcher_span,
            scope,
            at,
            limit,
        })
    }

    fn retain(&mut self) -> Result<Operation, Diagnostic> {
        self.keyword("retain")?;
        let source = self.handle()?;
        let around = if self.at_keyword("around") {
            self.bump();
            self.keyword("lines")?;
            Some(self.number()?)
        } else {
            None
        };
        Ok(Operation::Retain { source, around })
    }

    fn project(&mut self) -> Result<Project, Diagnostic> {
        self.keyword("project")?;
        let source = self.handle()?;
        let fields = self.list(Self::field)?;
        let sort = if self.at_keyword("sort") {
            self.bump();
            self.list(Self::sort_key)?
        } else {
            Vec::new()
        };
        let limit = self.optional_number("limit")?;
        Ok(Project {
            source,
            fields,
            sort,
            limit,
        })
    }

    fn list<T>(
        &mut self,
        item: fn(&mut Self) -> Result<T, Diagnostic>,
    ) -> Result<Vec<T>, Diagnostic> {
        self.punct(Tok::LBrack)?;
        let mut items = vec![item(self)?];
        while self.current().tok == Tok::Comma {
            self.bump();
            items.push(item(self)?);
        }
        self.punct(Tok::RBrack)?;
        Ok(items)
    }

    fn field(&mut self) -> Result<Field, Diagnostic> {
        let start = self.current().span;
        let kind = match &self.current().tok {
            Tok::Word(w) if w == "path" => FieldKind::Path,
            Tok::Word(w) if w == "line" => FieldKind::Line,
            Tok::Word(w) if w == "range" => FieldKind::Range,
            Tok::Word(w) if w == "binding" => FieldKind::Binding,
            Tok::Word(w) if w == "excerpt" => {
                self.bump();
                self.punct(Tok::LParen)?;
                self.keyword("max_chars")?;
                self.punct(Tok::Colon)?;
                let max = self.number()?;
                let end = self.punct(Tok::RParen)?;
                return Ok(Field {
                    kind: FieldKind::Excerpt(max),
                    span: start.to(end),
                });
            }
            _ => return Err(self.unexpected("a field: path, line, range, binding or excerpt(...)")),
        };
        self.bump();
        Ok(Field { kind, span: start })
    }

    fn sort_key(&mut self) -> Result<SortKey, Diagnostic> {
        let start = self.current().span;
        let field = match &self.current().tok {
            Tok::Word(w) if w == "path" => SortField::Path,
            Tok::Word(w) if w == "line" => SortField::Line,
            Tok::Word(w) if w == "range" => SortField::Range,
            _ => return Err(self.unexpected("a sort key: path, line or range")),
        };
        self.bump();
        let mut span = start;
        let direction = if self.at_keyword("desc") {
            span = start.to(self.bump().span);
            Direction::Desc
        } else {
            if self.at_keyword("asc") {
                span = start.to(self.bump().span);
            }
            Direction::Asc
        };
        Ok(SortKey {
            field,
            direction,
            span,
        })
    }

    fn emit(&mut self) -> Result<Emit, Diagnostic> {
        let start = self.keyword("emit")?;
        let source = self.handle()?;
        let budget = self.optional_number("budget_tokens")?;
        let end = self.punct(Tok::Semi)?;
        Ok(Emit {
            source,
            budget,
            span: start.to(end),
        })
    }
}
