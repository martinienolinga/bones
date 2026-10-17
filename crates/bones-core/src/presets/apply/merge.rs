//! Écriture des poses cuites dans le clip : remplacement, insertion (les
//! clés suivantes sont décalées dans le temps et dans l'espace pour que le
//! personnage reparte de là où le préréglage l'a laissé) ou fondu.

use super::root::{flat, position, root_bone, shift, start_at};
use super::{ApplyMode, ApplyOptions};
use crate::presets::roles::RoleMap;
use crate::{Clip, Interpolation, Pose, Skeleton};

/// Pose existante à l'image `frame` (clip, sinon pose courante).
fn existing(clip: &Clip, current: &Pose, frame: u32) -> Pose {
    let mut pose = current.clone();
    clip.sample_into(frame as f32, &mut pose);
    pose
}

fn put(clip: &mut Clip, frame: u32, pose: Pose) {
    clip.set_key(frame, pose);
    if let Some(k) = clip.key_at_mut(frame) {
        k.interpolation = Interpolation::Linear;
    }
}

/// Écrit `poses` dans `clip` selon `options.mode`. Les os hors de
/// `options.bones` (s'il n'est pas vide) gardent l'animation existante.
/// Retourne la première image écrite et le nombre d'images.
pub fn merge(
    clip: &mut Clip,
    skeleton: &Skeleton,
    roles: &RoleMap,
    mut poses: Vec<Pose>,
    options: &ApplyOptions,
    current: &Pose,
) -> (u32, u32) {
    let n = poses.len() as u32;
    let masked = |b: usize| !options.bones.is_empty() && !options.bones.iter().any(|x| x.index() == b);
    let keep = |pose: &mut Pose, old: &Pose| {
        for (i, slot) in pose.bones.iter_mut().enumerate() {
            if masked(i) {
                *slot = old.bones.get(i).copied().unwrap_or_default();
            }
        }
    };
    let start = if options.mode == ApplyMode::Replace { 0 } else { options.frame };
    if options.mode != ApplyMode::Replace && options.align_start && options.bones.is_empty() {
        let root = root_bone(skeleton, roles);
        let at = position(skeleton, &existing(clip, current, start), root);
        start_at(skeleton, roles, &mut poses, at);
    }
    match options.mode {
        ApplyMode::Replace => {
            let old = std::mem::replace(clip, Clip::new(clip.fps, n.max(2)));
            let total = if options.bones.is_empty() { n } else { n.max(old.frame_count) };
            for f in 0..total {
                let mut pose = poses.get(f as usize).cloned().unwrap_or_else(|| existing(&old, current, f));
                keep(&mut pose, &existing(&old, current, f));
                put(clip, f, pose);
            }
        }
        ApplyMode::Insert => {
            let root = root_bone(skeleton, roles);
            let travel = match (poses.first(), poses.last()) {
                (Some(a), Some(b)) => flat(roles, position(skeleton, b, root) - position(skeleton, a, root)),
                _ => glam::Vec3::ZERO,
            };
            let hold = existing(clip, current, start);
            let old = std::mem::replace(clip, Clip::new(clip.fps, 2));
            clip.frame_count = old.frame_count.max(start) + n;
            for key in old.keys() {
                if key.frame < start {
                    clip.set_key(key.frame, key.pose.clone());
                } else {
                    let mut pose = key.pose.clone();
                    if options.bones.is_empty() {
                        shift(skeleton, &mut pose, root, travel);
                    }
                    clip.set_key(key.frame + n, pose);
                }
                if let Some(k) = clip.key_at_mut(if key.frame < start { key.frame } else { key.frame + n }) {
                    k.interpolation = key.interpolation;
                }
            }
            for (j, mut pose) in poses.into_iter().enumerate() {
                keep(&mut pose, &hold);
                put(clip, start + j as u32, pose);
            }
        }
        ApplyMode::Blend => {
            let fade = options.blend_frames.min(n / 2).max(1) as f32;
            let before = clip.clone();
            for (j, pose) in poses.into_iter().enumerate() {
                let f = start + j as u32;
                let old = existing(&before, current, f);
                let into = (j as f32 + 1.0) / fade;
                let out = (n - j as u32) as f32 / fade;
                let w = into.min(out).min(1.0);
                let mut mixed = old.clone();
                for (i, slot) in mixed.bones.iter_mut().enumerate() {
                    if !masked(i) {
                        *slot = old.get(crate::BoneId(i as u32)).lerp(&pose.get(crate::BoneId(i as u32)), w);
                    }
                }
                put(clip, f, mixed);
            }
            clip.frame_count = clip.frame_count.max(start + n);
        }
    }
    (start, n)
}
