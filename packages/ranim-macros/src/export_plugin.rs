use proc_macro2::{Ident, Span, TokenStream};
use quote::quote;
use syn::{
    ItemFn, Meta, ReturnType, Type, ext::IdentExt, parse::Parser, punctuated::Punctuated,
    token::Comma,
};

pub(crate) fn expand(
    args: TokenStream,
    input: TokenStream,
    ranim: TokenStream,
) -> syn::Result<TokenStream> {
    if !args.is_empty() {
        return Err(syn::Error::new_spanned(
            args,
            "#[export_plugin] does not accept arguments",
        ));
    }

    let factory: ItemFn = syn::parse2(input)?;
    validate_factory(&factory)?;
    let name = &factory.sig.ident;
    // Keep the helper's identifier conventional even when the factory uses
    // custom lint allowances, and never shadow the factory in its call below.
    let entry_name = Ident::new(
        if name.unraw() == "__ranim_plugin_entry" {
            "__ranim_plugin_entry_"
        } else {
            "__ranim_plugin_entry"
        },
        Span::mixed_site(),
    );
    let export_cfgs = factory
        .attrs
        .iter()
        .filter_map(|attr| export_cfg(&attr.meta).transpose())
        .collect::<syn::Result<Vec<_>>>()?;

    Ok(quote! {
        #factory

        #(#[#export_cfgs])*
        #[cfg(not(target_family = "wasm"))]
        const _: () = {
            #[allow(deprecated)]
            #[unsafe(export_name = "ranim_plugin")]
            fn #entry_name()
            -> ::core::result::Result<::std::boxed::Box<dyn #ranim::Plugin>, ::std::string::String> {
                #ranim::__private::catch_plugin_panic(|| {
                    ::std::boxed::Box::new(#name()) as ::std::boxed::Box<dyn #ranim::Plugin>
                })
            }
        };
    })
}

fn validate_factory(factory: &ItemFn) -> syn::Result<()> {
    let signature = &factory.sig;
    if let Some(variadic) = &signature.variadic {
        return Err(syn::Error::new_spanned(
            variadic,
            "an exported plugin factory must not be variadic",
        ));
    }
    if !signature.inputs.is_empty() {
        return Err(syn::Error::new_spanned(
            &signature.inputs,
            "an exported plugin factory must not take arguments",
        ));
    }
    if !signature.generics.params.is_empty() || signature.generics.where_clause.is_some() {
        return Err(syn::Error::new_spanned(
            &signature.generics,
            "an exported plugin factory must not have generics or a where clause",
        ));
    }
    if let Some(token) = signature.asyncness {
        return Err(syn::Error::new_spanned(
            token,
            "an exported plugin factory must not be async",
        ));
    }
    if let syn::Safety::Unsafe(token) = signature.safety {
        return Err(syn::Error::new_spanned(
            token,
            "an exported plugin factory must be safe",
        ));
    }
    if let Some(abi) = &signature.abi {
        return Err(syn::Error::new_spanned(
            abi,
            "an exported plugin factory must use the implicit Rust ABI",
        ));
    }
    match &signature.output {
        ReturnType::Default => Err(syn::Error::new_spanned(
            signature,
            "an exported plugin factory must declare a Plugin return type",
        )),
        ReturnType::Type(_, ty) if is_unit(ty) => Err(syn::Error::new_spanned(
            ty,
            "an exported plugin factory must return a Plugin, not ()",
        )),
        _ => Ok(()),
    }
}

fn is_unit(ty: &Type) -> bool {
    match ty {
        Type::Tuple(tuple) => tuple.elems.is_empty(),
        Type::Paren(paren) => is_unit(&paren.elem),
        Type::Group(group) => is_unit(&group.elem),
        _ => false,
    }
}

// The factory and its native entry must disappear together. Only copy cfg
// semantics: applying unrelated cfg_attr attributes (such as inline or doc)
// to the generated const would change their meaning or make them invalid.
fn export_cfg(meta: &Meta) -> syn::Result<Option<Meta>> {
    if meta.path().is_ident("cfg") {
        return Ok(Some(meta.clone()));
    }
    if meta.path().is_ident("cfg_attr") {
        let Meta::List(list) = meta else {
            return Err(syn::Error::new_spanned(meta, "expected cfg_attr(...)"));
        };
        let mut arguments = Punctuated::<Meta, Comma>::parse_terminated
            .parse2(list.tokens.clone())?
            .into_iter();
        let condition = arguments.next().ok_or_else(|| {
            syn::Error::new_spanned(meta, "cfg_attr requires a condition and attributes")
        })?;
        let attributes = arguments
            .filter_map(|meta| export_cfg(&meta).transpose())
            .collect::<syn::Result<Vec<_>>>()?;
        if attributes.is_empty() {
            return Ok(None);
        }
        return Ok(Some(
            syn::parse_quote!(cfg_attr(#condition, #(#attributes),*)),
        ));
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preserves_the_factory_and_calls_its_original_name() {
        let input = quote! {
            /// Build the project with its chosen configuration.
            #[inline]
            #[allow(unused)]
            pub(crate) fn project_plugin() -> ConfiguredPlugin {
                ConfiguredPlugin::new(42)
            }
        };
        let output = expand(quote!(), input.clone(), quote!(::renamed_ranim)).unwrap();
        let file: syn::File = syn::parse2(output).unwrap();
        assert_eq!(file.items.len(), 2);
        let syn::Item::Fn(factory) = &file.items[0] else {
            panic!("the first item must be the original factory");
        };
        assert_eq!(quote!(#factory).to_string(), input.to_string());
        let syn::Item::Const(entry) = &file.items[1] else {
            panic!("the entry must be scoped to an anonymous const");
        };
        assert_eq!(entry.ident, "_");
        let entry = quote!(#entry).to_string();
        assert!(entry.contains("export_name = \"ranim_plugin\""));
        assert!(entry.contains(":: renamed_ranim :: __private :: catch_plugin_panic"));
        assert!(entry.contains("Box :: new (project_plugin ())"));
        assert!(entry.contains("not (target_family = \"wasm\")"));
    }

    #[test]
    fn accepts_opaque_and_const_factories() {
        for input in [
            quote!(
                pub fn plugin() -> impl Plugin {
                    ScenesPlugin
                }
            ),
            quote!(
                const fn project() -> ScenesPlugin {
                    ScenesPlugin
                }
            ),
        ] {
            expand(quote!(), input, quote!(::ranim)).unwrap();
        }
    }

    #[test]
    fn helper_never_shadows_the_factory_or_inherits_its_spelling() {
        for (input, expected_helper) in [
            (
                quote!(
                    fn r#type() -> ScenesPlugin {
                        ScenesPlugin
                    }
                ),
                "__ranim_plugin_entry",
            ),
            (
                quote!(
                    #[allow(non_snake_case)]
                    fn ProjectPlugin() -> ScenesPlugin {
                        ScenesPlugin
                    }
                ),
                "__ranim_plugin_entry",
            ),
            (
                quote!(
                    fn r#__ranim_plugin_entry() -> ScenesPlugin {
                        ScenesPlugin
                    }
                ),
                "__ranim_plugin_entry_",
            ),
        ] {
            let output = expand(quote!(), input, quote!(::ranim)).unwrap();
            let file: syn::File = syn::parse2(output).unwrap();
            let syn::Item::Fn(factory) = &file.items[0] else {
                panic!("the first item must be the original factory");
            };
            let syn::Item::Const(entry) = &file.items[1] else {
                panic!("the native entry must be scoped to an anonymous const");
            };
            let syn::Expr::Block(body) = &*entry.expr else {
                panic!("the anonymous const must contain the native entry");
            };
            let syn::Stmt::Item(syn::Item::Fn(helper)) = &body.block.stmts[0] else {
                panic!("the const's only item must be the native entry");
            };
            assert_eq!(helper.sig.ident, expected_helper);
            assert_ne!(helper.sig.ident, factory.sig.ident.unraw());
        }
    }

    #[test]
    fn propagates_only_conditional_compilation_to_the_native_entry() {
        let input = quote! {
            #[cfg(feature = "project")]
            #[cfg_attr(test, cfg(any()), inline)]
            #[cfg_attr(feature = "nested", cfg_attr(test, cfg(unix), doc = "nested"))]
            #[cfg_attr(test, inline)]
            #[doc = "factory"]
            fn plugin() -> ScenesPlugin { ScenesPlugin }
        };
        let output = expand(quote!(), input, quote!(::ranim)).unwrap();
        let file: syn::File = syn::parse2(output).unwrap();
        let syn::Item::Const(entry) = &file.items[1] else {
            panic!("the native entry must be scoped to an anonymous const");
        };
        let attrs = &entry.attrs;
        assert_eq!(
            quote!(#(#attrs)*).to_string(),
            quote! {
                #[cfg(feature = "project")]
                #[cfg_attr(test, cfg(any()))]
                #[cfg_attr(feature = "nested", cfg_attr(test, cfg(unix))) ]
                #[cfg(not(target_family = "wasm"))]
            }
            .to_string(),
        );
    }

    #[test]
    fn rejects_arguments_to_the_attribute() {
        let error = expand(
            quote!(name = "project"),
            quote!(
                fn plugin() -> ScenesPlugin {
                    ScenesPlugin
                }
            ),
            quote!(::ranim),
        )
        .unwrap_err();
        assert_eq!(
            error.to_string(),
            "#[export_plugin] does not accept arguments"
        );
    }

    #[test]
    fn rejects_incompatible_factory_signatures() {
        for (input, expected) in [
            (
                quote!(
                    fn plugin(value: u32) -> ScenesPlugin {
                        ScenesPlugin
                    }
                ),
                "must not take arguments",
            ),
            (
                quote!(
                    fn plugin<T>() -> ScenesPlugin {
                        ScenesPlugin
                    }
                ),
                "must not have generics or a where clause",
            ),
            (
                quote!(
                    fn plugin() -> ScenesPlugin
                    where
                        ScenesPlugin: Sized,
                    {
                        ScenesPlugin
                    }
                ),
                "must not have generics or a where clause",
            ),
            (
                quote!(
                    async fn plugin() -> ScenesPlugin {
                        ScenesPlugin
                    }
                ),
                "must not be async",
            ),
            (
                quote!(
                    unsafe fn plugin() -> ScenesPlugin {
                        ScenesPlugin
                    }
                ),
                "must be safe",
            ),
            (
                quote!(
                    extern "Rust" fn plugin() -> ScenesPlugin {
                        ScenesPlugin
                    }
                ),
                "must use the implicit Rust ABI",
            ),
            (
                quote!(
                    extern "C" fn plugin(...) -> ScenesPlugin {
                        ScenesPlugin
                    }
                ),
                "must not be variadic",
            ),
            (
                quote!(
                    fn plugin() {}
                ),
                "must declare a Plugin return type",
            ),
            (
                quote!(
                    fn plugin() -> () {}
                ),
                "must return a Plugin, not ()",
            ),
            (
                quote!(
                    fn plugin() -> (()) {}
                ),
                "must return a Plugin, not ()",
            ),
        ] {
            let error = expand(quote!(), input, quote!(::ranim)).unwrap_err();
            assert!(error.to_string().contains(expected), "{error}");
        }
    }
}
