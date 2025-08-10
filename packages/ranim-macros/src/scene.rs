use crate::{SceneAttrs, OutputDef};
use crate::utils::{expr_to_bool, expr_to_u32};

use quote::{ToTokens};
use syn::{
    Expr, ExprLit, Lit, Meta, MetaList, MetaNameValue,
    token::Comma,
};

pub fn parse_scene_attrs(attrs: &[syn::Attribute]) -> syn::Result<SceneAttrs> {
    use syn::{parse::Parser, punctuated::Punctuated};

    let mut res = SceneAttrs::default();

    for attr in attrs {
        if attr.path().is_ident("preview") {
            res.preview = true;
            continue;
        }

        // 统一拿 meta
        let meta = &attr.meta;

        if let Meta::List(list) = meta {
            if list.path.is_ident("scene") {
                // 解析 #[scene(key = value, ...)]
                let parser = Punctuated::<MetaNameValue, Comma>::parse_terminated;
                let kvs = parser.parse2(list.tokens.clone())?;
                for nv in kvs {
                    if nv.path.is_ident("name") {
                        if let Expr::Lit(ExprLit {
                            lit: Lit::Str(s), ..
                        }) = nv.value
                        {
                            res.name = Some(s.value());
                        }
                    } else if nv.path.is_ident("frame_height") {
                        if let Expr::Lit(ExprLit {
                            lit: Lit::Float(f), ..
                        }) = nv.value
                        {
                            res.frame_height = Some(f.base10_parse()?);
                        }
                    }
                }
            }

            if list.path.is_ident("output") {
                res.outputs.push(parse_output_list(list)?);
            }
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