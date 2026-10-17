//! Combat : garde de boxe, enchaînement direct du bras avant + direct du
//! bras arrière, coup de pied de face. La rotation des hanches et du buste
//! précède celle du bras (chaîne cinétique), le pied arrière pivote.

use glam::Vec3;

use super::rig::Human;
use super::stance::Foot;
use crate::presets::error::PresetError;
use crate::presets::params::{ParamSpec, Params};
use crate::presets::procedural::Motion;
use crate::presets::procedural::ctx::{Body, Rigging};
use crate::presets::procedural::curves::{Curve, cwave};
use crate::presets::roles::Lr;

pub(crate) const PUNCH: &[ParamSpec] = &[
    ParamSpec::new("speed", "Speed", 0.5, 2.0, 1.0),
    ParamSpec::new("reach", "Reach", 0.6, 1.0, 0.92),
];

pub(crate) const KICK: &[ParamSpec] = &[
    ParamSpec::new("speed", "Speed", 0.5, 2.0, 1.0),
    ParamSpec::new("height", "Kick height", 0.3, 1.2, 0.85),
];

/// Garde : pied gauche devant, bassin tourné, genoux fléchis.
fn stance(lift: f32) -> [Foot; 2] {
    [
        Foot {
            offset: Vec3::new(0.03, 0.0, 0.17),
            yaw: -0.35,
            ..Foot::default()
        },
        Foot {
            offset: Vec3::new(-0.05, 0.0, -0.17),
            yaw: -0.6,
            pitch: -lift,
            lift: 0.0,
        },
    ]
}

/// Impulsion 0 → 1 → 0 : sortie rapide, maintien bref, retour plus lent.
fn pulse(start: f32, out: f32, hold: f32, back: f32) -> Curve {
    Curve::new(&[(0.0, 0.0), (start, 0.0), (start + out, 1.0), (start + out + hold, 1.0), (start + out + hold + back, 0.0), (10.0, 0.0)])
}

pub(crate) struct Punch<'r, 'a> {
    h: Human<'r, 'a>,
    speed: f32,
    reach: f32,
    jab: Curve,
    cross: Curve,
}

impl<'r, 'a> Punch<'r, 'a> {
    pub fn new(rig: &'r Rigging<'a>, p: &Params) -> Result<Self, PresetError> {
        Ok(Self {
            h: Human::new(rig)?,
            speed: p.get(PUNCH, "speed"),
            reach: p.get(PUNCH, "reach"),
            jab: pulse(0.3, 0.12, 0.05, 0.2),
            cross: pulse(0.95, 0.16, 0.06, 0.28),
        })
    }
}

impl Motion for Punch<'_, '_> {
    fn duration(&self) -> f32 {
        2.4 / self.speed
    }

    fn pose(&self, u: f32, body: &mut Body) {
        let h = &self.h;
        let t = u.rem_euclid(1.0) * 2.4;
        let (jab, cross) = (self.jab.at(t), self.cross.at(t));
        // Les hanches tournent juste avant le bras.
        let hips = self.cross.at(t + 0.05);
        let leg = h.legs[0].length;
        let bounce = 0.008 * leg * cwave(2.0 * u);
        body.reset();
        let yaw = -0.4 + 0.65 * hips - 0.08 * jab;
        h.pelvis(body, Vec3::new(0.0, -0.06 * leg + bounce, 0.03 * leg * hips), yaw, 0.08, 0.0);
        h.bend_spine(body, 0.08 + 0.05 * hips, 0.25 + 0.3 * hips - 0.15 * jab, 0.0);
        for side in Lr::BOTH {
            h.curl(body, side, 1.0);
        }
        body.fk();
        let mut feet = stance(0.35 * hips);
        feet[1].yaw += 0.35 * hips;
        h.stand(body, feet);
        for (side, punch) in [(Lr::Left, jab), (Lr::Right, cross)] {
            let arm = h.arm_of(side).map_or(0.6, |a| a.length);
            let s = side.sign();
            h.guard(body, side, Vec3::new(-s * 0.12, 0.02, self.reach) * arm, punch);
            // Paume vers l'intérieur en garde, poing horizontal à l'impact.
            let palm = h.rig.w(Vec3::new(-s, 0.0, 0.0)).lerp(-h.rig.frame.up, punch);
            h.palm_towards(body, side, palm);
        }
        h.look(body, 0.12, 0.0, 0.0);
    }
}

pub(crate) struct Kick<'r, 'a> {
    h: Human<'r, 'a>,
    speed: f32,
    height: f32,
    chamber: Curve,
    extend: Curve,
}

impl<'r, 'a> Kick<'r, 'a> {
    pub fn new(rig: &'r Rigging<'a>, p: &Params) -> Result<Self, PresetError> {
        Ok(Self {
            h: Human::new(rig)?,
            speed: p.get(KICK, "speed"),
            height: p.get(KICK, "height"),
            chamber: pulse(0.35, 0.3, 0.55, 0.35),
            extend: pulse(0.68, 0.14, 0.08, 0.2),
        })
    }
}

impl Motion for Kick<'_, '_> {
    fn duration(&self) -> f32 {
        2.4 / self.speed
    }

    fn pose(&self, u: f32, body: &mut Body) {
        let h = &self.h;
        let t = u.rem_euclid(1.0) * 2.4;
        let (chamber, extend) = (self.chamber.at(t), self.extend.at(t));
        let leg = h.legs[0].length;
        body.reset();
        let rise = 0.04 * leg * chamber;
        h.pelvis(body, Vec3::new(0.03 * leg * chamber, -0.06 * leg + rise, 0.0), -0.4 + 0.3 * chamber, 0.08 - 0.15 * extend, 0.0);
        h.bend_spine(body, 0.08 - 0.25 * extend, 0.25 - 0.15 * chamber, 0.0);
        for side in Lr::BOTH {
            h.curl(body, side, 1.0);
        }
        body.fk();
        let feet = stance(0.0);
        h.stand(body, feet);
        if chamber > 1e-3 {
            // Pied droit : genou monté (armé), puis jambe tendue vers l'avant.
            let kick = h.leg_of(Lr::Right);
            let hip = body.head(kick.thigh);
            let rest = kick.ankle + h.rig.w(feet[1].offset);
            let armed = hip + h.rig.w(Vec3::new(0.0, -0.55, 0.3) * leg);
            let out = hip + h.rig.w(Vec3::new(0.0, -0.95 + self.height, 0.95) * leg);
            let target = rest.lerp(armed, chamber).lerp(out, extend);
            h.place_foot(body, Lr::Right, target, -0.5 * chamber, -0.6 * (1.0 - chamber), 0.0);
        }
        // Bras droit lancé en arrière pour l'équilibre pendant le coup.
        let arm = h.arm_of(Lr::Right).map_or(0.6, |a| a.length);
        h.guard(body, Lr::Left, Vec3::ZERO, 0.0);
        h.guard(body, Lr::Right, Vec3::new(-0.25, -0.75, -0.2) * arm, extend);
        h.look(body, 0.1, 0.0, 0.0);
    }
}
