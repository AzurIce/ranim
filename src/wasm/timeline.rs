//! Serialize retained definitions at the browser boundary.

use js_sys::{Array, Object, Reflect};
use ranim_core::animation::def_node::AnimDefNode;
use wasm_bindgen::JsValue;

pub(super) fn snapshot(duration: f64, roots: &[AnimDefNode]) -> Result<JsValue, JsValue> {
    object(&[("duration", duration.into()), ("roots", nodes(roots)?)])
}

fn nodes(definitions: &[AnimDefNode]) -> Result<JsValue, JsValue> {
    let result = Array::new();
    for definition in definitions {
        let kind = if definition.is_container() {
            "children"
        } else if definition.is_audio() {
            "audio"
        } else {
            "anim"
        };
        let range = definition.time_range();
        result.push(&object(&[
            ("name", definition.anim_name().into()),
            ("kind", kind.into()),
            (
                "range",
                Array::of2(&range.start.into(), &range.end.into()).into(),
            ),
            ("contentDuration", definition.content_duration_secs().into()),
            ("enabled", definition.enabled().into()),
            (
                "simStep",
                definition
                    .sim_step()
                    .map(JsValue::from)
                    .unwrap_or(JsValue::NULL),
            ),
            ("children", nodes(definition.definitions())?),
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
