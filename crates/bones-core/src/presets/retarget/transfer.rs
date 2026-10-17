//! Transfert d'une pose source vers la cible : rotations monde relatives
//! au repos dans le repère du corps, translation du bassin à l'échelle,
//! puis IK des appuis.

use glam::{Quat, Vec3};

use super::RetargetOptions;
use super::mapping::{Mapping, mirror_quat, mirror_vec};
use crate::presets::procedural::ctx::{Body, Rigging};
use crate::presets::roles::{Chain, Lr, RigType, RoleMap};
use crate::{BoneId, Pose, Skeleton, Transform, head_tail};

pub(crate) struct Retargeter<'a> {
    src: &'a Skeleton,
    src_rest: Vec<Transform>,
    pub rig: Rigging<'a>,
    mapping: Mapping,
    /// Repères du corps (canonique → monde) source et cible.
    bs: Quat,
    bt: Quat,
    /// Rapport des tailles (hauteurs de hanches) cible / source.
    scale: f32,
    options: RetargetOptions,
    /// Membres posés par IK : (chaîne cible, chaîne source).
    limbs: Vec<(Vec<BoneId>, Vec<BoneId>)>,
}

impl<'a> Retargeter<'a> {
    pub fn new(
        source: (&'a Skeleton, &'a RoleMap),
        target: (&'a Skeleton, &'a RoleMap),
        mapping: Mapping,
        options: RetargetOptions,
    ) -> Self {
        let (src, src_roles) = source;
        let (tgt, tgt_roles) = target;
        let mut limbs = Vec::new();
        if options.foot_ik {
            let quad = matches!(tgt_roles.rig_type, RigType::Quadruped | RigType::MultiLegged);
            for chain in tgt_roles.chains() {
                let wanted = match chain {
                    Chain::Leg(_) | Chain::LegPair(..) => true,
                    Chain::Arm(_) => quad,
                    _ => false,
                };
                let from = if options.mirror { chain.mirror() } else { chain };
                let (t, s) = (tgt_roles.chain(chain), src_roles.chain(from));
                if wanted && t.len() >= 2 && s.len() >= 2 {
                    limbs.push((t, s));
                }
            }
        }
        Self {
            src,
            src_rest: src.rest_world(),
            rig: Rigging::new(tgt, tgt_roles),
            mapping,
            bs: src_roles.frame.basis(),
            bt: tgt_roles.frame.basis(),
            scale: (tgt_roles.frame.size / src_roles.frame.size.max(1e-4)).clamp(1e-3, 1e3),
            options,
            limbs,
        }
    }

    pub fn mapping(&self) -> &Mapping {
        &self.mapping
    }

    /// Rotation monde source (relative au repos) → rotation monde cible.
    fn carry(&self, q: Quat) -> Quat {
        let c = self.bs.inverse() * q * self.bs;
        let c = if self.options.mirror { mirror_quat(c) } else { c };
        (self.bt * c * self.bt.inverse()).normalize()
    }

    /// Déplacement monde source → déplacement monde cible (à l'échelle).
    fn carry_vec(&self, v: Vec3) -> Vec3 {
        let c = self.bs.inverse() * v;
        self.bt * if self.options.mirror { mirror_vec(c) } else { c } * self.scale
    }

    fn src_tail(&self, world: &[Transform], b: BoneId) -> Vec3 {
        head_tail(&world[b.index()], self.src.bone(b).length).1
    }

    /// Pose cible pour une pose source (transformations monde `src_world`).
    /// `extra` : décalage monde ajouté au déplacement source (boucles) ;
    /// `base` : pose des os sans correspondance.
    pub fn pose(&self, src_world: &[Transform], extra: Vec3, base: &Pose, body: &mut Body) {
        let tgt = self.rig.skeleton;
        let rest = &self.rig.rest;
        let delta = |x: BoneId| {
            (src_world[x.index()].rotation * self.src_rest[x.index()].rotation.inverse()).normalize()
        };
        for (t, bone) in tgt.bones().iter().enumerate() {
            let parent = bone
                .parent()
                .map_or(Transform::IDENTITY, |p| body.world[p.index()]);
            let local = &bone.rest;
            let keep = base.get(BoneId(t as u32));
            let pose = match self.mapping.samples[t] {
                None => keep,
                Some(s) => {
                    let (qa, qb) = (delta(s.a), delta(s.b));
                    let qb = if qa.dot(qb) < 0.0 { -qb } else { qb };
                    let ds = if s.w > 0.0 { qa.slerp(qb, s.w) } else { qa };
                    let desired = self.carry(ds) * s.align * rest[t].rotation;
                    let basis = (parent.rotation * local.rotation).normalize();
                    let mut translation = keep.translation;
                    if s.translate && self.options.root_motion {
                        let a = s.a.index();
                        let moved = src_world[a].translation + extra - self.src_rest[a].translation;
                        let at = rest[t].translation + self.carry_vec(moved);
                        let p = parent.inverse().transform_point(at);
                        translation = local.rotation.inverse() * (p - local.translation) / local.scale;
                    }
                    Transform {
                        translation,
                        rotation: (basis.inverse() * desired).normalize(),
                        scale: keep.scale,
                    }
                }
            };
            body.pose.bones[t] = pose;
            body.world[t] = parent.mul(&local.mul(&pose));
        }
        for (t, s) in &self.limbs {
            self.plant(src_world, extra, t, s, body);
        }
    }

    /// Replace l'appui d'un membre (bout du 3ᵉ segment) à la position source
    /// mise à l'échelle, en gardant l'orientation des segments distaux.
    fn plant(&self, src_world: &[Transform], extra: Vec3, t: &[BoneId], s: &[BoneId], body: &mut Body) {
        let rig = &self.rig;
        let (kt, ks) = (2.min(t.len() - 1), 2.min(s.len() - 1));
        let src_rest_contact = head_tail(&self.src_rest[s[ks].index()], self.src.bone(s[ks]).length).1;
        let moved = self.src_tail(src_world, s[ks]) + extra - src_rest_contact;
        let contact = rig.tail(t[kt]) + self.carry_vec(moved);
        let target = if t.len() >= 3 {
            let q = body.rotation(t[2]) * rig.rest[t[2].index()].rotation.inverse();
            contact - q * (rig.tail(t[2]) - rig.head(t[2]))
        } else {
            contact
        };
        let saved: Vec<Quat> = t[2.min(t.len())..].iter().map(|b| body.rotation(*b)).collect();
        let (hip, knee) = (body.head(t[0]), body.head(t[1]));
        let side = t
            .first()
            .and_then(|b| self.rig.roles.role(*b))
            .and_then(|r| r.chain.side());
        let fallback = rig.w(Vec3::Z) + rig.w(Vec3::X) * side.map_or(0.0, Lr::sign) * 0.1;
        let bend = (knee - (hip + target) * 0.5).try_normalize().unwrap_or(fallback);
        let reach = rig.len(t[0]) + rig.len(t[1]);
        body.two_bone(t[0], t[1], target, knee + bend * reach);
        for (b, r) in t[2.min(t.len())..].iter().zip(saved) {
            body.set_rotation(*b, r);
        }
    }
}
