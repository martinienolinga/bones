//! Application d'un préréglage à un squelette : cuisson (générateur) ou
//! reciblage (clés), options (vitesse, répétitions, miroir, sur place,
//! restriction à certains os), puis fusion dans le clip du projet
//! (remplacer, insérer à l'image courante, fondre sur un intervalle).
//!
//! Tout est pur (sans état) : [`crate::Editor::apply_preset`] s'en sert et
//! crée une seule étape d'annulation.

mod merge;
mod root;

#[cfg(test)]
mod tests;

pub use merge::merge;

use super::error::PresetError;
use super::model::{Preset, PresetKind};
use super::params::Params;
use super::procedural::{self, ctx::Body};
use super::retarget::{Mapping, Report, RetargetOptions, Retargeter, retarget_clip};
use super::roles::RoleMap;
use crate::{BoneId, Pose, Skeleton};

/// Façon d'écrire l'animation dans le clip.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ApplyMode {
    /// Le clip est remplacé.
    #[default]
    Replace,
    /// Insérée à l'image `frame` ; les clés suivantes sont décalées.
    Insert,
    /// Fondue sur l'animation existante à partir de `frame`, avec des
    /// transitions de `blend_frames` images à l'entrée et à la sortie.
    Blend,
}

/// Réglages de l'application d'un préréglage.
#[derive(Clone, Debug, PartialEq)]
pub struct ApplyOptions {
    pub mode: ApplyMode,
    pub frame: u32,
    pub blend_frames: u32,
    /// Vitesse de lecture (2 : deux fois plus vite).
    pub speed: f32,
    /// Répétitions d'un préréglage en boucle.
    pub loops: u32,
    /// Gauche ↔ droite.
    pub mirror: bool,
    /// Supprime le déplacement horizontal du corps (marche sur place).
    pub in_place: bool,
    /// Rôles des os du projet réglés à la main (sinon détection automatique).
    pub roles: Option<RoleMap>,
    /// Paramètres d'un préréglage procédural (recouvrent les siens).
    pub params: Option<Params>,
    /// Seuls ces os sont animés (vide : tous) ; les autres gardent leur animation.
    pub bones: Vec<BoneId>,
    /// IK des appuis au reciblage (pieds sans glissement).
    pub foot_ik: bool,
    /// Le préréglage part de la position actuelle du personnage (insertion, fondu).
    pub align_start: bool,
}

impl Default for ApplyOptions {
    fn default() -> Self {
        Self {
            mode: ApplyMode::Replace,
            frame: 0,
            blend_frames: 6,
            speed: 1.0,
            loops: 1,
            mirror: false,
            in_place: false,
            roles: None,
            params: None,
            bones: Vec::new(),
            foot_ik: true,
            align_start: true,
        }
    }
}

/// Compte rendu de l'application (affiché par l'interface).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ApplyReport {
    /// Os animés et leur source (os source ou rôle).
    pub mapped: Vec<(String, String)>,
    /// Os qui gardent leur pose.
    pub unmapped: Vec<String>,
    pub warnings: Vec<String>,
    /// Première image écrite et nombre d'images.
    pub start: u32,
    pub frames: u32,
}

/// Poses du préréglage sur `skeleton`, une par image à `fps`, options
/// (vitesse, répétitions, miroir, sur place) appliquées. `base` donne la
/// pose des os non animés.
pub fn bake_preset(
    preset: &Preset,
    skeleton: &Skeleton,
    roles: &RoleMap,
    base: &Pose,
    options: &ApplyOptions,
    fps: f32,
) -> Result<(Vec<Pose>, Report), PresetError> {
    let (mut poses, mut report) = match &preset.kind {
        PresetKind::Procedural(p) => {
            let params = options
                .params
                .as_ref()
                .map_or(p.params.clone(), |o| p.params.merged(o));
            let clip = procedural::bake(
                p.generator,
                &params,
                skeleton,
                roles,
                fps,
                options.speed,
                options.loops,
            )?;
            let mut poses: Vec<Pose> = clip.keys().iter().map(|k| k.pose.clone()).collect();
            let report = keep_untouched(skeleton, roles, &mut poses, base);
            if options.mirror {
                mirror(skeleton, roles, &mut poses, base);
            }
            let mut report = report;
            if !p.generator.rig_type().fits(roles.rig_type) {
                report.warnings.push(format!(
                    "this preset is meant for a {} skeleton",
                    p.generator.rig_type().label().to_lowercase()
                ));
            }
            (poses, report)
        }
        PresetKind::Keyframes(data) => {
            let retarget = RetargetOptions {
                mirror: options.mirror,
                foot_ik: options.foot_ik,
                ..RetargetOptions::default()
            };
            let looping = preset.info.looping;
            retarget_clip(
                data,
                looping,
                (skeleton, roles),
                &retarget,
                base,
                fps,
                options.speed,
                options.loops,
            )?
        }
    };
    if options.in_place {
        root::strip_travel(skeleton, roles, &mut poses);
    }
    if !options.bones.is_empty() {
        for pose in &mut poses {
            for b in skeleton.ids().filter(|b| !options.bones.contains(b)) {
                pose.bones[b.index()] = base.get(b);
            }
        }
        report
            .mapped
            .retain(|(name, _)| options.bones.iter().any(|b| &skeleton.bone(*b).name == name));
    }
    Ok((poses, report))
}

/// Os jamais touchés par un générateur : ils gardent la pose `base`.
fn keep_untouched(skeleton: &Skeleton, roles: &RoleMap, poses: &mut [Pose], base: &Pose) -> Report {
    let mut report = Report::default();
    for b in skeleton.ids() {
        let i = b.index();
        let moved = poses
            .iter()
            .any(|p| !p.bones[i].abs_diff_eq(&crate::Transform::IDENTITY, 1e-6));
        let name = skeleton.bone(b).name.clone();
        if moved {
            let role = roles.role(b).map_or_else(|| "generator".into(), |r| r.label());
            report.mapped.push((name, role));
        } else {
            report.unmapped.push(name);
            for p in poses.iter_mut() {
                p.bones[i] = base.get(b);
            }
        }
    }
    report
}

/// Miroir gauche ↔ droite d'une suite de poses sur un même squelette.
fn mirror(skeleton: &Skeleton, roles: &RoleMap, poses: &mut [Pose], base: &Pose) {
    let options = RetargetOptions {
        mirror: true,
        foot_ik: false,
        ..RetargetOptions::default()
    };
    let mapping = Mapping::new((skeleton, roles), &[], (skeleton, roles), &options);
    let retargeter = Retargeter::new((skeleton, roles), (skeleton, roles), mapping, options);
    let mut body = Body::new(&retargeter.rig);
    for pose in poses.iter_mut() {
        let world = pose.world(skeleton);
        retargeter.pose(&world, glam::Vec3::ZERO, base, &mut body);
        pose.bones.clone_from(&body.pose.bones);
    }
}
