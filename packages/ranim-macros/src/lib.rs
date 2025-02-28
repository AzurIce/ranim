use darling::{ast::NestedMeta, FromMeta, Error};
use proc_macro::TokenStream;

#[derive(Debug, FromMeta)]
#[darling(default)]
struct TimelineArgs {
    width: u32,
    height: u32,
    fps: u32,
    save_frames: bool,
}

impl Default for TimelineArgs {
    fn default() -> Self {
        Self {
            width: 1920,
            height: 1080,
            fps: 60,
            save_frames: false,
        }
    }
}

#[proc_macro_attribute]
pub fn timeline(attr: TokenStream, item: TokenStream) -> TokenStream {
    let attr_args = match NestedMeta::parse_meta_list(attr.into()) {
        Ok(v) => v,
        Err(e) => return TokenStream::from(Error::from(e).write_errors()),
    };

    let args = match TimelineArgs::from_list(&attr_args) {
        Ok(v) => v,
        Err(e) => return TokenStream::from(e.write_errors()),
    };
    let func = syn::parse_macro_input!(item as syn::ItemFn);

    let func_name = &func.sig.ident;
    let func_name_upper = syn::Ident::new(&func_name.to_string().to_uppercase(), func_name.span());

    let TimelineArgs {
        width,
        height,
        fps,
        save_frames,
    } = args;

    quote::quote! {
        #[::linkme::distributed_slice(::ranim::TIMELINES)]
        static #func_name_upper: (&str, fn(&::ranim::timeline::Timeline), ::ranim::AppOptions<'static>) =
            (stringify!(#func_name), #func_name, ::ranim::AppOptions::<'static> {
                frame_size: (#width, #height),
                frame_rate: #fps,
                save_frames: #save_frames,
                output_dir: concat!("./output/", stringify!(#func_name)),
            });

        #func
    }
    .into()
}
