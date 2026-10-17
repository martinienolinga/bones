//! Changements de posture : s'accroupir, s'asseoir, se relever. Les pieds
//! restent plantés (IK) ; le buste se penche pour garder le centre de
//! masse au-dessus des appuis.

use glam::Vec3;

use super::rig::Human;
use super::stance::Foot;
use crate::presets::error::PresetError;
use crate::presets::params::{ParamSpec, Params};
use crate::presets::procedural::Motion;
use crate::presets::procedural::ctx::{Body, Rigging};
use crate::presets::procedural::curves::{Curve, minjerk, wave};
use crate::presets::roles::Lr;

pub(crate) const CROUCH: &[ParamSpec] = &[
    ParamSpec::new("depth", "Depth", 0.2, 1.0, 0.85),
    ParamSpec::new("hold", "Hold (s)", 0.0, 4.0, 1.2),
];

/// S'accroupir, rester accroupi en respirant, se relever.
pub(crate) struct Crouch<'r, 'a> {
    h: Human<'r, 'a>,
    depth: f32,
    times: [f32; 3],
}

impl<'r, 'a> Crouch<'r, 'a> {
    pub fn new(rig: &'r Rigging<'a>, p: &Params) -> Result<Self, PresetError> {
        let hold = p.get(CROUCH, "hold");
        Ok(Self {
            h: Human::new(rig)?,
            depth: p.get(CROUCH, "depth"),
            times: [0.7, 0.7 + hold, 1.5 + hold],
        })
    }
}

impl Motion for Crouch<'_, '_> {
    fn duration(&self) -> f32 {
        self.times[2] + 0.3
    }

    fn pose(&self, u: f32, body: &mut Body) {
        let h = &self.h;
        let [down, hold, up] = self.times;
        let t = u.clamp(0.0, 1.0) * self.duration();
        let k = if t < down {
            minjerk(t / down)
        } else if t < hold {
            1.0
        } else {
            1.0 - minjerk((t - hold) / (up - hold))
        };
        let d = self.depth * k;
        let leg = h.legs[0].length;
        let breath = if t >= down && t < hold {
            0.01 * leg * wave((t - down) / 2.5)
        } else {
            0.0
        };
        body.reset();
        let offset = Vec3::new(0.0, -0.5 * leg * d + breath, -0.12 * leg * d);
        h.pelvis(body, offset, 0.0, 0.25 * d, 0.0);
        h.bend_spine(body, 0.3 * d, 0.0, 0.0);
        for side in Lr::BOTH {
            h.curl(body, side, 0.3 + 0.2 * d);
        }
        body.fk();
        // Talons levés en accroupissement profond.
        let heel = Foot {
            pitch: -0.5 * ((d - 0.7) / 0.3).clamp(0.0, 1.0),
            ..Foot::default()
        };
        h.stand(body, [heel, heel]);
        let swing = 0.9 * d;
        h.relax_arms(body, [swing, swing], 0.15 + 0.15 * d, 0.3 + 0.6 * d);
        h.look(body, -0.15 * d, 0.0, 0.0);
    }
}

pub(crate) const SIT: &[ParamSpec] = &[
    ParamSpec::new("seat", "Seat height", 0.35, 0.75, 0.52),
    ParamSpec::toggle("reverse", "Stand up instead", false),
];

/// S'asseoir sur une chaise (ou se relever) : le buste se penche, le bassin
/// recule et descend, puis le buste se redresse ; les mains finissent sur
/// les cuisses.
pub(crate) struct Sit<'r, 'a> {
    h: Human<'r, 'a>,
    seat: f32,
    reverse: bool,
    lean: Curve,
    descend: Curve,
}

impl<'r, 'a> Sit<'r, 'a> {
    pub fn new(rig: &'r Rigging<'a>, p: &Params) -> Result<Self, PresetError> {
        Ok(Self {
            h: Human::new(rig)?,
            seat: p.get(SIT, "seat"),
            reverse: p.flag(SIT, "reverse"),
            lean: Curve::new(&[(0.0, 0.0), (0.35, 0.65), (0.6, 0.55), (0.9, 0.08), (1.0, 0.08)]),
            descend: Curve::new(&[(0.0, 0.0), (0.15, 0.0), (0.75, 1.0), (1.0, 1.0)]),
        })
    }
}

impl Motion for Sit<'_, '_> {
    fn duration(&self) -> f32 {
        2.2
    }

    fn pose(&self, u: f32, body: &mut Body) {
        let h = &self.h;
        let u = u.clamp(0.0, 1.0);
        let u = if self.reverse { 1.0 - u } else { u };
        let k = self.descend.at(u);
        let lean = self.lean.at(u);
        let thigh = h.rig.len(h.legs[0].thigh);
        let drop = h.hip_height * (1.0 - self.seat);
        body.reset();
        h.pelvis(
            body,
            Vec3::new(0.0, -drop * k, -0.85 * thigh * k),
            0.0,
            0.3 * lean,
            0.0,
        );
        h.bend_spine(body, 0.7 * lean, 0.0, 0.0);
        for side in Lr::BOTH {
            h.curl(body, side, 0.3);
        }
        body.fk();
        // Les pieds reculent un peu pour se placer sous les genoux.
        let feet = Foot::at(0.0, -0.06 * thigh * k);
        h.stand(body, [feet, feet]);
        // Bras en avant pendant la descente, mains sur les cuisses ensuite.
        for side in Lr::BOTH {
            let leg = h.leg_of(side);
            let knee = body.head(leg.shin);
            let hip = body.head(leg.thigh);
            let on_thigh = hip.lerp(knee, 0.75) + h.rig.frame.up * 0.06 * thigh;
            let forward = 1.2 * (lean - 0.1).max(0.0);
            h.swing_arm(body, side, 0.12, forward, 0.4, 0.1);
            let hand = body.tail(h.arm_of(side).map_or(leg.shin, |a| a.fore));
            let w = ((k - 0.7) / 0.3).clamp(0.0, 1.0);
            let pole = body.head(h.arm_of(side).map_or(leg.shin, |a| a.fore)) - h.rig.w(Vec3::Z) * 0.3;
            h.reach(body, side, hand.lerp(on_thigh, w), pole, 0.2 * w);
        }
        h.look(body, -0.1 * lean, 0.0, 0.0);
    }
}
