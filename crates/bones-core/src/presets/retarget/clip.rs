//! Reciblage d'un clip entier : rééchantillonnage à la cadence cible,
//! vitesse, répétitions (le déplacement de la racine s'accumule d'un cycle
//! à l'autre), calcul parallèle (une image par tâche).

use glam::Vec3;
use rayon::prelude::*;

use super::{Mapping, Report, RetargetOptions, Retargeter};
use crate::presets::error::{PresetError, invalid};
use crate::presets::model::KeyframeData;
use crate::presets::procedural::ctx::Body;
use crate::presets::procedural::MAX_FRAMES;
use crate::presets::roles::RoleMap;
use crate::{Pose, Skeleton};

/// Recible `data` sur `target` : une pose par image à `fps`, `speed` fois
/// plus vite, répétée `loops` fois si l'animation boucle. `base` donne la
/// pose des os sans correspondance.
#[allow(clippy::too_many_arguments)]
pub fn retarget_clip(
    data: &KeyframeData,
    looping: bool,
    target: (&Skeleton, &RoleMap),
    options: &RetargetOptions,
    base: &Pose,
    fps: f32,
    speed: f32,
    loops: u32,
) -> Result<(Vec<Pose>, Report), PresetError> {
    let (tgt, roles) = target;
    if !roles.fits(tgt) || !data.roles.fits(&data.skeleton) {
        return Err(invalid("bone roles do not match the skeleton"));
    }
    if data.clip.is_empty() {
        return Err(invalid("the preset has no keys"));
    }
    let mapping = Mapping::new((&data.skeleton, &data.roles), &data.bones, (tgt, roles), options);
    let mut report = mapping.report.clone();
    if mapping.mapped() == 0 {
        return Err(PresetError::Incompatible("no bone of the skeleton matches the preset".into()));
    }
    if data.roles.rig_type != roles.rig_type && !data.roles.rig_type.fits(roles.rig_type) {
        report.warnings.push(format!(
            "the preset was made for a {} skeleton, this one looks {}",
            data.roles.rig_type.label().to_lowercase(),
            roles.rig_type.label().to_lowercase()
        ));
    }
    let retargeter = Retargeter::new((&data.skeleton, &data.roles), (tgt, roles), mapping, *options);

    let src = &data.skeleton;
    let clip = &data.clip;
    let (first, last) = (clip.keys()[0].frame, clip.keys()[clip.keys().len() - 1].frame);
    let span = (last - first) as f32 / clip.fps.max(1.0);
    let fps = if fps.is_finite() { fps.clamp(1.0, 240.0) } else { 30.0 };
    let speed = if speed.is_finite() { speed.clamp(0.05, 20.0) } else { 1.0 };
    let cycles = if looping { loops.max(1) } else { 1 };
    let count = ((span * cycles as f32 / speed * fps).round() as u32 + 1).min(MAX_FRAMES);

    // Déplacement horizontal de la racine sur un cycle (pour enchaîner les boucles).
    let root = data.roles.root().unwrap_or(crate::BoneId(0));
    let world_at = |frame: f32| {
        let mut pose = Pose::rest(src);
        clip.sample_into(frame, &mut pose);
        pose.world(src)
    };
    let up = data.roles.frame.up;
    let step = world_at(last as f32)[root.index()].translation - world_at(first as f32)[root.index()].translation;
    let step = if looping { step - up * step.dot(up) } else { Vec3::ZERO };

    let poses = (0..count)
        .into_par_iter()
        .map_init(
            || (Body::new(&retargeter.rig), Pose::rest(src)),
            |(body, src_pose), j| {
                let time = j as f32 / fps * speed;
                let cycle = if span > 0.0 { ((time / span).floor() as u32).min(cycles - 1) } else { 0 };
                let local = (time - cycle as f32 * span).clamp(0.0, span);
                clip.sample_into(first as f32 + local * clip.fps, src_pose);
                let world = src_pose.world(src);
                retargeter.pose(&world, step * cycle as f32, base, body);
                body.pose.clone()
            },
        )
        .collect();
    Ok((poses, report))
}
