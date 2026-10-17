//! Animation glTF : les canaux (translation, rotation, échelle) sont
//! échantillonnés à 30 images/s, puis convertis en poses par les matrices
//! monde des articulations.

use std::sync::Arc;

use glam::{Mat4, Quat, Vec3, Vec4};
use gltf::animation::util::ReadOutputs;
use gltf::animation::{Interpolation, Property};

use super::Scene;
use crate::Clip;
use crate::io::rigging::BuiltSkeleton;

const FPS: f32 = 30.0;

struct Track {
    node: usize,
    property: Property,
    interpolation: Interpolation,
    times: Vec<f32>,
    /// Valeurs (×3 en CUBICSPLINE : tangente d'entrée, valeur, tangente de sortie).
    values: Vec<Vec4>,
}

impl Track {
    fn value(&self, k: usize) -> Vec4 {
        match self.interpolation {
            Interpolation::CubicSpline => self.values[3 * k + 1],
            _ => self.values[k],
        }
    }

    fn sample(&self, t: f32) -> Vec4 {
        let n = self.times.len();
        if n == 0 {
            return Vec4::ZERO;
        }
        if t <= self.times[0] {
            return self.value(0);
        }
        if t >= self.times[n - 1] {
            return self.value(n - 1);
        }
        let i = self.times.partition_point(|x| *x <= t);
        let (t0, t1) = (self.times[i - 1], self.times[i]);
        let dt = (t1 - t0).max(1e-6);
        let u = (t - t0) / dt;
        let (a, b) = (self.value(i - 1), self.value(i));
        let rotation = self.property == Property::Rotation;
        match self.interpolation {
            Interpolation::Step => a,
            Interpolation::Linear if rotation => Vec4::from(
                Quat::from_vec4(a)
                    .normalize()
                    .slerp(Quat::from_vec4(b).normalize(), u),
            ),
            Interpolation::Linear => a.lerp(b, u),
            Interpolation::CubicSpline => {
                let m0 = self.values[3 * (i - 1) + 2] * dt;
                let m1 = self.values[3 * i] * dt;
                let (u2, u3) = (u * u, u * u * u);
                let v = a * (2.0 * u3 - 3.0 * u2 + 1.0)
                    + m0 * (u3 - 2.0 * u2 + u)
                    + b * (-2.0 * u3 + 3.0 * u2)
                    + m1 * (u3 - u2);
                if rotation { v.normalize_or(a) } else { v }
            }
        }
    }
}

fn tracks(doc: &gltf::Document, buffers: &[Arc<[u8]>]) -> Vec<Track> {
    // Animation la plus longue en nombre de canaux.
    let Some(animation) = doc.animations().max_by_key(|a| a.channels().count()) else {
        return Vec::new();
    };
    animation
        .channels()
        .filter_map(|channel| {
            let reader = channel.reader(|b| buffers.get(b.index()).map(|d| &d[..]));
            let times: Vec<f32> = reader.read_inputs()?.collect();
            let values: Vec<Vec4> = match reader.read_outputs()? {
                ReadOutputs::Translations(it) | ReadOutputs::Scales(it) => {
                    it.map(|v| Vec3::from_array(v).extend(0.0)).collect()
                }
                ReadOutputs::Rotations(r) => r.into_f32().map(Vec4::from_array).collect(),
                ReadOutputs::MorphTargetWeights(_) => return None,
            };
            let interpolation = channel.sampler().interpolation();
            let per_key = if interpolation == Interpolation::CubicSpline {
                3
            } else {
                1
            };
            (values.len() >= times.len() * per_key && !times.is_empty()).then(|| Track {
                node: channel.target().node().index(),
                property: channel.target().property(),
                interpolation,
                times,
                values,
            })
        })
        .collect()
}

pub(super) fn sample(
    doc: &gltf::Document,
    buffers: &[Arc<[u8]>],
    scene: &Scene,
    joint_nodes: &[usize],
    built: &BuiltSkeleton,
) -> Option<Clip> {
    let tracks = tracks(doc, buffers);
    if tracks.is_empty() {
        return None;
    }
    let duration = tracks
        .iter()
        .filter_map(|t| t.times.last())
        .fold(0.0f32, |a, b| a.max(*b));
    let frames = ((duration * FPS).round() as u32 + 1).max(2);
    let rest: Vec<(Vec3, Quat, Vec3)> = doc
        .nodes()
        .map(|n| {
            let (t, r, s) = n.transform().decomposed();
            (Vec3::from_array(t), Quat::from_array(r), Vec3::from_array(s))
        })
        .collect();

    Some(built.clip(FPS, frames, |f| {
        let time = f as f32 / FPS;
        let mut local = rest.clone();
        for track in &tracks {
            let v = track.sample(time);
            let slot = &mut local[track.node];
            match track.property {
                Property::Translation => slot.0 = v.truncate(),
                Property::Rotation => slot.1 = Quat::from_vec4(v).normalize(),
                Property::Scale => slot.2 = v.truncate(),
                Property::MorphTargetWeights => {}
            }
        }
        let mut world = vec![Mat4::IDENTITY; local.len()];
        for &n in &scene.order {
            let (t, r, s) = local[n];
            let m = Mat4::from_scale_rotation_translation(s, r, t);
            world[n] = match scene.parent[n] {
                Some(p) => world[p] * m,
                None => m,
            };
        }
        joint_nodes.iter().map(|n| world[*n]).collect()
    }))
}
