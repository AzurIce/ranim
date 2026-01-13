use crate::utils::{expr_to_bool, expr_to_u32};
use crate::{OutputDef, SceneAttrs};

use quote::ToTokens;
use syn::{Expr, ExprLit, Lit, Meta, MetaList, MetaNameValue, token::Comma};

/// 解析 #[scene(...)] 中的参数
pub fn parse_scene_attrs(
    args: proc_macro::TokenStream,
    attrs: &[syn::Attribute],
) -> syn::Result<SceneAttrs> {
    use syn::{parse::Parser, punctuated::Punctuated};

    let mut res = SceneAttrs::default();

    if !args.is_empty() {
        let args = proc_macro2::TokenStream::from(args);
        let parser = Punctuated::<MetaNameValue, Comma>::parse_terminated;
        let kvs = parser.parse2(args)?;

        for nv in kvs {
            if nv.path.is_ident("name")
                && let Expr::Lit(ExprLit {
                    lit: Lit::Str(s), ..
                }) = nv.value
            {
                res.name = Some(s.value());
            } else if nv.path.is_ident("clear_color")
                && let Expr::Lit(ExprLit {
                    lit: Lit::Str(s), ..
                }) = nv.value
            {
                res.clear_color = Some(s.value());
            }
        }
    }

    for attr in attrs {
        if attr.path().is_ident("wasm_demo_doc") {
            res.wasm_demo_doc = true;
            continue;
        }

        if let Meta::List(list) = &attr.meta
            && list.path.is_ident("output")
        {
            res.outputs.push(parse_output_list(list)?);
        }
    }

    Ok(res)
}

// ---------- 解析单个 #[output(...)] ----------
pub fn parse_output_list(list: &MetaList) -> syn::Result<OutputDef> {
    use syn::{parse::Parser, punctuated::Punctuated};

    let mut def = OutputDef {
        width: 1920,
        height: 1080,
        fps: 60,
        save_frames: false,
        dir: "./".into(),
    };

    let parser = Punctuated::<MetaNameValue, Comma>::parse_terminated;
    let kvs = parser.parse2(list.tokens.clone())?;

    for nv in kvs {
        match nv.path.get_ident().map(|i| i.to_string()).as_deref() {
            Some("pixel_size") => {
                let tuple: syn::ExprTuple = syn::parse2(nv.value.to_token_stream())?;
                let mut elems = tuple.elems.iter();
                def.width = expr_to_u32(elems.next().unwrap())?;
                def.height = expr_to_u32(elems.next().unwrap())?;
            }
            Some("frame_rate") => def.fps = expr_to_u32(&nv.value)?,
            Some("save_frames") => def.save_frames = expr_to_bool(&nv.value)?,
            Some("dir") => {
                if let Expr::Lit(ExprLit {
                    lit: Lit::Str(s), ..
                }) = nv.value
                {
                    def.dir = s.value();
                }
            }
            _ => {}
        }
    }
    Ok(def)
}
