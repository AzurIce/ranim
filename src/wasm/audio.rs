//! JavaScript-owned copies of the scene's seal-time audio mix.

use js_sys::{Array, Float32Array, Object, Reflect};
use ranim_core::audio::{MASTER_CHANNELS, MASTER_SAMPLE_RATE};
use wasm_bindgen::JsValue;

pub(super) fn snapshot(pcm: &[f32]) -> Result<JsValue, JsValue> {
    if pcm.is_empty() {
        return Ok(JsValue::NULL);
    }
    let channels = Array::new();
    for channel in 0..usize::from(MASTER_CHANNELS) {
        let samples: Vec<f32> = pcm
            .iter()
            .skip(channel)
            .step_by(usize::from(MASTER_CHANNELS))
            .copied()
            .collect();
        channels.push(&Float32Array::from(samples.as_slice()));
    }
    let result = Object::new();
    Reflect::set(&result, &"sampleRate".into(), &MASTER_SAMPLE_RATE.into())?;
    Reflect::set(&result, &"channels".into(), &channels)?;
    Ok(result.into())
}
