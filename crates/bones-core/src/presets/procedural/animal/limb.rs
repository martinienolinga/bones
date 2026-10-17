//! Patte d'animal (membre digitigrade) : point d'appui au sol, IK à deux
//! os sur les segments proximaux, segments distaux (métatarse, doigts)
//! orientés autour de l'appui.

use glam::{Quat, Vec3};

use crate::BoneId;
use crate::presets::procedural::ctx::{Body, Rigging};
use crate::presets::roles::{Chain, Lr};

pub(crate) struct Limb {
    pub chain: Vec<BoneId>,
    pub side: Lr,
    /// Patte avant (bras) ou arrière (jambe).
    pub front: bool,
    /// Point d'appui au repos (monde) : bout du troisième segment, ou du
    /// deuxième pour une patte à deux segments.
    pub contact: Vec3,
    /// Sens de flexion de l'articulation intermédiaire au repos (monde).
    pub bend: Vec3,
    /// Longueur des deux premiers segments.
    pub reach: f32,
}

impl Limb {
    pub fn new(rig: &Rigging, chain: Chain, side: Lr, front: bool) -> Option<Self> {
        let bones = rig.chain(chain);
        if bones.len() < 2 {
            return None;
        }
        let end = bones[2.min(bones.len() - 1)];
        let contact = rig.tail(end);
        let (a, b) = (rig.head(bones[0]), rig.head(bones[1]));
        let c = rig.tail(bones[1]);
        let axis = (c - a).normalize_or(-rig.frame.up);
        let off = b - a;
        let fallback = rig.w(if front { -Vec3::Z } else { Vec3::Z });
        let bend = (off - axis * off.dot(axis)).try_normalize().unwrap_or(fallback);
        Some(Self {
            reach: rig.len(bones[0]) + rig.len(bones[1]),
            chain: bones,
            side,
            front,
            contact,
            bend,
        })
    }

    /// Repos du point d'attache (tête du premier segment).
    pub fn attach(&self, rig: &Rigging) -> Vec3 {
        rig.head(self.chain[0])
    }

    /// Pose la patte : appui en `paw` (monde), segments distaux tournés de
    /// `q` (monde) autour de l'appui, doigts à plat selon `toe_flat`.
    pub fn place(&self, body: &mut Body, paw: Vec3, q: Quat, toe_flat: f32) {
        let rig = body.rig;
        let c = &self.chain;
        let parent = rig.skeleton.bone(c[0]).parent();
        let carried = parent.map_or(Quat::IDENTITY, |p| {
            body.rotation(p) * rig.rest[p.index()].rotation.inverse()
        });
        let attach = body.head(c[0]);
        if c.len() == 2 {
            let pole = (attach + paw) * 0.5 + carried * self.bend * self.reach;
            body.two_bone(c[0], c[1], paw, pole);
            return;
        }
        let hock = paw - q * (self.contact - rig.head(c[2]));
        let pole = (attach + hock) * 0.5 + carried * self.bend * self.reach;
        body.two_bone(c[0], c[1], hock, pole);
        body.set_rotation(c[2], q * rig.rest[c[2].index()].rotation);
        for &toe in &c[3..] {
            let follow = body.rotation(toe);
            let flat = rig.rest[toe.index()].rotation;
            body.set_rotation(toe, follow.slerp(flat, toe_flat.clamp(0.0, 1.0)));
        }
    }
}
