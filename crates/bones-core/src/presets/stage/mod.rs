//! Préréglages joués sur un modèle de présentation (le mannequin de la
//! galerie) : vignettes rendues avec le vrai maillage plutôt qu'en bâtons.
//!
//! - [`StageModel`] : le modèle préparé une fois (squelette, rôles, repos
//!   inverse, sondes de cadrage prises sur le maillage) ;
//! - [`Staged`] : un préréglage cuit sur ce modèle — animation humanoïde
//!   reciblée et jouée sur place ([`Shot::Body`]), ou expression faciale à
//!   pleine intensité ([`Shot::Face`]) — avec son cadrage, calculé sur les
//!   sondes déformées à chaque image : la silhouette reste dans la vignette
//!   pendant tout le cycle.
//!
//! Tout est pur et sans GPU : l'application cuit sur un fil de fond, puis
//! demande les matrices de skinning de l'image voulue ([`Staged::joints`]).

mod framing;
#[cfg(test)]
mod tests;

use glam::{Mat4, Vec2, Vec3};

pub use framing::Extent;

use super::apply::{ApplyOptions, bake_preset};
use super::error::{PresetError, incompatible};
use super::model::{Preset, PresetKind};
use super::roles::{RigType, RoleMap, detect};
use crate::{OrbitCamera, Pose, Rig, Skeleton, Transform};

/// Cadrage d'une vignette.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Shot {
    /// Corps entier, de trois quarts.
    Body,
    /// Gros plan sur le visage, presque de face.
    Face,
}

impl Shot {
    /// Orientation de la caméra (lacet, tangage), en radians. Lacet positif :
    /// la caméra passe du côté gauche du personnage (+X).
    fn angles(self) -> (f32, f32) {
        match self {
            Shot::Body => (0.52, 0.10),
            Shot::Face => (0.30, 0.04),
        }
    }
}

/// Le préréglage peut-il être joué sur un modèle humanoïde, et avec quel
/// cadrage ? `None` : préréglage d'animal, de végétal… sans sens sur un
/// humain (l'interface garde alors l'aperçu en bâtons de son squelette).
pub fn shot_for(preset: &Preset) -> Option<Shot> {
    match &preset.kind {
        PresetKind::Expression(_) => Some(Shot::Face),
        _ if preset.info.rig_type == RigType::Humanoid => Some(Shot::Body),
        _ => None,
    }
}

/// Modèle de présentation préparé une fois.
pub struct StageModel {
    skeleton: Skeleton,
    roles: RoleMap,
    /// Inverse des matrices monde de repos (skinning).
    rest_inv: Vec<Mat4>,
    probes: framing::Probes,
}

impl StageModel {
    pub fn new(rig: &Rig) -> Self {
        let skeleton = rig.skeleton.clone();
        let roles = detect(&skeleton);
        let rest_inv = skeleton
            .rest_world()
            .iter()
            .map(|t| t.to_matrix().inverse())
            .collect();
        let probes = framing::Probes::new(rig);
        Self {
            skeleton,
            roles,
            rest_inv,
            probes,
        }
    }

    pub fn skeleton(&self) -> &Skeleton {
        &self.skeleton
    }

    /// Matrices de skinning d'une pose, sans allocation (`world` et `out`
    /// sont des tampons réutilisés).
    fn skinning(&self, pose: &Pose, world: &mut Vec<Transform>, out: &mut Vec<Mat4>) {
        world.clear();
        out.clear();
        for (i, bone) in self.skeleton.bones().iter().enumerate() {
            let local = bone.rest.mul(&pose.bones.get(i).copied().unwrap_or_default());
            let w = match bone.parent() {
                Some(p) => world[p.index()].mul(&local),
                None => local,
            };
            world.push(w);
            out.push(w.to_matrix() * self.rest_inv[i]);
        }
    }
}

/// Préréglage cuit sur le modèle de présentation.
pub struct Staged {
    shot: Shot,
    /// Une pose par image à `FPS` (une seule pour une expression).
    poses: Vec<Pose>,
    looping: bool,
    extent: Extent,
}

/// Cadence de cuisson des vignettes.
pub const FPS: f32 = 30.0;

/// Pause sur la dernière image d'une action non bouclée, avant de rejouer.
const HOLD: f32 = 0.7;

impl Staged {
    /// Cuit `preset` sur `model` : animation reciblée sur place, ou
    /// expression à pleine intensité. Erreur si le préréglage ne convient pas
    /// à un humanoïde (voir [`shot_for`]).
    pub fn new(preset: &Preset, model: &StageModel) -> Result<Self, PresetError> {
        let shot = shot_for(preset).ok_or_else(|| incompatible("not a humanoid preset"))?;
        let skeleton = &model.skeleton;
        let poses = match &preset.kind {
            PresetKind::Expression(data) => {
                vec![super::expression::face_pose(data, skeleton, 1.0, false).0]
            }
            _ => {
                let options = ApplyOptions {
                    in_place: true,
                    ..ApplyOptions::default()
                };
                let base = Pose::rest(skeleton);
                bake_preset(preset, skeleton, &model.roles, &base, &options, FPS)?.0
            }
        };
        if poses.is_empty() {
            return Err(incompatible("empty preset"));
        }
        let extent = framing::extent(model, &poses, shot);
        Ok(Self {
            shot,
            poses,
            looping: preset.info.looping,
            extent,
        })
    }

    pub fn shot(&self) -> Shot {
        self.shot
    }

    /// Nombre d'images cuites (1 : pose fixe).
    pub fn frames(&self) -> usize {
        self.poses.len()
    }

    /// Pose fixe (expression) : inutile de redessiner la vignette.
    pub fn is_static(&self) -> bool {
        self.poses.len() < 2
    }

    /// Image à jouer à l'instant `time` (secondes) : boucle, ou action
    /// rejouée après une courte pause sur sa fin.
    pub fn frame_at(&self, time: f32) -> usize {
        let n = self.poses.len();
        if n < 2 || !time.is_finite() {
            return 0;
        }
        let last = (n - 1) as f32;
        let f = if self.looping {
            (time * FPS).rem_euclid(last)
        } else {
            (time.rem_euclid(last / FPS + HOLD) * FPS).min(last)
        };
        (f.floor() as usize).min(n - 1)
    }

    /// Matrices de skinning de l'image `frame`. `world` et `out` sont des
    /// tampons réutilisés d'un appel à l'autre.
    pub fn joints(&self, model: &StageModel, frame: usize, world: &mut Vec<Transform>, out: &mut Vec<Mat4>) {
        let pose = &self.poses[frame.min(self.poses.len() - 1)];
        model.skinning(pose, world, out);
    }

    /// Zone à cadrer (tout le cycle), dans le plan de la caméra du cadrage.
    pub fn extent(&self) -> Extent {
        self.extent
    }

    /// Caméra orthographique qui cadre le préréglage dans une vignette de
    /// rapport `aspect` (largeur / hauteur), avec une marge relative `margin`.
    pub fn camera(&self, aspect: f32, margin: f32) -> OrbitCamera {
        let (yaw, pitch) = self.shot.angles();
        let mut camera = OrbitCamera {
            yaw,
            pitch,
            orthographic: true,
            ..OrbitCamera::default()
        };
        let Extent { center, half } = self.extent;
        let half = half.max(Vec2::splat(1e-3));
        let h = half.y.max(half.x / aspect.max(1e-3)) * (1.0 + margin.max(0.0));
        camera.target = center;
        camera.distance = h / (camera.fov_y * 0.5).tan();
        camera
    }
}

/// Point du plan de la caméra d'un cadrage : (droite, haut) d'un point monde.
fn plane_axes(shot: Shot) -> (Vec3, Vec3) {
    let (yaw, pitch) = shot.angles();
    let camera = OrbitCamera {
        yaw,
        pitch,
        ..OrbitCamera::default()
    };
    (camera.right(), camera.up())
}
