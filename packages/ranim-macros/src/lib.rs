use darling::{Error, FromMeta, ast::NestedMeta};
use heck::AsSnekCase;
use proc_macro::TokenStream;
use proc_macro_crate::{FoundCrate, crate_name};
use proc_macro2::Span;
use quote::quote;
use syn::{parse_macro_input, Data, DeriveInput, Fields, Ident, ItemFn};

/// 标记函数的宏，将函数信息收集到 linkme 分布式切片中
#[proc_macro_attribute]
pub fn preview(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let ranim = ranim_path();
    let input_fn = parse_macro_input!(item as ItemFn);
    let fn_name = &input_fn.sig.ident;
    let fn_name_str = fn_name.to_string();
    
    // 生成唯一的静态变量名
    let static_name = syn::Ident::new(
        &format!("__PREVIEW_FUNC_{}", fn_name.to_string().to_uppercase()),
        fn_name.span()
    );
    
    let expanded = quote! {
        #input_fn
        
        #[#ranim::linkme::distributed_slice(#ranim::PREVIEW_FUNCS)]
        #[linkme(crate = #ranim::linkme)]
        static #static_name: #ranim::PreviewFunc = #ranim::PreviewFunc {
            name: #fn_name_str,
            fn_ptr: #fn_name,
        };
    };
    
    TokenStream::from(expanded)
}

const RANIM_CRATE_NAME: &str = "ranim";

fn ranim_path() -> proc_macro2::TokenStream {
    match (
        crate_name(RANIM_CRATE_NAME),
        std::env::var("CARGO_CRATE_NAME").as_deref(),
    ) {
        (Ok(FoundCrate::Itself), Ok(RANIM_CRATE_NAME)) => quote!(crate),
        (Ok(FoundCrate::Name(name)), _) => {
            let ident = Ident::new(&name, Span::call_site());
            quote!(::#ident)
        }
        _ => quote!(::ranim),
    }
}

#[derive(Default, Debug, FromMeta)]
#[darling(default)]
struct SceneMeta {
    name: Option<String>,
}

#[proc_macro_attribute]
pub fn scene(attr: TokenStream, item: TokenStream) -> TokenStream {
    let ranim = ranim_path();
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

        impl #ranim::SceneMetaTrait for #struct_name {
            fn meta(&self) -> #ranim::SceneMeta {
                #ranim::SceneMeta {
                    name: #name.to_string(),
                }
            }
        }
    }
    .into()
}

// MARK: derive Traits

#[proc_macro_derive(Fill)]
pub fn derive_fill(input: TokenStream) -> TokenStream {
    impl_derive(
        input,
        |ranim| quote! {#ranim::traits::Fill},
        |ranim, field_positions| {
            quote! {
                fn set_fill_opacity(&mut self, opacity: f32) -> &mut Self {
                    #(
                        self.#field_positions.set_fill_opacity(opacity);
                    )*
                    self
                }
                fn fill_color(&self) -> #ranim::color::AlphaColor<#ranim::color::Srgb> {
                    [#(self.#field_positions.fill_color(), )*].first().cloned().unwrap()
                }
                fn set_fill_color(&mut self, color:  #ranim::color::AlphaColor<#ranim::color::Srgb>) -> &mut Self {
                    #(
                        self.#field_positions.set_fill_color(color);
                    )*
                    self
                }
            }
        },
    )
}

#[proc_macro_derive(Stroke)]
pub fn derive_stroke(input: TokenStream) -> TokenStream {
    impl_derive(
        input,
        |ranim| quote! {#ranim::traits::Stroke},
        |ranim, field_positions| {
            quote! {
                fn stroke_color(&self) -> #ranim::color::AlphaColor<#ranim::color::Srgb> {
                    [#(self.#field_positions.stroke_color(), )*].first().cloned().unwrap()
                }
                fn apply_stroke_func(&mut self, f: impl for<'a> Fn(&'a mut [#ranim::components::width::Width])) -> &mut Self {
                    #(
                        self.#field_positions.apply_stroke_func(&f);
                    )*
                    self
                }
                fn set_stroke_color(&mut self, color: #ranim::color::AlphaColor<#ranim::color::Srgb>) -> &mut Self {
                    #(
                        self.#field_positions.set_stroke_color(color);
                    )*
                    self
                }
                fn set_stroke_opacity(&mut self, opacity: f32) -> &mut Self {
                    #(
                        self.#field_positions.set_stroke_opacity(opacity);
                    )*
                    self
                }
            }
        },
    )
}

#[proc_macro_derive(Partial)]
pub fn derive_partial(input: TokenStream) -> TokenStream {
    impl_derive(
        input,
        |ranim| quote! {#ranim::traits::Partial},
        |_ranim, field_positions| {
            quote! {
                fn get_partial(&self, range: std::ops::Range<f64>) -> Self {
                    Self {
                        #(
                            #field_positions: self.#field_positions.get_partial(range.clone()),
                        )*
                    }
                }
                fn get_partial_closed(&self, range: std::ops::Range<f64>) -> Self {
                    Self {
                        #(
                            #field_positions: self.#field_positions.get_partial(range.clone()),
                        )*
                    }
                }
            }
        },
    )
}

#[proc_macro_derive(Empty)]
pub fn derive_empty(input: TokenStream) -> TokenStream {
    let ranim = ranim_path();
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
        impl #impl_generics #ranim::traits::Empty for #name #ty_generics #where_clause {
            fn empty() -> Self {
                #field_impls
            }
        }
    };

    TokenStream::from(expanded)
}

#[proc_macro_derive(Opacity)]
pub fn derive_opacity(input: TokenStream) -> TokenStream {
    impl_derive(
        input,
        |ranim| quote! {#ranim::traits::Opacity},
        |_ranim, field_positions| {
            quote! {
                fn set_opacity(&mut self, opacity: f32) -> &mut Self {
                    #(
                        self.#field_positions.set_opacity(opacity);
                    )*
                    self
                }
            }
        },
    )
}

#[proc_macro_derive(Alignable)]
pub fn derive_alignable(input: TokenStream) -> TokenStream {
    impl_derive(
        input,
        |ranim| quote! {#ranim::traits::Alignable},
        |_ranim, field_positions| {
            quote! {
                fn is_aligned(&self, other: &Self) -> bool {
                    #(
                        self.#field_positions.is_aligned(&other.#field_positions) &&
                    )* true
                }
                fn align_with(&mut self, other: &mut Self) {
                    #(
                        self.#field_positions.align_with(&mut other.#field_positions);
                    )*
                }
            }
        },
    )
}

#[proc_macro_derive(Interpolatable)]
pub fn derive_interpolatable(input: TokenStream) -> TokenStream {
    impl_derive(
        input,
        |ranim| quote! {#ranim::traits::Interpolatable},
        |ranim, field_positions| {
            quote! {
                fn lerp(&self, other: &Self, t: f64) -> Self {
                    Self {
                        #(
                            #field_positions: #ranim::traits::Interpolatable::lerp(&self.#field_positions, &other.#field_positions, t),
                        )*
                    }
                }
            }
        },
    )
}

#[proc_macro_derive(BoundingBox)]
pub fn derive_bounding_box(input: TokenStream) -> TokenStream {
    let ranim = ranim_path();
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;
    let generics = &input.generics;
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let fields = match &input.data {
        Data::Struct(data) => &data.fields,
        _ => panic!("Can only be derived for structs"),
    };

    let field_positions = get_field_positions(fields)
        .ok_or("cannot get field from unit struct")
        .unwrap();

    let expanded = quote! {
        impl #impl_generics #ranim::traits::BoundingBox for #name #ty_generics #where_clause {
            fn get_bounding_box(&self) -> [DVec3; 3] {
                let [min, max] = [#(self.#field_positions.get_bounding_box(), )*]
                    .into_iter()
                    .map(|[min, _, max]| [min, max])
                    .reduce(|[acc_min, acc_max], [min, max]| [acc_min.min(min), acc_max.max(max)])
                    .unwrap();
                [min, (min + max) / 2.0, max]
            }
        }
    };

    TokenStream::from(expanded)
}

#[proc_macro_derive(Position)]
pub fn derive_position(input: TokenStream) -> TokenStream {
    impl_derive(
        input,
        |ranim| {
            quote! {#ranim::traits::Position}
        },
        |ranim, field_positions| {
            quote! {
                fn shift(&mut self, shift: DVec3) -> &mut Self {
                    #(self.#field_positions.shift(shift);)*
                    self
                }

                fn rotate_by_anchor(&mut self, angle: f64, axis: #ranim::glam::DVec3, anchor: #ranim::components::Anchor) -> &mut Self {
                    #(self.#field_positions.rotate_by_anchor(angle, axis, anchor);)*
                    self
                }

                fn scale_by_anchor(&mut self, scale: #ranim::glam::DVec3, anchor: #ranim::components::Anchor) -> &mut Self {
                    #(self.#field_positions.scale_by_anchor(scale, anchor);)*
                    self
                }
            }
        },
    )
}

#[proc_macro_derive(PointsFunc)]
pub fn derive_point_func(input: TokenStream) -> TokenStream {
    impl_derive(
        input,
        |ranim| {
            quote! {#ranim::traits::PointsFunc}
        },
        |_ranim, field_positions| {
            quote! {
                fn apply_points_func(&mut self, f: impl for<'a> Fn(&'a mut [DVec3])) -> &mut Self {
                    #(self.#field_positions.apply_points_func(f);)*
                    self
                }
            }
        },
    )
}

fn impl_derive(
    input: TokenStream,
    trait_path: impl Fn(&proc_macro2::TokenStream) -> proc_macro2::TokenStream,
    impl_token: impl Fn(
        &proc_macro2::TokenStream,
        Vec<proc_macro2::TokenStream>,
    ) -> proc_macro2::TokenStream,
) -> TokenStream {
    let ranim = ranim_path();
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;
    let generics = &input.generics;
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let fields = match &input.data {
        Data::Struct(data) => &data.fields,
        _ => panic!("Can only be derived for structs"),
    };

    let field_positions = get_field_positions(fields)
        .ok_or("cannot get field from unit struct")
        .unwrap();

    let trait_path = trait_path(&ranim);
    let impl_token = impl_token(&ranim, field_positions);
    let expanded = quote! {
        impl #impl_generics #trait_path for #name #ty_generics #where_clause {
            #impl_token
        }
    };

    TokenStream::from(expanded)
}

fn get_field_positions(fields: &Fields) -> Option<Vec<proc_macro2::TokenStream>> {
    match fields {
        Fields::Named(fields) => Some(
            fields
                .named
                .iter()
                .map(|f| {
                    let pos = &f.ident;
                    quote! { #pos }
                })
                .collect::<Vec<_>>(),
        ),
        Fields::Unnamed(fields) => Some(
            (0..fields.unnamed.len())
                .map(syn::Index::from)
                .map(|i| {
                    quote! { #i }
                })
                .collect::<Vec<_>>(),
        ),
        Fields::Unit => None,
    }
}
