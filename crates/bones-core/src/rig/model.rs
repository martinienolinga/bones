//! Transformation du modèle entier : maillage, os de repos, translations de
//! pose et d'animation, cibles IK et épingles, comme un objet unique.

use std::sync::Arc;

use glam::Vec3;

use super::document::{Rig, next_revision};
use crate::math::rotation_arc;
use crate::{BoneId, Pose, Transform, head_tail};

impl Rig {
    /// Applique `m` (translation, rotation, échelle éventuellement non
    /// uniforme selon les axes du monde) à tout le modèle.
    ///
    /// Les poids restent valables : s'ils étaient à jour, aucune nouvelle
    /// liaison n'est demandée (les révisions du maillage et du squelette
    /// changent, celle de la liaison les suit).
    pub fn transform_model(&mut self, m: &Transform) {
        let start = self.clone();
        self.transform_model_from(&start, m);
    }

    /// `self` devient `start` transformé par `m`. Pendant un geste, `start`
    /// est l'état du début : aucune erreur ne s'accumule, et les tampons du
    /// maillage sont réutilisés d'un appel à l'autre.
    pub fn transform_model_from(&mut self, start: &Rig, m: &Transform) {
        let bound = !start.needs_bind();
        self.transform_mesh(start, m);

        // Os de repos : têtes et queues transformées ; la rotation suit
        // exactement celle de `m` (roulis compris), puis l'échelle non
        // uniforme réoriente l'os vers sa nouvelle queue.
        self.skeleton.clone_from(&start.skeleton);
        let old_world = start.skeleton.rest_world();
        let mut world = old_world.clone();
        for (i, w) in world.iter_mut().enumerate() {
            let length = start.skeleton.bones()[i].length;
            let (head, tail) = head_tail(w, length);
            let (h, t) = (m.transform_point(head), m.transform_point(tail));
            let rotation = (m.rotation * w.rotation).normalize();
            w.rotation = (rotation_arc(rotation * Vec3::Y, t - h) * rotation).normalize();
            w.translation = h;
            let old = (tail - head).length();
            if old > 1e-8 {
                self.skeleton.bone_mut(BoneId(i as u32)).length = (length * (t - h).length() / old).max(1e-4);
            }
        }
        self.skeleton.set_rest_world(&world);

        // Translations des deltas de pose : exprimées dans le repère de
        // l'os, elles ne changent qu'avec l'échelle.
        self.pose.clone_from(&start.pose);
        self.clip = start.clip.clone();
        if m.scale != Vec3::ONE {
            let scale_pose = |pose: &mut Pose| {
                for (i, delta) in pose.bones.iter_mut().enumerate() {
                    if delta.translation == Vec3::ZERO || i >= world.len() {
                        continue;
                    }
                    let v = old_world[i].rotation * delta.translation;
                    delta.translation = world[i].rotation.inverse() * (m.rotation * (m.scale * v));
                }
            };
            scale_pose(&mut self.pose);
            if !self.clip.is_empty() {
                self.clip_mut().poses_mut().for_each(scale_pose);
            }
        }

        self.limbs.clone_from(&start.limbs);
        for limb in &mut self.limbs {
            limb.target = m.transform_point(limb.target);
            limb.pole = m.transform_point(limb.pole);
        }
        self.pins.clone_from(&start.pins);
        for pin in &mut self.pins {
            pin.target = m.transform_point(pin.target);
        }

        self.transform_objects_from(start, m);
        self.skin = start.skin.clone();
        self.revisions = start.revisions;
        self.revisions.mesh = next_revision();
        self.touch_skeleton();
        if bound {
            self.revisions.bound_to = (self.revisions.skeleton, self.revisions.mesh);
        }
    }

    /// Positions et normales de `start` transformées, écrites dans le
    /// maillage courant (copié une seule fois s'il est partagé).
    fn transform_mesh(&mut self, start: &Rig, m: &Transform) {
        let src = &*start.mesh;
        let same_layout = self.mesh.positions.len() == src.positions.len()
            && self.mesh.normals.len() == src.normals.len()
            && self.mesh.indices.len() == src.indices.len();
        if !same_layout {
            self.mesh = start.mesh.clone();
        }
        let mesh = Arc::make_mut(&mut self.mesh);
        for (dst, p) in mesh.positions.iter_mut().zip(&src.positions) {
            *dst = m.transform_point(*p);
        }
        // Normales : inverse transposée de rotation × échelle.
        let inv_scale = m.scale.recip();
        for (dst, n) in mesh.normals.iter_mut().zip(&src.normals) {
            *dst = (m.rotation * (*n * inv_scale)).normalize_or(*n);
        }
    }
}
