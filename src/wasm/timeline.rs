//! Transfer the core animation metadata without renderer or UI handles.

use js_sys::{Array, Object, Reflect};
use ranim_core::animation::node::{AnimationInfo, AnimationInfoKind};
use wasm_bindgen::JsValue;

pub(super) fn snapshot(duration: f64, roots: Vec<AnimationInfo>) -> Result<JsValue, JsValue> {
    object(&[("duration", duration.into()), ("roots", nodes(roots)?)])
}

fn nodes(infos: Vec<AnimationInfo>) -> Result<JsValue, JsValue> {
    let result = Array::new();
    for info in infos {
        let kind = match info.kind {
            AnimationInfoKind::Anim => "anim",
            AnimationInfoKind::Children => "children",
            AnimationInfoKind::Audio => "audio",
        };
        result.push(&object(&[
            ("name", info.anim_name.into()),
            ("kind", kind.into()),
            (
                "range",
                Array::of2(&info.range.start.into(), &info.range.end.into()).into(),
            ),
            ("contentDuration", info.content_duration_secs.into()),
            ("enabled", info.enabled.into()),
            (
                "simStep",
                info.sim_step.map(JsValue::from).unwrap_or(JsValue::NULL),
            ),
            ("children", nodes(info.children)?),
        ])?);
    }
    Ok(result.into())
}

fn object(fields: &[(&str, JsValue)]) -> Result<JsValue, JsValue> {
    let result = Object::new();
    for (key, value) in fields {
        Reflect::set(&result, &JsValue::from_str(key), value)?;
    }
    Ok(result.into())
}
