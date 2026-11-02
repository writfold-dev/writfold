use crate::syntax::Audit;
use syn::{
    spanned::Spanned,
    visit::{self, Visit},
};

impl Audit<'_> {
    fn rust_function(
        &mut self,
        signature: &syn::Signature,
        block: &syn::Block,
        attrs: &[syn::Attribute],
    ) {
        let old = (self.test, self.ghost);
        self.test |= is_test(attrs);
        self.ghost = false;
        self.budget(block.span(), 80, 50, "function-budget");
        if signature.unsafety.is_some() {
            self.emit("unsafe", signature.span(), "unsafe function");
        }
        visit::visit_signature(self, signature);
        self.visit_block(block);
        (self.test, self.ghost) = old;
    }
}

pub fn is_test(attrs: &[syn::Attribute]) -> bool {
    attrs.iter().any(|attribute| {
        attribute.path().is_ident("test")
            || (attribute.path().is_ident("cfg")
                && matches!(&attribute.meta,
            syn::Meta::List(list) if list.tokens.to_string() == "test"))
    })
}

impl<'ast> Visit<'ast> for Audit<'_> {
    fn visit_item_fn(&mut self, node: &'ast syn::ItemFn) {
        let old = self.test;
        self.test |= is_test(&node.attrs);
        for attr in &node.attrs {
            self.visit_attribute(attr);
        }
        self.rust_function(&node.sig, &node.block, &node.attrs);
        self.test = old;
    }
    fn visit_impl_item_fn(&mut self, node: &'ast syn::ImplItemFn) {
        for attr in &node.attrs {
            self.visit_attribute(attr);
        }
        self.rust_function(&node.sig, &node.block, &node.attrs);
    }
    fn visit_trait_item_fn(&mut self, node: &'ast syn::TraitItemFn) {
        for attr in &node.attrs {
            self.visit_attribute(attr);
        }
        if let Some(block) = &node.default {
            self.rust_function(&node.sig, block, &node.attrs);
        }
    }
    fn visit_item_mod(&mut self, node: &'ast syn::ItemMod) {
        let old = self.test;
        self.test |= is_test(&node.attrs);
        self.budget(node.span(), 400, 250, "module-budget");
        visit::visit_item_mod(self, node);
        self.test = old;
    }
    fn visit_expr_closure(&mut self, node: &'ast syn::ExprClosure) {
        self.budget(node.body.span(), 80, 50, "function-budget");
        visit::visit_expr_closure(self, node);
    }
    fn visit_expr_method_call(&mut self, node: &'ast syn::ExprMethodCall) {
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
    fn visit_macro(&mut self, node: &'ast syn::Macro) {
        self.inspect_rust_macro(node);
    }
    fn visit_expr_unsafe(&mut self, node: &'ast syn::ExprUnsafe) {
        self.emit("unsafe", node.span(), "unsafe block");
        visit::visit_expr_unsafe(self, node);
    }
    fn visit_item_trait(&mut self, node: &'ast syn::ItemTrait) {
        if node.unsafety.is_some() {
            self.emit("unsafe", node.span(), "unsafe trait");
        }
        visit::visit_item_trait(self, node);
    }
    fn visit_item_foreign_mod(&mut self, node: &'ast syn::ItemForeignMod) {
        self.emit(
            "unsafe",
            node.span(),
            "foreign boundary requires a separately audited crate",
        );
        visit::visit_item_foreign_mod(self, node);
    }
    fn visit_item_extern_crate(&mut self, node: &'ast syn::ItemExternCrate) {
        if node.rename.is_some() && node.ident == "std" {
            self.emit(
                "core-io",
                node.span(),
                "renamed std crate conceals effect paths",
            );
        }
        visit::visit_item_extern_crate(self, node);
    }
    fn visit_item_impl(&mut self, node: &'ast syn::ItemImpl) {
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
    fn visit_attribute(&mut self, node: &'ast syn::Attribute) {
        crate::rust_attributes::check(self, node);
        visit::visit_attribute(self, node);
    }
    fn visit_path(&mut self, node: &'ast syn::Path) {
        crate::rust_safety::path(self, node);
        visit::visit_path(self, node);
    }
    fn visit_item_use(&mut self, node: &'ast syn::ItemUse) {
        crate::rust_safety::imports(self, &node.tree, String::new());
        visit::visit_item_use(self, node);
    }
}
