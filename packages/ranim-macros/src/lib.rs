use darling::{Error, FromMeta, ast::NestedMeta};
use heck::AsSnekCase;
use proc_macro::TokenStream;

#[derive(Default, Debug, FromMeta)]
#[darling(default)]
struct SceneMeta {
    name: Option<String>,
}

#[proc_macro_attribute]
pub fn scene(attr: TokenStream, item: TokenStream) -> TokenStream {
    let attr_args = match NestedMeta::parse_meta_list(attr.into()) {
        Ok(v) => v,
        Err(e) => return TokenStream::from(Error::from(e).write_errors()),
    };

    let args = match SceneMeta::from_list(&attr_args) {
        Ok(v) => v,
        Err(e) => return TokenStream::from(e.write_errors()),
    };
    let input = syn::parse_macro_input!(item as syn::ItemStruct);

    let struct_name = &input.ident;
    // 获取 struct_name 的字符串表示
    let name = match args.name {
        Some(ref name) => name.to_string(),
        None => {
            let name = struct_name.to_string();
            let name = name.strip_suffix("Scene").unwrap_or(&name);
            AsSnekCase(name).to_string()
        }
    };

    quote::quote! {
        #input

        impl ::ranim::SceneMetaTrait for #struct_name {
            fn meta(&self) -> ::ranim::SceneMeta {
                ::ranim::SceneMeta {
                    name: #name.to_string(),
                }
            }
        }
    }
    .into()
}

#[proc_macro_attribute]
pub fn item(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(item as syn::ItemStruct);
    let struct_name = &input.ident;
    let fields = match &input.fields {
        syn::Fields::Named(fields) => &fields.named,
        _ => panic!("item can only be used on structs with named fields"),
    };

    let mut_parts_name = quote::format_ident!("{}MutParts", struct_name);
    let rabject_name = quote::format_ident!("{}Rabject", struct_name);

    let owned_fileds = fields.iter().map(|f| {
        let name = &f.ident;
        quote::quote! {
            #name: self.#name.clone(),
        }
    });
    let rabject_owned_fields = fields.iter().map(|f| {
        let name = &f.ident;
        quote::quote! {
            #name: self.#name.data.clone(),
        }
    });

    let mut_parts_fields = fields.iter().map(|f| {
        let name = &f.ident;
        let ty = &f.ty;
        quote::quote! {
            pub #name: &'r mut #ty,
        }
    });

    let rabject_fields = fields.iter().map(|f| {
        let name = &f.ident;
        let ty = &f.ty;
        quote::quote! {
            pub #name: crate::items::Rabject<'t, #ty>,
        }
    });

    let mut_parts_impl = fields.iter().map(|f| {
        let name = &f.ident;
        quote::quote! {
            #name: &mut self.#name,
        }
    });

    let rabject_impl = fields.iter().map(|f| {
        let name = &f.ident;
        quote::quote! {
            #name: &mut self.#name.data,
        }
    });

    quote::quote! {
        #input

        pub struct #mut_parts_name<'r> {
            #(#mut_parts_fields)*
        }

        pub struct #rabject_name<'t> {
            #(#rabject_fields)*
        }

        impl<'r> crate::items::MutParts<'r> for #struct_name {
            type Owned = #struct_name;
            type Mut = #mut_parts_name<'r>;
            fn mut_parts(&'r mut self) -> Self::Mut {
                #mut_parts_name {
                    #(#mut_parts_impl)*
                }
            }
            fn owned(&'r self) -> Self::Owned {
                #struct_name {
                    #(#owned_fileds)*
                }
            }
        }

        impl<'r, 't: 'r> crate::items::MutParts<'r> for #rabject_name<'t> {
            type Owned = #struct_name;
            type Mut = #mut_parts_name<'r>;
            fn mut_parts(&'r mut self) -> Self::Mut {
                #mut_parts_name {
                    #(#rabject_impl)*
                }
            }
            fn owned(&'r self) -> Self::Owned {
                #struct_name {
                    #(#rabject_owned_fields)*
                }
            }
        }
    }
    .into()
}
