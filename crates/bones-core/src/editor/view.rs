//! Données d'affichage dérivées de l'état : os, centre de masse,
//! trajectoires, fantômes et poignées.

use glam::Vec3;

use super::{Editor, Handle, Mode, is_limb_marker, mirror_x};
use crate::anim::physics;
use crate::{LimbMode, MarkerKind, Transform};

impl Editor {
    /// Transformations monde des os telles qu'affichées.
    pub fn display_world(&self) -> Vec<Transform> {
        match self.mode {
            // La peinture de poids se fait sur le maillage posé, comme dans Blender.
            Mode::Pose | Mode::WeightPaint => self.rig.world(),
            Mode::Edit | Mode::AutoRig => self.rig.skeleton.rest_world(),
        }
    }

    pub fn skinning_matrices(&self) -> Vec<glam::Mat4> {
        match self.mode {
            Mode::Pose | Mode::WeightPaint => self.rig.skinning_matrices(),
            Mode::Edit | Mode::AutoRig => vec![glam::Mat4::IDENTITY; self.rig.skeleton.len()],
        }
    }

    pub fn is_dragging(&self) -> bool {
        self.drag.is_some()
    }

    pub fn current_frame(&self) -> u32 {
        self.frame.round().max(0.0) as u32
    }

    /// Centre de masse de la pose courante (masses mises en cache).
    pub fn center_of_mass(&mut self) -> Option<Vec3> {
        let revs = self.rig.revisions();
        let stale = self
            .masses
            .as_ref()
            .is_none_or(|(skin, skel, _)| *skin != revs.skin || *skel != revs.skeleton);
        if stale {
            let m = physics::bone_masses(&self.rig.skeleton, &self.rig.skin);
            self.masses = Some((revs.skin, revs.skeleton, m));
        }
        let masses = &self.masses.as_ref()?.2;
        physics::center_of_mass(&self.rig.skeleton, &self.display_world(), masses)
    }

    /// Trajectoire de l'os sélectionné sur toute l'animation (en cache).
    pub fn trajectory(&mut self) -> Option<&[Vec3]> {
        let bone = self.selected?;
        if self.rig.clip.keys().len() < 2 || self.mode != Mode::Pose {
            return None;
        }
        let revs = self.rig.revisions();
        let stale = self
            .trajectory
            .as_ref()
            .is_none_or(|(b, c, s, _)| *b != bone || *c != revs.clip || *s != revs.skeleton);
        if stale {
            let points = self.rig.clip.trajectory(&self.rig.skeleton, bone);
            self.trajectory = Some((bone, revs.clip, revs.skeleton, points));
        }
        self.trajectory.as_ref().map(|t| t.3.as_slice())
    }

    /// Poses fantômes (onion skin) aux clés précédente et suivante.
    pub fn ghosts(&self) -> Vec<Vec<Transform>> {
        let frame = self.current_frame();
        let clip = &self.rig.clip;
        [clip.prev_key_frame(frame), clip.next_key_frame(frame)]
            .into_iter()
            .flatten()
            .filter_map(|f| clip.key_at(f))
            .map(|k| k.pose.world(&self.rig.skeleton))
            .collect()
    }

    /// Poignées visibles et leur position monde.
    pub fn handles(&self) -> Vec<(Handle, Vec3)> {
        let mut out = Vec::new();
        match self.mode {
            Mode::Pose => {
                for (i, limb) in self.rig.limbs.iter().enumerate() {
                    if limb.mode == LimbMode::Ik && limb.is_valid(&self.rig.skeleton) {
                        out.push((Handle::IkTarget(i), limb.target));
                        out.push((Handle::IkPole(i), limb.pole));
                    }
                }
            }
            Mode::AutoRig => {
                if let Some(m) = &self.markers {
                    for kind in MarkerKind::ALL {
                        let p = m.get(kind);
                        out.push((Handle::Marker(kind, false), p));
                        if is_limb_marker(kind) {
                            out.push((Handle::Marker(kind, true), mirror_x(p, m.root.x)));
                        }
                    }
                }
            }
            Mode::Edit | Mode::WeightPaint => {}
        }
        out
    }
}
