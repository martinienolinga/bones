//! Générateurs procéduraux : chaque préréglage procédural est un
//! générateur et ses paramètres ; il produit des clés pour n'importe quel
//! squelette dont il trouve les rôles nécessaires (proportions, longueurs de
//! jambes et orientations de repos sont lues sur le squelette cible).
//!
//! Un générateur décrit la pose à un instant normalisé `u` (en cycles pour
//! les boucles, dans [0, 1] pour les actions) ; la cuisson échantillonne
//! une pose par image, en parallèle.

pub(crate) mod animal;
pub(crate) mod ctx;
pub(crate) mod curves;
mod generator;
pub(crate) mod human;
mod misc;
mod plant;
mod pose_ops;

#[cfg(test)]
pub(crate) mod tests;

use rayon::prelude::*;

pub use generator::Generator;

use crate::io::rigging::clip_from_poses;
use crate::presets::error::{PresetError, invalid};
use crate::presets::params::Params;
use crate::presets::roles::RoleMap;
use crate::{Clip, Pose, Skeleton};
use ctx::{Body, Rigging};

/// Mouvement prêt à échantillonner sur un squelette.
pub(crate) trait Motion: Sync {
    /// Durée naturelle d'un cycle (ou de l'action), en secondes.
    fn duration(&self) -> f32;
    /// Pose au temps normalisé `u` (la pose de `body` est réinitialisée
    /// par le générateur).
    fn pose(&self, u: f32, body: &mut Body);
}

/// Nombre maximal d'images d'une cuisson.
pub(crate) const MAX_FRAMES: u32 = 100_000;

/// Cuisson d'un générateur : une clé par image (interpolation linéaire).
///
/// `speed` accélère le mouvement, `loops` répète les cycles (préréglages en
/// boucle seulement ; le déplacement s'accumule).
pub fn bake(
    generator: Generator,
    params: &Params,
    skeleton: &Skeleton,
    roles: &RoleMap,
    fps: f32,
    speed: f32,
    loops: u32,
) -> Result<Clip, PresetError> {
    if !roles.fits(skeleton) {
        return Err(invalid("bone roles do not match the skeleton"));
    }
    let rig = Rigging::new(skeleton, roles);
    let motion = generator.build(&rig, params)?;
    let fps = if fps.is_finite() { fps.clamp(1.0, 240.0) } else { 30.0 };
    let speed = if speed.is_finite() { speed.clamp(0.05, 20.0) } else { 1.0 };
    let duration = (motion.duration() / speed).clamp(1.0 / fps, 600.0);
    let per_cycle = ((duration * fps).round() as u32).max(1);
    let cycles = if generator.looping() { loops.max(1) } else { 1 };
    let count = (per_cycle * cycles + 1).min(MAX_FRAMES);
    let poses = sample(&*motion, &rig, per_cycle, count);
    Ok(clip_from_poses(fps, poses))
}

/// Une pose par image `f` ∈ [0, count), au temps normalisé f / per_cycle.
pub(crate) fn sample(motion: &dyn Motion, rig: &Rigging, per_cycle: u32, count: u32) -> Vec<Pose> {
    (0..count)
        .into_par_iter()
        .map_init(
            || Body::new(rig),
            |body, f| {
                motion.pose(f as f32 / per_cycle as f32, body);
                body.pose.clone()
            },
        )
        .collect()
}

/// Durée naturelle (secondes) et déplacement horizontal de la racine sur
/// un cycle, sans cuisson complète ; `None` si le squelette ne convient pas.
pub(crate) fn measure(
    generator: Generator,
    params: &Params,
    skeleton: &Skeleton,
    roles: &RoleMap,
) -> Option<(f32, f32)> {
    let rig = Rigging::new(skeleton, roles);
    let motion = generator.build(&rig, params).ok()?;
    let mut body = Body::new(&rig);
    let root = roles.root().unwrap_or(crate::BoneId(0));
    let mut at = |u: f32| {
        motion.pose(u, &mut body);
        body.fk();
        body.world[root.index()].translation
    };
    let d = at(1.0) - at(0.0);
    let up = roles.frame.up;
    Some((motion.duration(), (d - up * d.dot(up)).length()))
}
