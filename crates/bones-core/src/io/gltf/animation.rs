//! Échantillonnage de l'animation en canaux glTF.

use glam::Vec3;
use rayon::prelude::*;
use serde_json::{Value, json};

use super::FLOAT;
use super::buffer::Builder;
use crate::{BoneId, Pose, Rig, Transform};

pub(super) fn animation(b: &mut Builder, rig: &Rig, bone_node: impl Fn(usize) -> usize) -> Value {
    let skeleton = &rig.skeleton;
    let clip = &rig.clip;
    let first = clip.keys().first().map(|k| k.frame).unwrap_or(0);
    let last = clip.keys().last().map(|k| k.frame).unwrap_or(0);
    let frames: Vec<u32> = (first..=last).collect();
    let times: Vec<f32> = frames.iter().map(|f| (*f - first) as f32 / clip.fps).collect();

    // Échantillonnage parallèle : une pose locale complète par image.
    let locals: Vec<Vec<Transform>> = frames
        .par_iter()
        .map_init(
            || Pose::rest(skeleton),
            |pose, f| {
                clip.sample_into(*f as f32, pose);
                skeleton
                    .bones()
                    .iter()
                    .enumerate()
                    .map(|(i, bone)| bone.rest.mul(&pose.get(BoneId(i as u32))))
                    .collect()
            },
        )
        .collect();

    let time_view = b.view(bytemuck::cast_slice(&times), None);
    let input = b.accessor(
        time_view,
        FLOAT,
        times.len(),
        "SCALAR",
        json!({ "min": [times[0]], "max": [*times.last().unwrap()] }),
    );

    let mut samplers = Vec::new();
    let mut channels = Vec::new();
    for bone in 0..skeleton.len() {
        let t: Vec<Vec3> = locals.iter().map(|l| l[bone].translation).collect();
        let r: Vec<[f32; 4]> = locals
            .iter()
            .map(|l| l[bone].rotation.normalize().to_array())
            .collect();
        let s: Vec<Vec3> = locals.iter().map(|l| l[bone].scale).collect();
        let tv = b.vec3s(&t, None, false);
        let rv = b.view(bytemuck::cast_slice(&r), None);
        let ra = b.accessor(rv, FLOAT, r.len(), "VEC4", json!({}));
        let sv = b.vec3s(&s, None, false);
        for (output, path) in [(tv, "translation"), (ra, "rotation"), (sv, "scale")] {
            channels.push(json!({
                "sampler": samplers.len(),
                "target": { "node": bone_node(bone), "path": path },
            }));
            samplers.push(json!({ "input": input, "output": output, "interpolation": "LINEAR" }));
        }
    }
    json!({ "name": "Animation", "samplers": samplers, "channels": channels })
}
