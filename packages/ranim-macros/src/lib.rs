mod scene;
mod utils;

use proc_macro::TokenStream;
use proc_macro_crate::{FoundCrate, crate_name};
use proc_macro2::Span;
use quote::quote;
use syn::{Data, DeriveInput, Fields, Ident, ItemFn, parse_macro_input};

use crate::scene::parse_scene_attrs;

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

/// 解析单个属性（#[scene(...)] /  / #[output(...)]）
#[derive(Default)]
struct SceneAttrs {
    name: Option<String>,        // #[scene(name = "...")]
    frame_height: Option<f64>,   // #[scene(frame_height = 8.0)]
    clear_color: Option<String>, // #[scene(clear_color = "#000000")]
    wasm_demo_doc: bool,         // #[wasm_demo_doc]
    outputs: Vec<OutputDef>,     // #[output(...)]
}

/// 一个 #[output(...)] 里的字段
#[derive(Default)]
struct OutputDef {
    width: u32,
    height: u32,
    fps: u32,
    save_frames: bool,
    dir: String,
}

// MARK: scene
#[proc_macro_attribute]
pub fn scene(args: TokenStream, input: TokenStream) -> TokenStream {
    let ranim = ranim_path();
    let input_fn = parse_macro_input!(input as ItemFn);
    let attrs = parse_scene_attrs(args, input_fn.attrs.as_slice()).unwrap();

    let fn_name = &input_fn.sig.ident;
    let vis = &input_fn.vis;
    let fn_body = &input_fn.block;
    let doc_attrs: Vec<_> = input_fn
        .attrs
        .iter()
        .filter(|attr| attr.path().is_ident("doc"))
        .collect();

    // 场景名称
    let scene_name = attrs.name.unwrap_or_else(|| fn_name.to_string());

    // SceneConfig
    let frame_height = attrs.frame_height.unwrap_or(8.0);
    let clear_color = attrs.clear_color.unwrap_or("#333333ff".to_string());
    let scene_config = quote! {
        #ranim::SceneConfig {
            frame_height: #frame_height,
            clear_color: #clear_color,
        }
    };

    // Output 列表
    let mut outputs = Vec::new();
    for OutputDef {
        width,
        height,
        fps,
        save_frames,
        dir,
    } in attrs.outputs
    {
        outputs.push(quote! {
            #ranim::Output {
                width: #width,
                height: #height,
                fps: #fps,
                save_frames: #save_frames,
                dir: #dir,
            }
        });
    }
    if outputs.is_empty() {
        outputs.push(quote! {
            #ranim::Output::DEFAULT
        });
    }

    let doc = if attrs.wasm_demo_doc {
        quote! {
            #[doc = concat!("<canvas id=\"ranim-app-", stringify!(#fn_name), "\" width=\"1280\" height=\"720\" style=\"width: 100%;\"></canvas>")]
            #[doc = concat!("<script type=\"module\">")]
            #[doc = concat!("  const { find_scene } = await ranim_examples;")]
            #[doc = concat!("  find_scene(\"", stringify!(#fn_name), "\").run_app();")]
            #[doc = "</script>"]
        }
    } else {
        quote! {}
    };

    let static_output_name = syn::Ident::new(
        &format!("__SCENE_{}_OUTPUTS", fn_name.to_string().to_uppercase()),
        fn_name.span(),
    );
    let static_name = syn::Ident::new(
        &format!("__SCENE_{}", fn_name.to_string().to_uppercase()),
        fn_name.span(),
    );
    let static_scene_name = syn::Ident::new(&format!("{fn_name}_scene"), fn_name.span());

    let output_cnt = outputs.len();

    let scene = quote! {
        #ranim::Scene {
            name: #scene_name,
            constructor: #fn_name,
            config: #scene_config,
            outputs: &#static_output_name,
        }
    };

    // 构造 Scene 并塞进分布式切片
    let expanded = quote! {
        #doc
        #(#doc_attrs)*
        #vis fn #fn_name(r: &mut #ranim::timeline::RanimScene) #fn_body

        static #static_output_name: [#ranim::Output; #output_cnt] = [#(#outputs),*];
        #[doc(hidden)]
        static #static_name: #ranim::Scene = #scene;
        #ranim::inventory::submit!{
            #scene
        }

        #[allow(non_upper_case_globals)]
        #vis static #static_scene_name: &'static #ranim::Scene = &#static_name;
    };

    TokenStream::from(expanded)
}

#[proc_macro_attribute]
pub fn output(_: TokenStream, _: TokenStream) -> TokenStream {
    TokenStream::new()
}

// #[proc_macro_attribute]
// pub fn preview(_: TokenStream, _: TokenStream) -> TokenStream {
//     TokenStream::new()
// }

#[proc_macro_attribute]
pub fn wasm_demo_doc(_attr: TokenStream, _: TokenStream) -> TokenStream {
    TokenStream::new()
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
