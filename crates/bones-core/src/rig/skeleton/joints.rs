//! Édition géométrique des os de repos (mode édition) : tête, déplacement
//! d'un os entier, roulis, longueur, inversion, alignement, connexion.
//!
//! Chaque opération modifie des transformations monde puis recalcule les
//! locales : les os non concernés ne bougent pas. Comme dans Blender, une
//! articulation connectée (queue du parent = tête des enfants connectés) se
//! déplace d'un bloc.

use std::f32::consts::{PI, TAU};

use glam::{Quat, Vec3};

use super::{BoneId, Skeleton, head_tail};
use crate::Transform;
use crate::math::rotation_arc;

/// Replace un os entre `head` et `tail` en conservant son roulis.
pub(super) fn retarget(world: &mut Transform, length: &mut f32, head: Vec3, tail: Vec3) {
    let old_dir = world.rotation * Vec3::Y;
    world.rotation = (rotation_arc(old_dir, tail - head) * world.rotation).normalize();
    world.translation = head;
    *length = (tail - head).length().max(1e-4);
}

impl Skeleton {
    /// Tête et queue de repos d'un os, en espace monde.
    pub fn rest_head_tail(&self, id: BoneId) -> (Vec3, Vec3) {
        head_tail(&self.rest_world_of(id), self.bone(id).length)
    }

    /// Déplace l'articulation formée par la queue de `parent` et la tête de
    /// ses enfants connectés ; leurs autres extrémités restent en place.
    pub(super) fn move_joint(&mut self, world: &mut [Transform], parent: BoneId, p: Vec3) {
        let connected: Vec<BoneId> = self.children(parent).filter(|c| self.is_connected(*c)).collect();
        let (head, _) = head_tail(&world[parent.index()], self.bones[parent.index()].length);
        retarget(
            &mut world[parent.index()],
            &mut self.bones[parent.index()].length,
            head,
            p,
        );
        for c in connected {
            let (_, tail) = head_tail(&world[c.index()], self.bones[c.index()].length);
            retarget(&mut world[c.index()], &mut self.bones[c.index()].length, p, tail);
        }
    }

    /// Déplace la tête d'un os, sa queue restant en place. Si l'os est
    /// connecté, la queue du parent (et les têtes des os frères connectés)
    /// suivent.
    pub fn set_head_world(&mut self, id: BoneId, p: Vec3) {
        let mut world = self.rest_world();
        match self.bone(id).parent.filter(|_| self.is_connected(id)) {
            Some(parent) => self.move_joint(&mut world, parent, p),
            None => {
                let (_, tail) = head_tail(&world[id.index()], self.bones[id.index()].length);
                retarget(
                    &mut world[id.index()],
                    &mut self.bones[id.index()].length,
                    p,
                    tail,
                );
            }
        }
        self.set_rest_world(&world);
    }

    /// Déplace un os entier. Les articulations connectées suivent : queue du
    /// parent à la tête, têtes des enfants connectés à la queue.
    pub fn translate_world(&mut self, id: BoneId, delta: Vec3) {
        let mut world = self.rest_world();
        let (head, tail) = head_tail(&world[id.index()], self.bones[id.index()].length);
        let children: Vec<BoneId> = self.children(id).filter(|c| self.is_connected(*c)).collect();
        if let Some(parent) = self.bone(id).parent.filter(|_| self.is_connected(id)) {
            self.move_joint(&mut world, parent, head + delta);
        }
        // Rétablit la direction (et donc le roulis) d'origine de l'os.
        retarget(
            &mut world[id.index()],
            &mut self.bones[id.index()].length,
            head + delta,
            tail + delta,
        );
        for c in children {
            let (_, child_tail) = head_tail(&world[c.index()], self.bones[c.index()].length);
            retarget(
                &mut world[c.index()],
                &mut self.bones[c.index()].length,
                tail + delta,
                child_tail,
            );
        }
        self.set_rest_world(&world);
    }

    /// Tourne un os autour de sa tête, roulis compris (rotation monde). Les
    /// enfants connectés suivent par leur tête en gardant leur queue en
    /// place ; les autres os ne bougent pas.
    pub fn rotate_bone_world(&mut self, id: BoneId, rotation: Quat) {
        let mut world = self.rest_world();
        let children: Vec<BoneId> = self.children(id).filter(|c| self.is_connected(*c)).collect();
        let w = &mut world[id.index()];
        w.rotation = (rotation * w.rotation).normalize();
        let (_, tail) = head_tail(w, self.bones[id.index()].length);
        for c in children {
            let (_, child_tail) = head_tail(&world[c.index()], self.bones[c.index()].length);
            retarget(
                &mut world[c.index()],
                &mut self.bones[c.index()].length,
                tail,
                child_tail,
            );
        }
        self.set_rest_world(&world);
    }

    /// Translation rigide d'un ensemble d'os (sans toucher aux autres).
    pub fn offset_world(&mut self, ids: &[BoneId], delta: Vec3) {
        let mut world = self.rest_world();
        for id in ids {
            world[id.index()].translation += delta;
        }
        self.set_rest_world(&world);
    }

    /// Roulis : rotation de l'os autour de son propre axe, mesurée depuis
    /// l'orientation de référence (rotation minimale de +Y vers l'os), en
    /// radians dans ]−π, π].
    pub fn roll(&self, id: BoneId) -> f32 {
        let rotation = self.rest_world_of(id).rotation;
        let reference = rotation_arc(Vec3::Y, rotation * Vec3::Y);
        let twist = reference.inverse() * rotation;
        let angle = 2.0 * twist.y.atan2(twist.w);
        (angle + PI).rem_euclid(TAU) - PI
    }

    /// Fixe le roulis d'un os ; ses enfants ne bougent pas.
    pub fn set_roll(&mut self, id: BoneId, roll: f32) {
        let mut world = self.rest_world();
        let w = &mut world[id.index()];
        let reference = rotation_arc(Vec3::Y, w.rotation * Vec3::Y);
        w.rotation = (reference * Quat::from_rotation_y(roll)).normalize();
        self.set_rest_world(&world);
    }

    /// Change la longueur en gardant tête et direction.
    pub fn set_length(&mut self, id: BoneId, length: f32) {
        let (head, tail) = self.rest_head_tail(id);
        let dir = (tail - head).normalize_or(Vec3::Y);
        self.set_tail_world(id, head + dir * length.max(1e-4));
    }

    /// Oriente l'os selon `axis` en gardant tête et longueur.
    pub fn align(&mut self, id: BoneId, axis: Vec3) {
        let (head, _) = self.rest_head_tail(id);
        let length = self.bone(id).length;
        self.set_tail_world(id, head + axis.normalize_or(Vec3::Y) * length);
    }

    /// Inverse l'os (tête et queue échangées). Les enfants restent en place
    /// et ne sont plus connectés.
    pub fn flip(&mut self, id: BoneId) {
        let mut world = self.rest_world();
        let (head, tail) = head_tail(&world[id.index()], self.bones[id.index()].length);
        retarget(
            &mut world[id.index()],
            &mut self.bones[id.index()].length,
            tail,
            head,
        );
        self.set_rest_world(&world);
    }

    /// Connecte l'os à son parent (sa tête rejoint la queue du parent, sa
    /// queue reste en place) ou le déconnecte (il peut alors être déplacé
    /// librement, sans entraîner son parent).
    pub fn set_connected(&mut self, id: BoneId, connect: bool) {
        let Some(parent) = self.bone(id).parent else {
            return;
        };
        self.bones[id.index()].detached = !connect;
        if connect {
            let mut world = self.rest_world();
            let (_, parent_tail) = head_tail(&world[parent.index()], self.bones[parent.index()].length);
            let (_, tail) = head_tail(&world[id.index()], self.bones[id.index()].length);
            retarget(
                &mut world[id.index()],
                &mut self.bones[id.index()].length,
                parent_tail,
                tail,
            );
            self.set_rest_world(&world);
        }
    }
}
