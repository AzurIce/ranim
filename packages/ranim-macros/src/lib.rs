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
