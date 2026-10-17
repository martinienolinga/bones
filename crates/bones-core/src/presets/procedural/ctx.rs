//! Contexte des générateurs : rôles et repère du squelette cible
//! ([`Rigging`]), et pose de travail avec ses transformations monde
//! ([`Body`]) : rotations autour d'axes canoniques, visée, IK à deux os.
//!
//! Repère canonique : X à gauche, Y en haut, Z vers l'avant. Une rotation
//! « canonique » d'un os est exprimée dans le repère de son parent au
//! repos : tourner la cuisse autour de l'axe gauche la lève vers l'avant,
//! quelle que soit l'orientation de ses axes locaux.

use glam::{Quat, Vec3};

use crate::math::rotation_arc;
use crate::presets::roles::{BodyFrame, Chain, RoleMap};
use crate::{BoneId, Pose, Skeleton, Transform, head_tail};

/// Squelette cible vu par un générateur.
pub(crate) struct Rigging<'a> {
    pub skeleton: &'a Skeleton,
    pub roles: &'a RoleMap,
    /// Transformations monde au repos.
    pub rest: Vec<Transform>,
    /// Canonique → monde.
    pub basis: Quat,
    pub frame: BodyFrame,
}

impl<'a> Rigging<'a> {
    pub fn new(skeleton: &'a Skeleton, roles: &'a RoleMap) -> Self {
        Self {
            skeleton,
            roles,
            rest: skeleton.rest_world(),
            basis: roles.frame.basis(),
            frame: roles.frame,
        }
    }

    pub fn bone(&self, chain: Chain, index: u8) -> Option<BoneId> {
        self.roles.bone(chain, index)
    }

    pub fn chain(&self, chain: Chain) -> Vec<BoneId> {
        self.roles.chain(chain)
    }

    /// Vecteur canonique → monde.
    pub fn w(&self, v: Vec3) -> Vec3 {
        self.basis * v
    }

    /// Vecteur monde → canonique.
    pub fn c(&self, v: Vec3) -> Vec3 {
        self.basis.inverse() * v
    }

    pub fn head(&self, b: BoneId) -> Vec3 {
        self.rest[b.index()].translation
    }

    pub fn tail(&self, b: BoneId) -> Vec3 {
        head_tail(&self.rest[b.index()], self.skeleton.bone(b).length).1
    }

    pub fn len(&self, b: BoneId) -> f32 {
        (self.tail(b) - self.head(b)).length()
    }

    /// Direction monde de repos de l'os.
    pub fn dir(&self, b: BoneId) -> Vec3 {
        (self.tail(b) - self.head(b)).normalize_or(Vec3::Y)
    }

    /// Hauteur d'un point au-dessus du sol.
    pub fn height(&self, p: Vec3) -> f32 {
        (p - self.frame.up * self.frame.ground).dot(self.frame.up)
    }

    /// Projection d'un point sur le sol.
    pub fn on_ground(&self, p: Vec3) -> Vec3 {
        p - self.frame.up * self.height(p)
    }
}

/// Pose en construction et ses transformations monde.
pub(crate) struct Body<'r, 'a> {
    pub rig: &'r Rigging<'a>,
    pub pose: Pose,
    pub world: Vec<Transform>,
}

impl<'r, 'a> Body<'r, 'a> {
    pub fn new(rig: &'r Rigging<'a>) -> Self {
        Self {
            rig,
            pose: Pose::rest(rig.skeleton),
            world: rig.rest.clone(),
        }
    }

    /// Retour à la pose de repos.
    pub fn reset(&mut self) {
        self.pose.bones.fill(Transform::IDENTITY);
        self.world.clone_from(&self.rig.rest);
    }

    /// Rotation de `angle` (radians) autour d'un axe canonique, appliquée
    /// par-dessus la rotation actuelle de l'os (pose seulement : appeler
    /// [`Self::fk`] ensuite).
    pub fn turn(&mut self, bone: Option<BoneId>, axis: Vec3, angle: f32) {
        if let Some(b) = bone
            && angle != 0.0
        {
            let q = Quat::from_axis_angle(self.rig.w(axis).normalize_or(Vec3::Y), angle);
            self.turn_world(b, q);
        }
    }

    /// Rotation exprimée en monde (au repos du parent), cumulée.
    pub fn turn_world(&mut self, bone: BoneId, q: Quat) {
        let rest = self.rig.rest[bone.index()].rotation;
        let local = (rest.inverse() * q * rest).normalize();
        let delta = &mut self.pose.bones[bone.index()];
        delta.rotation = (local * delta.rotation).normalize();
    }

    /// Déplacement (canonique, en mètres) d'un os dont le parent est immobile.
    pub fn shift(&mut self, bone: Option<BoneId>, offset: Vec3) {
        let Some(b) = bone else { return };
        let rest = &self.rig.rest[b.index()];
        let local = rest.rotation.inverse() * self.rig.w(offset) / rest.scale;
        self.pose.bones[b.index()].translation += local;
    }

    /// Recalcule toutes les transformations monde depuis la pose.
    pub fn fk(&mut self) {
        self.refresh(BoneId(0));
    }

    /// Recalcule les transformations monde des os d'indice ≥ `from`.
    pub fn refresh(&mut self, from: BoneId) {
        let skeleton = self.rig.skeleton;
        for i in from.index()..skeleton.len() {
            let bone = &skeleton.bones()[i];
            let local = bone.rest.mul(&self.pose.bones[i]);
            self.world[i] = match bone.parent() {
                Some(p) => self.world[p.index()].mul(&local),
                None => local,
            };
        }
    }

    pub fn head(&self, b: BoneId) -> Vec3 {
        self.world[b.index()].translation
    }

    pub fn tail(&self, b: BoneId) -> Vec3 {
        head_tail(&self.world[b.index()], self.rig.skeleton.bone(b).length).1
    }

    pub fn rotation(&self, b: BoneId) -> Quat {
        self.world[b.index()].rotation
    }

    /// Donne à l'os la rotation monde `rotation` (descendants mis à jour).
    pub fn set_rotation(&mut self, b: BoneId, rotation: Quat) {
        let bone = self.rig.skeleton.bone(b);
        let parent = bone.parent().map_or(Quat::IDENTITY, |p| self.world[p.index()].rotation);
        let basis = (parent * bone.rest.rotation).normalize();
        self.pose.bones[b.index()].rotation = (basis.inverse() * rotation).normalize();
        self.refresh(b);
    }

    /// Applique une rotation monde autour de la tête de l'os.
    pub fn rotate(&mut self, b: BoneId, q: Quat) {
        let r = (q * self.rotation(b)).normalize();
        self.set_rotation(b, r);
    }

    /// Oriente l'os vers une direction monde (rotation minimale).
    pub fn aim(&mut self, b: BoneId, dir: Vec3) {
        let current = self.tail(b) - self.head(b);
        self.rotate(b, rotation_arc(current, dir));
    }

    /// Torsion autour de l'axe actuel de l'os.
    pub fn twist(&mut self, b: BoneId, angle: f32) {
        let axis = (self.tail(b) - self.head(b)).normalize_or(Vec3::Y);
        self.rotate(b, Quat::from_axis_angle(axis, angle));
    }

    /// IK analytique à deux os : la queue de `lower` vers `target`, le
    /// coude ou le genou vers `pole`. Retourne vrai si la cible est atteinte.
    pub fn two_bone(&mut self, upper: BoneId, lower: BoneId, target: Vec3, pole: Vec3) -> bool {
        let a = self.head(upper);
        let b = self.head(lower);
        let c = self.tail(lower);
        let (la, lb) = ((b - a).length(), (c - b).length());
        if la < 1e-6 || lb < 1e-6 {
            return false;
        }
        let reach = (target - a).length();
        let dist = reach.clamp((la - lb).abs() + 1e-5, la + lb - 1e-5);
        let dir = (target - a).try_normalize().unwrap_or((c - a).normalize_or(Vec3::Y));
        let side = |v: Vec3| (v - dir * v.dot(dir)).try_normalize();
        let bend = side(pole - a)
            .or_else(|| side(b - a))
            .unwrap_or_else(|| dir.any_orthonormal_vector());
        let cos_a = ((la * la + dist * dist - lb * lb) / (2.0 * la * dist)).clamp(-1.0, 1.0);
        let elbow = a + (dir * cos_a + bend * (1.0 - cos_a * cos_a).sqrt()) * la;
        self.rotate(upper, rotation_arc(b - a, elbow - a));
        let (b, c) = (self.head(lower), self.tail(lower));
        self.rotate(lower, rotation_arc(c - b, a + dir * dist - b));
        (reach - dist).abs() < 1e-3
    }
}
