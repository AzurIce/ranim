pub fn expr_to_u32(expr: &syn::Expr) -> syn::Result<u32> {
    match expr {
        syn::Expr::Lit(syn::ExprLit {
            lit: syn::Lit::Int(i), ..
        }) => i.base10_parse(),
        _ => Err(syn::Error::new_spanned(expr, "expected integer literal")),
    }
}

pub fn expr_to_bool(expr: &syn::Expr) -> syn::Result<bool> {
    match expr {
        syn::Expr::Lit(syn::ExprLit {
            lit: syn::Lit::Bool(b), ..
        }) => Ok(b.value),
        _ => Err(syn::Error::new_spanned(expr, "expected bool literal")),
    }
}