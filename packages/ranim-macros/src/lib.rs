use darling::{Error, FromMeta, ast::NestedMeta};
use heck::AsSnekCase;
use proc_macro::TokenStream;
use syn::{Path, parse::Parse, punctuated::Punctuated, spanned::Spanned, token::Comma};

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

#[derive(Default, Debug, FromMeta)]
#[darling(default)]
struct ItemMeta {
    base_item: Option<syn::Type>,
}

struct Item {
    path: Path,
}

impl Parse for Item {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let path = input.parse::<Path>()?;
        Ok(Self { path })
    }
}

#[proc_macro_derive(Item, attributes(item))]
pub fn derive_item(input: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(input as syn::DeriveInput);
    let struct_name = &input.ident;

    // Parse item attribute
    let item_attr = input
        .attrs
        .iter()
        .find(|attr| attr.path().is_ident("item"))
        .expect("needs an item attribute to work");
    let items = item_attr
        .parse_args_with(Punctuated::<Item, Comma>::parse_terminated)
        .expect("failed to parse Item attributes");
    if items.len() != 1 {
        return syn::Error::new(item_attr.span(), "needs exactly one item")
            .to_compile_error()
            .into();
    }
    let item = items.first().expect("needs at least one item");

    let base_item = item.path.clone();

    let fields = match &input.data {
        syn::Data::Struct(s) => &s.fields,
        _ => panic!("Item can only be used on structs"),
    };

    let mut_parts_name = quote::format_ident!("{}MutParts", struct_name);
    let rabject_name = quote::format_ident!("{}Rabject", struct_name);

    let res = match fields {
        // MARK: Named item
        syn::Fields::Named(fields) => {
            let (field_names, field_types): (Vec<_>, Vec<_>) =
                fields.named.iter().map(|f| (&f.ident, &f.ty)).unzip();

            let mut_parts_fields: Vec<_> = field_names
                .iter()
                .zip(field_types.iter())
                .map(|(name, ty)| {
                    quote::quote! {
                        pub #name: <#ty as crate::items::MutParts<'a>>::Mut,
                    }
                })
                .collect();

            let rabject_fields: Vec<_> = field_names
                .iter()
                .zip(field_types.iter())
                .map(|(name, ty)| {
                    quote::quote! {
                        pub #name: <#ty as crate::items::Item>::Rabject<'t>,
                    }
                })
                .collect();

            let mut_parts_impl: Vec<_> = field_names
                .iter()
                .map(|name| {
                    quote::quote! {
                        #name: self.#name.mut_parts(),
                    }
                })
                .collect();

            let owned_impl: Vec<_> = field_names
                .iter()
                .map(|name| {
                    quote::quote! {
                        #name: self.#name.owned(),
                    }
                })
                .collect();

            let insert_into_timeline_impl: Vec<_> = field_names
                .iter()
                .map(|name| {
                    quote::quote! {
                        #name: crate::items::Item::insert_into_timeline(self.#name, ranim_timeline),
                    }
                })
                .collect();

            let iter_mut_impl = field_names
                .iter()
                .map(|name| {
                    quote::quote! {
                        self.#name.iter_mut()
                    }
                })
                .collect::<Vec<_>>();

            let chain_iter_mut = iter_mut_impl.iter().fold(
                quote::quote! { std::iter::empty() },
                |acc, iter| quote::quote! { #acc.chain(#iter) },
            );

            quote::quote! {
                pub struct #mut_parts_name<'a> {
                    #(#mut_parts_fields)*
                }

                pub struct #rabject_name<'t> {
                    #(#rabject_fields)*
                }

                impl<'a> crate::items::MutParts<'a> for #struct_name {
                    type Owned = #struct_name;
                    type Mut = #mut_parts_name<'a>;
                    fn mut_parts(&'a mut self) -> Self::Mut {
                        #mut_parts_name {
                            #(#mut_parts_impl)*
                        }
                    }
                    fn owned(&'a self) -> Self::Owned {
                        #struct_name {
                            #(#owned_impl)*
                        }
                    }
                }

                impl<'a, 't: 'a> crate::items::MutParts<'a> for #rabject_name<'t> {
                    type Owned = #struct_name;
                    type Mut = #mut_parts_name<'a>;
                    fn mut_parts(&'a mut self) -> Self::Mut {
                        #mut_parts_name {
                            #(#mut_parts_impl)*
                        }
                    }
                    fn owned(&'a self) -> Self::Owned {
                        #struct_name {
                            #(#owned_impl)*
                        }
                    }
                }

                impl crate::items::Item for #struct_name {
                    type BaseItem = #base_item;
                    type Rabject<'t> = #rabject_name<'t>;
                    fn insert_into_timeline<'t>(self, ranim_timeline: &'t crate::RanimTimeline) -> Self::Rabject<'t> {
                        #rabject_name {
                            #(#insert_into_timeline_impl)*
                        }
                    }
                }

                impl<'t: 'r, 'r> crate::items::IterMutRabjects<'t, 'r, #base_item> for #rabject_name<'t> {
                    fn iter_mut<'a, 'b>(&'a mut self) -> impl Iterator<Item = &'b mut crate::items::Rabject<'t, #base_item>>
                    where
                        'a: 'b,
                        't: 'b,
                        #base_item: 'b,
                    {
                        #chain_iter_mut
                    }
                }
            }
        }
        // MARK: Unnamed item
        syn::Fields::Unnamed(fields) => {
            let (idx, field_types) = fields
                .unnamed
                .iter()
                .enumerate()
                .map(|(idx, f)| (syn::Index::from(idx), &f.ty))
                .unzip::<_, _, Vec<_>, Vec<_>>();

            let mut_parts_fields: Vec<_> = field_types
                .iter()
                .map(|ty| {
                    quote::quote! {
                        pub <#ty as crate::items::MutParts<'a>>::Mut
                    }
                })
                .collect();

            let rabject_fields: Vec<_> = field_types
                .iter()
                .map(|ty| {
                    quote::quote! {
                        pub <#ty as crate::items::Item>::Rabject<'t>
                    }
                })
                .collect();

            let mut_parts_impl: Vec<_> = idx
                .iter()
                .map(|idx| {
                    quote::quote! {
                        self.#idx.mut_parts()
                    }
                })
                .collect();

            let owned_impl: Vec<_> = idx
                .iter()
                .map(|idx| {
                    quote::quote! {
                        self.#idx.owned()
                    }
                })
                .collect();

            let insert_into_timeline_impl: Vec<_> = idx
                .iter()
                .map(|idx| {
                    quote::quote! {
                        crate::items::Item::insert_into_timeline(self.#idx, ranim_timeline),
                    }
                })
                .collect();

            let iter_mut_impl = idx
                .iter()
                .map(|idx| {
                    quote::quote! {
                        self.#idx.iter_mut()
                    }
                })
                .collect::<Vec<_>>();

            let chain_iter_mut = iter_mut_impl.iter().fold(
                quote::quote! { std::iter::empty() },
                |acc, iter| quote::quote! { #acc.chain(#iter) },
            );

            quote::quote! {
                pub struct #mut_parts_name<'a> (
                    #(#mut_parts_fields)*
                );

                pub struct #rabject_name<'t> (
                    #(#rabject_fields)*
                );

                impl<'a> crate::items::MutParts<'a> for #struct_name {
                    type Owned = #struct_name;
                    type Mut = #mut_parts_name<'a>;
                    fn mut_parts(&'a mut self) -> Self::Mut {
                        #mut_parts_name (
                            #(#mut_parts_impl)*
                        )
                    }
                    fn owned(&'a self) -> Self::Owned {
                        #struct_name (
                            #(#owned_impl)*
                        )
                    }
                }

                impl<'a, 't: 'a> crate::items::MutParts<'a> for #rabject_name<'t> {
                    type Owned = #struct_name;
                    type Mut = #mut_parts_name<'a>;
                    fn mut_parts(&'a mut self) -> Self::Mut {
                        #mut_parts_name (
                            #(#mut_parts_impl)*
                        )
                    }
                    fn owned(&'a self) -> Self::Owned {
                        #struct_name (
                            #(#owned_impl)*
                        )
                    }
                }

                impl crate::items::Item for #struct_name {
                    type BaseItem = #base_item;
                    type Rabject<'t> = #rabject_name<'t>;
                    fn insert_into_timeline<'t>(self, ranim_timeline: &'t crate::RanimTimeline) -> Self::Rabject<'t> {
                        #rabject_name (
                            #(#insert_into_timeline_impl)*
                        )
                    }
                }

                impl<'t: 'r, 'r> crate::items::IterMutRabjects<'t, 'r, #base_item> for #rabject_name<'t> {
                    fn iter_mut<'a, 'b>(&'a mut self) -> impl Iterator<Item = &'b mut crate::items::Rabject<'t, #base_item>>
                    where
                        'a: 'b,
                        't: 'b,
                        #base_item: 'b,
                    {
                        #chain_iter_mut
                    }
                }
            }
        }
        syn::Fields::Unit => panic!("Item can not be used on unit structs"),
    };

    // dbg!(res.to_string());

    res.into()
}

#[proc_macro_derive(BaseMutParts)]
pub fn base_mut_parts_derive(input: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(input as syn::DeriveInput);
    let name = &input.ident;

    quote::quote! {
        impl crate::items::BaseMutParts for #name {}
    }
    .into()
}
