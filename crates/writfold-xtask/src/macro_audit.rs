use crate::{macros, syntax::Audit, tokens};
use verus_syn::{spanned::Spanned, visit::Visit};

const BUILTIN: &[&str] = &[
    "vec",
    "format",
    "format_args",
    "write",
    "writeln",
    "println",
    "eprintln",
    "print",
    "eprint",
    "matches",
    "stringify",
    "concat",
    "env",
    "option_env",
    "cfg",
    "file",
    "line",
    "column",
    "include_str",
    "include_bytes",
    "compile_error",
    "panic",
    "todo",
    "unimplemented",
    "unreachable",
    "assert",
    "assert_eq",
    "assert_ne",
];
impl Audit<'_> {
    pub fn inspect_macro(&mut self, node: &verus_syn::Macro) {
        let name = node
            .path
            .segments
            .last()
            .map(|segment| segment.ident.to_string())
            .unwrap_or_default();
        if name == "verus" {
            match verus_syn::parse2::<verus_syn::File>(node.tokens.clone()) {
                Ok(file) => self.visit_file(&file),
                Err(error) => self.emit("macro-parse", error.span(), error.to_string()),
            }
        } else if name == "macro_rules" {
            match macros::templates(node.tokens.clone()) {
                Ok(bodies) => {
                    for body in bodies {
                        match verus_syn::parse2::<verus_syn::File>(body.clone()) {
                            Ok(file) => self.visit_file(&file),
                            Err(_) => match verus_syn::parse2::<verus_syn::Expr>(body) {
                                Ok(expr) => self.visit_expr(&expr),
                                Err(error) => {
                                    self.emit("macro-parse", error.span(), error.to_string())
                                }
                            },
                        }
                    }
                }
                Err(error) => self.emit("macro-parse", node.span(), error),
            }
        } else {
            self.inspect_ordinary_macro(node, &name);
        }
    }

    fn inspect_ordinary_macro(&mut self, node: &verus_syn::Macro, name: &str) {
        if name == "seq" {
            // vstd's literal grammar is a comma-separated expression list,
            // not vec!'s repetition grammar. Preserve the caller's mode.
            use verus_syn::parse::Parser;
            let parser =
                verus_syn::punctuated::Punctuated::<verus_syn::Expr, verus_syn::Token![,]>::parse_terminated;
            match parser.parse2(node.tokens.clone()) {
                Ok(expressions) => {
                    for expression in expressions {
                        self.visit_expr(&expression);
                    }
                }
                Err(error) => self.emit("macro-parse", error.span(), error.to_string()),
            }
            return;
        }
        if name == "json" {
            match verus_syn::parse2::<crate::json_macro::Expressions>(node.tokens.clone()) {
                Ok(expressions) => {
                    for expression in expressions.0 {
                        self.visit_expr(&expression);
                    }
                }
                Err(error) => self.emit("macro-parse", error.span(), error.to_string()),
            }
            return;
        }
        if self.package == "writfold-xtask" && matches!(name, "braced" | "bracketed" | "Token") {
            return; // verus_syn's parser-token DSL, not executable declaration input.
        }
        if !self.test
            && matches!(
                self.package,
                "writfold-domain" | "writfold-kernel" | "writfold-protocol"
            )
            && matches!(name, "println" | "eprintln" | "print" | "eprint")
        {
            self.emit(
                "core-io",
                node.span(),
                "stdio macro in pure production crate",
            );
        }
        if !self.test
            && !self.ghost
            && matches!(
                name,
                "panic"
                    | "todo"
                    | "unimplemented"
                    | "unreachable"
                    | "assert"
                    | "assert_eq"
                    | "assert_ne"
            )
        {
            self.emit("production-panic", node.span(), name);
        }
        self.inspect_arguments(node, name);
    }

    fn inspect_arguments(&mut self, node: &verus_syn::Macro, name: &str) {
        if self.definitions.contains(name) {
            // A declaration macro's template is checked independently. Arguments
            // may supply data/types, never undisclosed declarations or verifier
            // attributes (including bare $meta fragments without a # wrapper).
            if tokens::contains_ident(
                node.tokens.clone(),
                &[
                    "fn",
                    "unsafe",
                    "impl",
                    "mod",
                    "loop",
                    "while",
                    "return",
                    "unwrap",
                    "expect",
                    "panic",
                    "todo",
                    "unimplemented",
                ],
            ) || tokens::contains_verifier_path(node.tokens.clone())
            {
                self.emit(
                    "macro-argument",
                    node.span(),
                    "executable or verifier macro arguments require explicit syntax support",
                );
            }
            self.argument_blocks(node.tokens.clone());
            self.declaration_bindings(node.tokens.clone(), node.span(), name);
            return;
        }
        if !BUILTIN.contains(&name) {
            self.emit(
                "opaque-macro",
                node.span(),
                format!("uninspected macro {name}"),
            );
            return;
        }
        if self.special_arguments(node, name) {
            return;
        }
        // Visit parseable expression arguments, including nested blocks/closures.
        let parser = verus_syn::punctuated::Punctuated::<verus_syn::Expr, verus_syn::Token![,]>::parse_terminated;
        use verus_syn::parse::Parser;
        if let Ok(expressions) = parser.parse2(node.tokens.clone()) {
            for expression in expressions {
                self.visit_expr(&expression);
            }
        } else if !node.tokens.is_empty() {
            self.emit("macro-parse", node.span(), "unparsed macro arguments");
        }
    }
    fn special_arguments(&mut self, node: &verus_syn::Macro, name: &str) -> bool {
        match name {
            "stringify" | "cfg" => true, // Token data, never executable input.
            "matches" => {
                match verus_syn::parse2::<crate::matches_macro::Arguments>(node.tokens.clone()) {
                    Ok(args) => {
                        self.visit_expr(&args.value);
                        self.visit_pat(&args.pattern);
                        if let Some(guard) = args.guard {
                            self.visit_expr(&guard);
                        }
                    }
                    Err(error) => self.emit("macro-parse", error.span(), error.to_string()),
                }
                true
            }
            "vec" => {
                let mut group =
                    proc_macro2::Group::new(proc_macro2::Delimiter::Bracket, node.tokens.clone());
                group.set_span(node.span());
                let stream = proc_macro2::TokenStream::from(proc_macro2::TokenTree::Group(group));
                match verus_syn::parse2::<verus_syn::Expr>(stream) {
                    Ok(expression) => self.visit_expr(&expression),
                    Err(error) => self.emit("macro-parse", error.span(), error.to_string()),
                }
                true
            }
            _ => false,
        }
    }

    fn argument_blocks(&mut self, tokens: proc_macro2::TokenStream) {
        let mut tokens = tokens.into_iter().peekable();
        while let Some(token) = tokens.next() {
            if matches!(&token, proc_macro2::TokenTree::Punct(p) if p.as_char() == '#')
                && let Some(proc_macro2::TokenTree::Group(_)) = tokens.peek()
                && let Some(group) = tokens.next()
            {
                use verus_syn::parse::Parser;
                let stream = [token, group].into_iter().collect();
                match verus_syn::Attribute::parse_outer.parse2(stream) {
                    Ok(attributes) => {
                        for attribute in attributes {
                            self.visit_attribute(&attribute);
                        }
                    }
                    Err(error) => self.emit("macro-parse", error.span(), error.to_string()),
                }
                continue;
            }
            if let proc_macro2::TokenTree::Group(group) = token {
                let stream =
                    proc_macro2::TokenStream::from(proc_macro2::TokenTree::Group(group.clone()));
                if let Ok(block) = verus_syn::parse2::<verus_syn::Block>(stream) {
                    self.budget(block.span(), 80, 50, "function-budget");
                    self.visit_block(&block);
                } else {
                    self.argument_blocks(group.stream());
                }
            }
        }
    }

    pub fn declaration_bindings(
        &mut self,
        tokens: proc_macro2::TokenStream,
        span: proc_macro2::Span,
        name: &str,
    ) {
        if self.test || self.ghost || !matches!(name, "record" | "quantity") {
            return;
        }
        let parsed = if name == "record" {
            verus_syn::parse2::<RecordBindings>(tokens).map(|record| {
                record
                    .fields
                    .into_iter()
                    .map(|(field, span)| (field, span, "record-field", record.owner.to_string()))
                    .collect::<Vec<_>>()
            })
        } else {
            verus_syn::parse2::<QuantityBinding>(tokens).map(|quantity| {
                vec![(
                    "clone".to_owned(),
                    span,
                    "quantity-clone",
                    quantity.owner.to_string(),
                )]
            })
        };
        // Unrelated macros may share these names; unsupported invocations get
        // no declaration linkage, while their ordinary syntax audit still runs.
        if let Ok(bindings) = parsed {
            for (name, span, kind, type_name) in bindings {
                self.functions.insert(crate::syntax::FunctionSource {
                    source: self.path.to_owned(),
                    name,
                    lines: [span.start().line, span.end().line],
                    origin: Some(crate::syntax::GeneratedOrigin { kind, type_name }),
                });
            }
        }
    }
}

struct RecordBindings {
    owner: verus_syn::Ident,
    fields: Vec<(String, proc_macro2::Span)>,
}

impl verus_syn::parse::Parse for RecordBindings {
    fn parse(input: verus_syn::parse::ParseStream<'_>) -> verus_syn::Result<Self> {
        let _ = input.call(verus_syn::Attribute::parse_outer)?;
        let owner = input.parse()?;
        input.parse::<verus_syn::Token![,]>()?;
        let _: verus_syn::Ident = input.parse()?;
        let fields;
        verus_syn::braced!(fields in input);
        let mut bindings = Vec::new();
        while !fields.is_empty() {
            let field: verus_syn::Ident = fields.parse()?;
            fields.parse::<verus_syn::Token![:]>()?;
            let _: verus_syn::Type = fields.parse()?;
            if fields.peek(verus_syn::Token![=>]) {
                fields.parse::<verus_syn::Token![=>]>()?;
                let _: verus_syn::Path = fields.parse()?;
            }
            bindings.push((field.to_string(), field.span()));
            if !fields.is_empty() {
                fields.parse::<verus_syn::Token![,]>()?;
            }
        }
        if input.peek(verus_syn::Token![,]) {
            input.parse::<verus_syn::Token![,]>()?;
        }
        Ok(Self {
            owner,
            fields: bindings,
        })
    }
}

struct QuantityBinding {
    owner: verus_syn::Ident,
}

impl verus_syn::parse::Parse for QuantityBinding {
    fn parse(input: verus_syn::parse::ParseStream<'_>) -> verus_syn::Result<Self> {
        let owner = input.parse()?;
        input.parse::<verus_syn::Token![,]>()?;
        let _: verus_syn::Expr = input.parse()?;
        Ok(Self { owner })
    }
}
