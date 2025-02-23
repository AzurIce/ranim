use proc_macro::TokenStream;

#[proc_macro_attribute]
pub fn timeline(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let func = syn::parse_macro_input!(item as syn::ItemFn);

    let func_name = &func.sig.ident;
    let func_name_upper = syn::Ident::new(&func_name.to_string().to_uppercase(), func_name.span());

    quote::quote! {
        #[::linkme::distributed_slice(::ranim::TIMELINES)]
        static #func_name_upper: (&str, fn(&Timeline)) =
            (stringify!(#func_name), #func_name);

        #func
    }
    .into()
}
