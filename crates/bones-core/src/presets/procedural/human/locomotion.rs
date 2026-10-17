//! Marche et course humaines (marche, jogging, course, sprint, pas de
//! loup). Les pieds suivent des trajectoires planifiées et restent plantés
//! au sol pendant l'appui (déroulé talon → plante → pointe), les jambes
//! sont résolues en IK à deux os, le bassin oscille (pendule inversé à la
//! marche, ressort à la course : Cavagna 1977), la colonne contre-tourne,
//! les bras balancent en opposition avec un léger retard.
//!
//! Longueur de foulée et cadence suivent la vitesse adimensionnée
//! v̂ = v / √(g·L) (nombre de Froude, Alexander 1984) : S ≈ 2,4·L·√v̂.

use std::f32::consts::PI;

use glam::{Quat, Vec3};

use super::rig::Human;
use crate::presets::error::PresetError;
use crate::presets::params::{ParamSpec, Params};
use crate::presets::procedural::Motion;
use crate::presets::procedural::ctx::{Body, Rigging};
use crate::presets::procedural::curves::{bump, cwave, envelope, minjerk, periodic, smooth};
use crate::presets::roles::Lr;

pub(crate) const PARAMS: &[ParamSpec] = &[
    ParamSpec::new("speed", "Speed (m/s)", 0.3, 9.0, 1.4),
    ParamSpec::new("stride", "Stride length", 0.6, 1.5, 1.0),
    ParamSpec::new("bounce", "Bounce", 0.0, 2.0, 1.0),
    ParamSpec::new("arms", "Arm swing", 0.0, 2.0, 1.0),
    ParamSpec::new("lean", "Lean", -1.0, 1.0, 0.0),
    ParamSpec::new("crouch", "Crouch", 0.0, 1.0, 0.0),
    ParamSpec::toggle("run", "Run (flight phase)", false),
];

pub(crate) struct Locomotion<'r, 'a> {
    h: Human<'r, 'a>,
    run: bool,
    /// Distance parcourue par cycle (deux pas), en mètres.
    stride: f32,
    period: f32,
    duty: f32,
    strike: f32,
    toe_off: f32,
    heel_flat: f32,
    heel_off: f32,
    lift: f32,
    bob: f32,
    sway: f32,
    yaw: f32,
    list: f32,
    lean: f32,
    crouch: f32,
    arm_swing: f32,
    elbow: f32,
    elbow_swing: f32,
    width: f32,
    /// Abaissement minimal du bassin.
    base_drop: f32,
    /// Abaissement du bassin sur un cycle (voir [`Self::drop_curve`]).
    drops: Vec<f32>,
}

impl<'r, 'a> Locomotion<'r, 'a> {
    pub fn new(rig: &'r Rigging<'a>, p: &Params) -> Result<Self, PresetError> {
        let h = Human::new(rig)?;
        let get = |k| p.get(PARAMS, k);
        let leg = (h.legs[0].length + h.legs[1].length) * 0.5;
        let v = get("speed");
        let vh = v / (9.81 * leg).sqrt();
        let run = p.flag(PARAMS, "run");
        let stride = 2.4 * leg * vh.sqrt() * get("stride");
        let (bounce, arms, crouch) = (get("bounce"), get("arms"), get("crouch"));
        let mut m = if run {
            Self {
                h,
                run,
                stride,
                period: stride / v,
                duty: (0.47 - 0.09 * vh).clamp(0.22, 0.45),
                strike: (0.2 - 0.08 * vh).clamp(-0.05, 0.2),
                toe_off: 0.95,
                heel_flat: 0.1,
                heel_off: 0.35,
                lift: leg * (0.18 + 0.12 * vh).min(0.6),
                bob: 0.04 * leg * bounce,
                sway: 0.008 * leg,
                yaw: 0.14,
                list: 0.05,
                lean: 0.1 + 0.05 * vh + 0.2 * get("lean"),
                crouch,
                arm_swing: (0.5 + 0.12 * vh) * arms,
                elbow: 1.45,
                elbow_swing: 0.25,
                width: 0.55,
                base_drop: 0.0,
                drops: Vec::new(),
            }
        } else {
            Self {
                h,
                run,
                stride,
                period: stride / v,
                duty: (0.68 - 0.13 * vh).clamp(0.55, 0.68),
                strike: 0.3,
                toe_off: 1.0,
                heel_flat: 0.15,
                heel_off: 0.5,
                lift: 0.09 * leg,
                bob: 0.022 * leg * bounce,
                sway: 0.025 * leg,
                yaw: 0.09,
                list: 0.08,
                lean: 0.03 + 0.2 * get("lean") + 0.35 * crouch,
                crouch,
                arm_swing: 0.3 * arms * (1.0 - 0.6 * crouch),
                elbow: 0.2 + 0.6 * crouch,
                elbow_swing: 0.2,
                width: 0.85,
                base_drop: 0.0,
                drops: Vec::new(),
            }
        };
        m.base_drop = 0.01 * m.h.hip_height + crouch * 0.25 * leg;
        m.drops = m.drop_curve();
        Ok(m)
    }

    /// Bassin : déplacement canonique (sans abaissement) et rotation.
    fn pelvis(&self, u: f32) -> (Vec3, f32, f32) {
        let mid = u - self.duty * 0.5;
        let bob = if self.run { -self.bob } else { self.bob } * cwave(2.0 * mid);
        let offset = Vec3::new(self.sway * cwave(mid), bob, self.stride * u);
        (offset, -self.yaw * cwave(u), self.list * cwave(mid))
    }

    fn pelvis_rotation(&self, yaw: f32, roll: f32) -> Quat {
        let pitch = if self.run { 0.08 } else { 0.02 };
        let rig = self.h.rig;
        rig.basis * Quat::from_rotation_y(yaw) * Quat::from_rotation_z(roll) * Quat::from_rotation_x(pitch) * rig.basis.inverse()
    }

    /// Cheville visée (monde), tangage du pied et orteils à plat (0..1).
    fn foot(&self, side: Lr, u: f32) -> (Vec3, f32, f32) {
        let offset = if side == Lr::Left { 0.0 } else { 0.5 };
        let p = u + offset;
        let k = p.floor();
        let phi = p - k;
        let flat = |k: f32| self.flat_ankle(side, k, offset);
        if phi < self.duty {
            let s = phi / self.duty;
            let pitch = if s < self.heel_flat {
                self.strike * (1.0 - smooth(s / self.heel_flat))
            } else if s > self.heel_off {
                -self.toe_off * smooth((s - self.heel_off) / (1.0 - self.heel_off)).powf(1.3)
            } else {
                0.0
            };
            let toe = if s < self.heel_flat { 0.0 } else { 1.0 };
            (self.h.rolled_ankle(side, flat(k), pitch, 0.0), pitch, toe)
        } else {
            let w = (phi - self.duty) / (1.0 - self.duty);
            let rig = self.h.rig;
            let start = self.h.rolled_ankle(side, flat(k), -self.toe_off, 0.0);
            let end = self.h.rolled_ankle(side, flat(k + 1.0), self.strike, 0.0);
            let up = rig.frame.up;
            let (h0, h1) = (rig.height(start), rig.height(end));
            let along = start + (end - start) * minjerk(w);
            let height = h0 + (h1 - h0) * smooth(w) + self.lift * bump(w, if self.run { 0.4 } else { 0.35 });
            let ankle = along + up * (height - rig.height(along));
            let pitch = -self.toe_off + (self.strike + self.toe_off) * smooth(w) + 0.15 * (PI * w).sin();
            (ankle, pitch, 1.0 - smooth(w / 0.3))
        }
    }

    /// Cheville à plat au pas `k` : sous le bassin au milieu de l'appui.
    fn flat_ankle(&self, side: Lr, k: f32, offset: f32) -> Vec3 {
        let h = &self.h;
        let rig = h.rig;
        let leg = h.leg_of(side);
        let left = rig.w(Vec3::X);
        let lateral = (leg.ankle - rig.head(h.root)).dot(left);
        // Le bassin passe au-dessus de la cheville un peu avant le milieu de
        // l'appui (le déroulé sur la pointe fait encore avancer la cheville).
        let mid = if self.run { 0.5 } else { 0.45 };
        leg.ankle + left * lateral * (self.width - 1.0) + rig.w(Vec3::Z) * self.stride * (k + self.duty * mid - offset)
    }

    /// Abaissement imposé par la portée à l'instant `u` : chaque cheville
    /// doit rester atteignable (jambe tendue à 99 % au plus).
    fn reach_drop(&self, u: f32) -> f32 {
        let h = &self.h;
        let rig = h.rig;
        let up = rig.frame.up;
        let root = rig.head(h.root);
        let (offset, yaw, roll) = self.pelvis(u);
        let r = self.pelvis_rotation(yaw, roll);
        let mut drop = self.base_drop;
        for leg in &h.legs {
            let hip = root + rig.w(offset) + r * (rig.head(leg.thigh) - root);
            let (ankle, _, _) = self.foot(leg.side, u);
            let d = hip - ankle;
            let dv = d.dot(up);
            let dh = (d - up * dv).length();
            let max = 0.99 * leg.length;
            drop = drop.max(if dh < max { dv - (max * max - dh * dh).sqrt() } else { dv });
        }
        drop
    }

    /// Courbe d'abaissement du bassin sur un cycle : enveloppe de la portée
    /// à accélération verticale bornée, d'où des arcs doux comme ceux du
    /// pendule inversé.
    fn drop_curve(&self) -> Vec<f32> {
        let n = DROP_SAMPLES;
        let need: Vec<f32> = (0..n).map(|i| self.reach_drop(i as f32 / n as f32) + 0.002).collect();
        envelope(&need, self.period / n as f32, if self.run { 6.0 } else { 3.5 })
    }
}

/// Échantillons par cycle de la courbe d'abaissement du bassin.
const DROP_SAMPLES: usize = 128;

impl Motion for Locomotion<'_, '_> {
    fn duration(&self) -> f32 {
        self.period
    }

    fn pose(&self, u: f32, body: &mut Body) {
        let h = &self.h;
        body.reset();
        let (mut offset, yaw, roll) = self.pelvis(u);
        offset.y -= periodic(&self.drops, u);
        let pitch = if self.run { 0.08 } else { 0.02 } + 0.15 * self.crouch;
        h.pelvis(body, offset, yaw, pitch, roll);
        let flex = if self.run { 0.04 * cwave(2.0 * (u - self.duty * 0.5)) } else { 0.0 };
        h.bend_spine(body, self.lean + flex - pitch, -1.5 * yaw, -0.8 * roll);
        let fist = if self.run { 0.55 } else { 0.25 };
        for side in Lr::BOTH {
            h.curl(body, side, fist);
        }
        body.fk();
        for side in Lr::BOTH {
            let (ankle, pitch, toe) = self.foot(side, u);
            h.place_foot(body, side, ankle, pitch, 0.0, toe);
        }
        for side in Lr::BOTH {
            let phase = if side == Lr::Left { 0.0 } else { 0.5 };
            let s = -cwave(u + phase - 0.04);
            let swing = self.arm_swing * s + 0.05;
            let abduct = if self.run { 0.16 } else { 0.1 };
            let elbow = self.elbow + self.elbow_swing * s.max(0.0);
            h.shrug(body, side, 0.0, 0.04 * s);
            body.fk();
            h.swing_arm(body, side, abduct, swing, elbow, 0.12);
        }
        h.look(body, 0.0, 0.0, 0.0);
    }
}
