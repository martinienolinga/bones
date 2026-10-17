//! Tampon binaire glTF : vues, accesseurs et assemblage du conteneur GLB.

use glam::Vec3;
use serde_json::{Value, json};

use super::FLOAT;

#[derive(Default)]
pub(super) struct Builder {
    pub(super) bin: Vec<u8>,
    pub(super) views: Vec<Value>,
    pub(super) accessors: Vec<Value>,
}

impl Builder {
    pub(super) fn view(&mut self, bytes: &[u8], target: Option<u32>) -> usize {
        while !self.bin.len().is_multiple_of(4) {
            self.bin.push(0);
        }
        let mut view = json!({
            "buffer": 0,
            "byteOffset": self.bin.len(),
            "byteLength": bytes.len(),
        });
        if let Some(t) = target {
            view["target"] = json!(t);
        }
        self.bin.extend_from_slice(bytes);
        self.views.push(view);
        self.views.len() - 1
    }

    pub(super) fn accessor(
        &mut self,
        view: usize,
        component: u32,
        count: usize,
        kind: &str,
        extra: Value,
    ) -> usize {
        let mut acc = json!({
            "bufferView": view,
            "componentType": component,
            "count": count,
            "type": kind,
        });
        if let (Some(a), Some(e)) = (acc.as_object_mut(), extra.as_object()) {
            a.extend(e.clone());
        }
        self.accessors.push(acc);
        self.accessors.len() - 1
    }

    pub(super) fn vec3s(&mut self, data: &[Vec3], target: Option<u32>, bounds: bool) -> usize {
        let view = self.view(bytemuck::cast_slice(data), target);
        let extra = if bounds {
            let (lo, hi) = data
                .iter()
                .fold((Vec3::splat(f32::MAX), Vec3::splat(f32::MIN)), |(lo, hi), p| {
                    (lo.min(*p), hi.max(*p))
                });
            json!({ "min": lo.to_array(), "max": hi.to_array() })
        } else {
            json!({})
        };
        self.accessor(view, FLOAT, data.len(), "VEC3", extra)
    }
}

pub(super) fn pack_glb(doc: &Value, mut bin: Vec<u8>) -> Vec<u8> {
    let mut json = serde_json::to_vec(doc).expect("JSON sérialisable");
    while !json.len().is_multiple_of(4) {
        json.push(b' ');
    }
    while !bin.len().is_multiple_of(4) {
        bin.push(0);
    }
    let total = 12 + 8 + json.len() + 8 + bin.len();
    let mut out = Vec::with_capacity(total);
    out.extend_from_slice(b"glTF");
    out.extend_from_slice(&2u32.to_le_bytes());
    out.extend_from_slice(&(total as u32).to_le_bytes());
    out.extend_from_slice(&(json.len() as u32).to_le_bytes());
    out.extend_from_slice(b"JSON");
    out.extend_from_slice(&json);
    out.extend_from_slice(&(bin.len() as u32).to_le_bytes());
    out.extend_from_slice(b"BIN\0");
    out.extend_from_slice(&bin);
    out
}
