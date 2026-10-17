//! Aperçu animé pour la galerie : le préréglage est cuit une fois sur son
//! squelette (référence pour un générateur, squelette source pour des
//! clés), sur place ; l'interface dessine ensuite les os en bâtons à
//! n'importe quel instant, sans toucher au projet.

use glam::Vec3;

use super::apply::{ApplyOptions, bake_preset};
use super::error::PresetError;
use super::model::{Preset, PresetKind};
use super::roles::{Lr, RoleMap, detect};
use crate::{Clip, Pose, Skeleton, head_tail};

/// Aperçu d'un préréglage (léger : quelques centaines de poses au plus).
pub struct Preview {
    skeleton: Skeleton,
    roles: RoleMap,
    clip: Clip,
    looping: bool,
    lo: Vec3,
    hi: Vec3,
}

impl Preview {
    /// Cuisson à 30 images/s, déplacement du corps retiré.
    pub fn new(preset: &Preset) -> Result<Self, PresetError> {
        let (skeleton, roles) = match &preset.kind {
            PresetKind::Procedural(p) => {
                let s = p.reference().skeleton();
                let r = detect(&s);
                (s, r)
            }
            PresetKind::Keyframes(data) => (data.skeleton.clone(), data.roles.clone()),
        };
        let options = ApplyOptions {
            in_place: true,
            ..ApplyOptions::default()
        };
        let base = Pose::rest(&skeleton);
        let (poses, _) = bake_preset(preset, &skeleton, &roles, &base, &options, 30.0)?;
        let mut clip = Clip::new(30.0, poses.len().max(2) as u32);
        let (mut lo, mut hi) = (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN));
        for (f, pose) in poses.into_iter().enumerate() {
            for (w, b) in pose.world(&skeleton).iter().zip(skeleton.bones()) {
                let (h, t) = head_tail(w, b.length);
                lo = lo.min(h.min(t));
                hi = hi.max(h.max(t));
            }
            clip.set_key(f as u32, pose);
        }
        Ok(Self {
            skeleton,
            roles,
            clip,
            looping: preset.info.looping,
            lo,
            hi,
        })
    }

    /// Durée d'un cycle (ou de l'action), en secondes.
    pub fn duration(&self) -> f32 {
        self.clip.frame_count.saturating_sub(1) as f32 / self.clip.fps
    }

    pub fn skeleton(&self) -> &Skeleton {
        &self.skeleton
    }

    /// Boîte englobant tout le mouvement (cadrage de la vignette).
    pub fn bounds(&self) -> (Vec3, Vec3) {
        (self.lo, self.hi)
    }

    /// Côté de chaque os (couleurs gauche/droite du dessin).
    pub fn sides(&self) -> Vec<Option<Lr>> {
        self.roles
            .roles
            .iter()
            .map(|r| r.and_then(|r| r.chain.side()))
            .collect()
    }

    /// Segments (tête, queue) monde de chaque os à l'instant `time`
    /// (secondes ; boucle si le préréglage boucle, sinon reste sur la fin).
    pub fn segments(&self, time: f32, out: &mut Vec<[Vec3; 2]>) {
        let d = self.duration().max(1e-3);
        let t = if self.looping {
            time.rem_euclid(d)
        } else {
            time.clamp(0.0, d)
        };
        let mut pose = Pose::rest(&self.skeleton);
        self.clip.sample_into(t * self.clip.fps, &mut pose);
        out.clear();
        out.extend(
            pose.world(&self.skeleton)
                .iter()
                .zip(self.skeleton.bones())
                .map(|(w, b)| {
                    let (h, t) = head_tail(w, b.length);
                    [h, t]
                }),
        );
    }
}
