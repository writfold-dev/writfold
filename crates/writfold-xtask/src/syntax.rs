use proc_macro2::Span;
use serde_json::{Value, json};
use std::{
    collections::BTreeSet,
    path::{Component, Path},
};
use verus_syn::{
    spanned::Spanned,
    visit::{self, Visit},
};

pub struct Audit<'a> {
    pub path: &'a str,
    pub package: &'a str,
    pub file: &'a Path,
    pub package_root: &'a Path,
    pub owned_sources: &'a BTreeSet<(&'a str, &'a Path)>,
    pub lines: &'a BTreeSet<usize>,
    pub diagnostics: &'a mut Vec<Value>,
    pub definitions: &'a BTreeSet<String>,
    pub test: bool,
    pub ghost: bool,
}

impl Audit<'_> {
    pub fn emit(&mut self, rule: &str, span: Span, message: impl Into<String>) {
        self.diagnostics.push(json!({"rule":rule,"path":self.path,
            "line":span.start().line,"column":span.start().column + 1,
            "end_line":span.end().line,"end_column":span.end().column + 1,
            "message":message.into(),"severity":"error"}));
    }

    pub fn budget(&mut self, span: Span, limit: usize, warning: usize, rule: &str) {
        let count = self
            .lines
            .range(span.start().line..=span.end().line)
            .count();
        if count > warning {
            self.diagnostics.push(json!({"rule":rule,"path":self.path,
                "line":span.start().line,"lines":count,"limit":limit,
                "severity":if count > limit {"error"} else {"warning"}}));
        }
    }

    pub fn module_path(&mut self, span: Span, value: Option<&str>) {
        let inspected = value.is_some_and(|value| {
            let Some(parent) = self.file.parent() else {
                return false;
            };
            let mut resolved = parent.to_path_buf();
            for component in Path::new(value).components() {
                match component {
                    Component::Normal(name) => resolved.push(name),
                    Component::CurDir => {}
                    Component::ParentDir => {
                        if !resolved.pop() || !resolved.starts_with(self.package_root) {
                            return false;
                        }
                    }
                    Component::RootDir | Component::Prefix(_) => return false,
                }
            }
            self.owned_sources
                .contains(&(self.package, resolved.as_path()))
        });
        if !inspected {
            self.emit(
                "opaque-module",
                span,
                "module path must name inspected source in its owning package",
            );
        }
    }

    fn function(
        &mut self,
        signature: &verus_syn::Signature,
        block: &verus_syn::Block,
        attrs: &[verus_syn::Attribute],
    ) {
        let old = (self.test, self.ghost);
        self.test |= is_test(attrs);
        self.ghost = !matches!(
            signature.mode,
            verus_syn::FnMode::Exec(_) | verus_syn::FnMode::Default
        );
        self.budget(block.span(), 80, 50, "function-budget");
        if signature.unsafety.is_some() {
            self.emit("unsafe", signature.span(), "unsafe function");
        }
        visit::visit_signature(self, signature);
        self.visit_block(block);
        (self.test, self.ghost) = old;
    }
}

pub fn is_test(attrs: &[verus_syn::Attribute]) -> bool {
    attrs.iter().any(|attribute| {
        attribute.path().is_ident("test")
            || (attribute.path().is_ident("cfg")
                && matches!(&attribute.meta,
            verus_syn::Meta::List(list) if list.tokens.to_string() == "test"))
    })
}

impl<'ast> Visit<'ast> for Audit<'_> {
    fn visit_ensures(&mut self, node: &'ast verus_syn::Ensures) {
        let old = self.ghost;
        self.ghost = true;
        visit::visit_ensures(self, node);
        self.ghost = old;
    }
    fn visit_requires(&mut self, node: &'ast verus_syn::Requires) {
        let old = self.ghost;
        self.ghost = true;
        visit::visit_requires(self, node);
        self.ghost = old;
    }
    fn visit_loop_spec(&mut self, node: &'ast verus_syn::LoopSpec) {
        let old = self.ghost;
        self.ghost = true;
        visit::visit_loop_spec(self, node);
        self.ghost = old;
    }
    fn visit_assert(&mut self, node: &'ast verus_syn::Assert) {
        let old = self.ghost;
        self.ghost = true;
        visit::visit_assert(self, node);
        self.ghost = old;
    }
    fn visit_assert_forall(&mut self, node: &'ast verus_syn::AssertForall) {
        let old = self.ghost;
        self.ghost = true;
        visit::visit_assert_forall(self, node);
        self.ghost = old;
    }
    fn visit_assume(&mut self, node: &'ast verus_syn::Assume) {
        let old = self.ghost;
        self.ghost = true;
        visit::visit_assume(self, node);
        self.ghost = old;
    }
    fn visit_item_fn(&mut self, node: &'ast verus_syn::ItemFn) {
        let old = self.test;
        self.test |= is_test(&node.attrs);
        for attr in &node.attrs {
            self.visit_attribute(attr);
        }
        self.function(&node.sig, &node.block, &node.attrs);
        self.test = old;
    }
    fn visit_impl_item_fn(&mut self, node: &'ast verus_syn::ImplItemFn) {
        for attr in &node.attrs {
            self.visit_attribute(attr);
        }
        self.function(&node.sig, &node.block, &node.attrs);
    }
    fn visit_trait_item_fn(&mut self, node: &'ast verus_syn::TraitItemFn) {
        for attr in &node.attrs {
            self.visit_attribute(attr);
        }
        if let Some(block) = &node.default {
            self.function(&node.sig, block, &node.attrs);
        }
    }
    fn visit_item_mod(&mut self, node: &'ast verus_syn::ItemMod) {
        let old = self.test;
        self.test |= is_test(&node.attrs);
        self.budget(node.span(), 400, 250, "module-budget");
        visit::visit_item_mod(self, node);
        self.test = old;
    }
    fn visit_expr_closure(&mut self, node: &'ast verus_syn::ExprClosure) {
        self.budget(node.body.span(), 80, 50, "function-budget");
        visit::visit_expr_closure(self, node);
    }
    fn visit_expr_unary(&mut self, node: &'ast verus_syn::ExprUnary) {
        let old = self.ghost;
        self.ghost |= matches!(node.op, verus_syn::UnOp::Proof(_));
        visit::visit_expr_unary(self, node);
        self.ghost = old;
    }
    fn visit_expr_method_call(&mut self, node: &'ast verus_syn::ExprMethodCall) {
        if !self.test
            && !self.ghost
            && matches!(
                node.method.to_string().as_str(),
                "unwrap" | "expect" | "unwrap_unchecked"
            )
        {
            self.emit("production-panic", node.span(), node.method.to_string());
        }
        visit::visit_expr_method_call(self, node);
    }
    fn visit_macro(&mut self, node: &'ast verus_syn::Macro) {
        self.inspect_macro(node);
    }
    fn visit_expr_unsafe(&mut self, node: &'ast verus_syn::ExprUnsafe) {
        self.emit("unsafe", node.span(), "unsafe block");
        visit::visit_expr_unsafe(self, node);
    }
    fn visit_item_trait(&mut self, node: &'ast verus_syn::ItemTrait) {
        if node.unsafety.is_some() {
            self.emit("unsafe", node.span(), "unsafe trait");
        }
        visit::visit_item_trait(self, node);
    }
    fn visit_item_foreign_mod(&mut self, node: &'ast verus_syn::ItemForeignMod) {
        self.emit(
            "unsafe",
            node.span(),
            "foreign boundary requires a separately audited crate",
        );
        visit::visit_item_foreign_mod(self, node);
    }
    fn visit_item_extern_crate(&mut self, node: &'ast verus_syn::ItemExternCrate) {
        if node.rename.is_some() && node.ident == "std" {
            self.emit(
                "core-io",
                node.span(),
                "renamed std crate conceals effect paths",
            );
        }
        visit::visit_item_extern_crate(self, node);
    }
    fn visit_item_impl(&mut self, node: &'ast verus_syn::ItemImpl) {
        if node.unsafety.is_some() {
            self.emit("unsafe", node.span(), "unsafe implementation");
        }
        if self.package == "writfold-domain"
            && let Some((_, path, _)) = &node.trait_
            && path
                .segments
                .last()
                .is_some_and(|segment| segment.ident == "Deserialize")
        {
            self.emit(
                "domain-deserialize",
                node.span(),
                "domain deserialization belongs at the checked wire boundary",
            );
        }
        visit::visit_item_impl(self, node);
    }
    fn visit_attribute(&mut self, node: &'ast verus_syn::Attribute) {
        crate::attributes::check(self, node);
        visit::visit_attribute(self, node);
    }
    fn visit_path(&mut self, node: &'ast verus_syn::Path) {
        crate::safety::path(self, node);
        visit::visit_path(self, node);
    }
    fn visit_item_use(&mut self, node: &'ast verus_syn::ItemUse) {
        crate::safety::imports(self, &node.tree, String::new());
        visit::visit_item_use(self, node);
    }
}
