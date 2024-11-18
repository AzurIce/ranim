extern crate proc_macro;

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Ident, ItemStruct};

#[proc_macro_attribute]
pub fn mobject(attr: TokenStream, input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as ItemStruct);
    let pipeline_type = parse_macro_input!(attr as Ident);

    let vis = &input.vis;
    let struct_name = &input.ident;
    let fields = input.fields.iter();
    let attrs = input.attrs.iter();
    let expanded = quote! {
        #(#attrs)*
        #vis struct #struct_name {
            #(#fields),*,
            vertex_buffer: WgpuBuffer<<#pipeline_type as RenderPipeline>::Vertex>,
        }

        impl #struct_name {
            pub fn update_buffer(&mut self, ctx: &WgpuContext) {
                self.vertex_buffer.prepare_from_slice(ctx, &self.to_pipeline_vertex());
            }
        }
    };

    // eprintln!("{}", expanded);
    expanded.into()
}

// #[proc_macro_derive(Mobject)]
// pub fn mobject(input: TokenStream) -> TokenStream {
//     let input = parse_macro_input!(input as DeriveInput);
//     match input.data {
//         Data::Struct(data) => {

//         }
//         _ => todo!()
//     }
//     todo!()
// }

// fn impl_mobject(input: DeriveInput) -> TokenStream {
//     let name = &input.ident;
//     quote::quote! {
//         impl Mobject for #name {
//             fn to_pipeline_vertex(&self) -> Vec<SimpleVertex> {
//                 todo!()
//             }
//         }
//     }.into()
// }
