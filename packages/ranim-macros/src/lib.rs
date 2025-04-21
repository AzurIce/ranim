use darling::{Error, FromMeta, ast::NestedMeta};
use heck::AsSnekCase;
use proc_macro::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Fields, parse_macro_input};

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

#[proc_macro_derive(Fill)]
pub fn derive_fill(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;
    let generics = &input.generics;
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let fields = match &input.data {
        Data::Struct(data) => &data.fields,
        _ => panic!("Fill can only be derived for structs"),
    };

    let (set_fill_opacity_impls, fill_color_impl, set_fill_color_impls) = match fields {
        Fields::Named(fields) => {
            let field_names = fields.named.iter().map(|f| &f.ident).collect::<Vec<_>>();
            let first_field = field_names.first().unwrap();
            (
                quote! {
                    #(
                        self.#field_names.set_fill_opacity(opacity);
                    )*
                },
                quote! {
                    self.#first_field.fill_color()
                },
                quote! {
                    #(
                        self.#field_names.set_fill_color(color);
                    )*
                },
            )
        }
        Fields::Unnamed(fields) => {
            let field_indices = (0..fields.unnamed.len())
                .map(syn::Index::from)
                .collect::<Vec<_>>();
            (
                quote! {
                    #(
                        self.#field_indices.set_fill_opacity(opacity);
                    )*
                },
                quote! {
                    self.0.fill_color()
                },
                quote! {
                    #(
                        self.#field_indices.set_fill_color(color);
                    )*
                },
            )
        }
        Fields::Unit => panic!("Cannot be derived for unit structs"),
    };

    let expanded = quote! {
        impl #impl_generics crate::traits::Fill for #name #ty_generics #where_clause {
            fn set_fill_opacity(&mut self, opacity: f32) -> &mut Self {
                #set_fill_opacity_impls
                self
            }
            fn fill_color(&self) -> crate::color::AlphaColor<crate::color::Srgb> {
                #fill_color_impl
            }
            fn set_fill_color(&mut self, color:  crate::color::AlphaColor<crate::color::Srgb>) -> &mut Self {
                #set_fill_color_impls
                self
            }
        }
    };

    TokenStream::from(expanded)
}

#[proc_macro_derive(Stroke)]
pub fn derive_stroke(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;
    let generics = &input.generics;
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let fields = match &input.data {
        Data::Struct(data) => &data.fields,
        _ => panic!("Stroke can only be derived for structs"),
    };

    let (set_stroke_width_impls, set_stroke_color_impls, set_stroke_opacity_impls) = match fields {
        Fields::Named(fields) => {
            let field_names = fields.named.iter().map(|f| &f.ident).collect::<Vec<_>>();
            (
                quote! {
                    #(
                        self.#field_names.set_stroke_width(width);
                    )*
                },
                quote! {
                    #(
                        self.#field_names.set_stroke_color(color);
                    )*
                },
                quote! {
                    #(
                        self.#field_names.set_stroke_opacity(opacity);
                    )*
                },
            )
        }
        Fields::Unnamed(fields) => {
            let field_indices = (0..fields.unnamed.len())
                .map(syn::Index::from)
                .collect::<Vec<_>>();
            (
                quote! {
                    #(
                        self.#field_indices.set_stroke_width(width);
                    )*
                },
                quote! {
                    #(
                        self.#field_indices.set_stroke_color(color);
                    )*
                },
                quote! {
                    #(
                        self.#field_indices.set_stroke_opacity(opacity);
                    )*
                },
            )
        }
        Fields::Unit => panic!("Cannot be derived for unit structs"),
    };

    let expanded = quote! {
        impl #impl_generics crate::traits::Stroke for #name #ty_generics #where_clause {
            fn set_stroke_width(&mut self, width: f32) -> &mut Self {
                #set_stroke_width_impls
                self
            }
            fn set_stroke_color(&mut self, color: crate::color::AlphaColor<crate::color::Srgb>) -> &mut Self {
                #set_stroke_color_impls
                self
            }
            fn set_stroke_opacity(&mut self, opacity: f32) -> &mut Self {
                #set_stroke_opacity_impls
                self
            }
        }
    };

    TokenStream::from(expanded)
}

// #[proc_macro_derive(Color)]
// pub fn derive_color(input: TokenStream) -> TokenStream {
//     let input = parse_macro_input!(input as DeriveInput);
//     let name = &input.ident;
//     let generics = &input.generics;
//     let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

//     let fields = match &input.data {
//         Data::Struct(data) => &data.fields,
//         _ => panic!("Color can only be derived for structs"),
//     };

//     let field_impls = match fields {
//         Fields::Named(fields) => {
//             let field_names = fields.named.iter().map(|f| &f.ident);
//             quote! {
//                 #(
//                     self.#field_names.color(ctx, color);
//                 )*
//             }
//         }
//         Fields::Unnamed(fields) => {
//             let field_indices = (0..fields.unnamed.len()).map(syn::Index::from);
//             quote! {
//                 #(
//                     self.#field_indices.color(ctx, color);
//                 )*
//             }
//         }
//         Fields::Unit => quote! {},
//     };

//     let expanded = quote! {
//         impl #impl_generics crate::traits::Color for #name #ty_generics #where_clause {
//             fn color(&mut self, ctx: &crate::context::WgpuContext, color: &crate::items::Color) {
//                 #field_impls
//             }
//         }
//     };

//     TokenStream::from(expanded)
// }

#[proc_macro_derive(Empty)]
pub fn derive_empty(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;
    let generics = &input.generics;
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let fields = match &input.data {
        Data::Struct(data) => &data.fields,
        _ => panic!("Empty can only be derived for structs"),
    };

    let field_impls = match fields {
        Fields::Named(fields) => {
            let (field_names, field_types): (Vec<_>, Vec<_>) =
                fields.named.iter().map(|f| (&f.ident, &f.ty)).unzip();

            quote! {
                Self {
                    #(
                        #field_names: #field_types::empty(),
                    )*
                }
            }
        }
        Fields::Unnamed(fields) => {
            let field_types = fields.unnamed.iter().map(|f| &f.ty);
            quote! {
                Self (
                    #(
                        #field_types::empty(),
                    )*
                )
            }
        }
        Fields::Unit => quote! {},
    };

    let expanded = quote! {
        impl #impl_generics crate::traits::Empty for #name #ty_generics #where_clause {
            fn empty() -> Self {
                #field_impls
            }
        }
    };

    TokenStream::from(expanded)
}

#[proc_macro_derive(Opacity)]
pub fn derive_opacity(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;
    let generics = &input.generics;
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let fields = match &input.data {
        Data::Struct(data) => &data.fields,
        _ => panic!("Opacity can only be derived for structs"),
    };

    let field_impls = match fields {
        Fields::Named(fields) => {
            let field_names = fields.named.iter().map(|f| &f.ident);
            quote! {
                #(
                    self.#field_names.set_opacity(opacity);
                )*
            }
        }
        Fields::Unnamed(fields) => {
            let field_indices = (0..fields.unnamed.len()).map(syn::Index::from);
            quote! {
                #(
                    self.#field_indices.set_opacity(opacity);
                )*
            }
        }
        Fields::Unit => panic!("Cannot be derived for unit structs"),
    };

    let expanded = quote! {
        impl #impl_generics crate::traits::Opacity for #name #ty_generics #where_clause {
            fn set_opacity(&mut self, opacity: f32) -> &mut Self {
                #field_impls
                self
            }
        }
    };

    TokenStream::from(expanded)
}

#[proc_macro_derive(Alignable)]
pub fn derive_alignable(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;
    let generics = &input.generics;
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let fields = match &input.data {
        Data::Struct(data) => &data.fields,
        _ => panic!("Alignable can only be derived for structs"),
    };

    let (is_aligned_impls, align_with_impls) = match fields {
        Fields::Named(fields) => {
            let field_names = fields.named.iter().map(|f| &f.ident).collect::<Vec<_>>();
            (
                quote! {
                    #(
                        self.#field_names.is_aligned(&other.#field_names) &&
                    )* true
                },
                quote! {
                    #(
                        self.#field_names.align_with(&mut other.#field_names);
                    )*
                },
            )
        }
        Fields::Unnamed(fields) => {
            let field_indices = (0..fields.unnamed.len())
                .map(syn::Index::from)
                .collect::<Vec<_>>();
            (
                quote! {
                    #(
                        self.#field_indices.is_aligned(&other.#field_indices) &&
                    )* true
                },
                quote! {
                    #(
                        self.#field_indices.align_with(&mut other.#field_indices);
                    )*
                },
            )
        }
        Fields::Unit => panic!("Cannot be derived for unit structs"),
    };

    let expanded = quote! {
        impl #impl_generics crate::traits::Alignable for #name #ty_generics #where_clause {
            fn is_aligned(&self, other: &Self) -> bool {
                #is_aligned_impls
            }
            fn align_with(&mut self, other: &mut Self) {
                #align_with_impls
            }
        }
    };

    TokenStream::from(expanded)
}

#[proc_macro_derive(Interpolatable)]
pub fn derive_interpolatable(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;
    let generics = &input.generics;
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let fields = match &input.data {
        Data::Struct(data) => &data.fields,
        _ => panic!("Interpolatable can only be derived for structs"),
    };

    let field_impls = match fields {
        Fields::Named(fields) => {
            let field_names = fields.named.iter().map(|f| &f.ident);
            quote! {
                Self {
                    #(
                        #field_names: self.#field_names.lerp(&other.#field_names, t),
                    )*
                }
            }
        }
        Fields::Unnamed(fields) => {
            let field_indices = (0..fields.unnamed.len()).map(syn::Index::from);
            quote! {
                Self {
                    #(
                        self.#field_indices.lerp(&other.#field_indices, t);
                    )*
                }
            }
        }
        Fields::Unit => panic!("Cannot be derived for unit structs"),
    };

    let expanded = quote! {
        impl #impl_generics crate::traits::Interpolatable for #name #ty_generics #where_clause {
            fn lerp(&self, other: &Self, t: f64) -> Self {
                #field_impls
            }
        }
    };

    TokenStream::from(expanded)
}
