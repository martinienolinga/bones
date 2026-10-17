//! Gestes humains : salut de la main, gestes de conversation.

use glam::Vec3;

use super::rig::Human;
use super::stance::Foot;
use crate::presets::error::PresetError;
use crate::presets::params::{ParamSpec, Params};
use crate::presets::procedural::Motion;
use crate::presets::procedural::ctx::{Body, Rigging};
use crate::presets::procedural::curves::{noise, wave};
use crate::presets::roles::Lr;

pub(crate) const WAVE: &[ParamSpec] = &[
    ParamSpec::new("waves", "Waves per second", 0.5, 3.0, 1.5),
    ParamSpec::new("amplitude", "Amplitude", 0.2, 2.0, 1.0),
    ParamSpec::toggle("left", "Left hand", false),
];

/// Salut : main levée à hauteur de tête, l'avant-bras balance de côté,
/// paume vers l'avant ; le corps se penche légèrement à l'opposé.
pub(crate) struct Wave<'r, 'a> {
    h: Human<'r, 'a>,
    rate: f32,
    amplitude: f32,
    side: Lr,
}

impl<'r, 'a> Wave<'r, 'a> {
    pub fn new(rig: &'r Rigging<'a>, p: &Params) -> Result<Self, PresetError> {
        Ok(Self {
            h: Human::new(rig)?,
            rate: p.get(WAVE, "waves"),
            amplitude: p.get(WAVE, "amplitude"),
            side: if p.flag(WAVE, "left") { Lr::Left } else { Lr::Right },
        })
    }
}

impl Motion for Wave<'_, '_> {
    /// Deux allers-retours de la main par cycle.
    fn duration(&self) -> f32 {
        2.0 / self.rate
    }

    fn pose(&self, u: f32, body: &mut Body) {
        let h = &self.h;
        let (side, other) = (self.side, self.side.mirror());
        let s = side.sign();
        body.reset();
        let size = h.hip_height;
        let beat = wave(2.0 * u);
        h.pelvis(body, Vec3::new(-s * 0.02 * size, -0.012 * size, 0.0), 0.0, 0.0, -s * 0.03);
        h.bend_spine(body, 0.0, s * 0.05, s * 0.06 + 0.01 * beat);
        h.shrug(body, side, 0.12, 0.05);
        h.curl(body, side, 0.05);
        h.curl(body, other, 0.3);
        body.fk();
        h.stand(body, [Foot::at(0.0, 0.0), Foot::at(0.0, 0.0)]);
        let arm = h.arm_of(side).map_or(0.6, |a| a.length);
        // Main au-dessus de l'épaule, sur le côté ; elle balance latéralement.
        let hand = Vec3::new(s * (0.22 + 0.28 * self.amplitude * beat), 0.62, 0.12) * arm;
        let elbow = Vec3::new(s * 0.9, -0.5, 0.0);
        h.swing_arm(body, other, 0.1, 0.02 * beat, 0.2, 0.1);
        h.reach_from_shoulder(body, side, hand, elbow, -0.15);
        h.palm_towards(body, side, h.rig.w(Vec3::Z));
        h.look(body, -0.05, s * 0.15, -s * 0.08 + 0.02 * beat);
    }
}

pub(crate) const TALK: &[ParamSpec] = &[
    ParamSpec::new("energy", "Energy", 0.2, 2.0, 1.0),
    ParamSpec::new("duration", "Duration (s)", 2.0, 10.0, 4.0),
];

/// Conversation : gestes rythmés des deux mains devant le ventre (bruit
/// périodique), hochements de tête sur les temps forts.
pub(crate) struct Talk<'r, 'a> {
    h: Human<'r, 'a>,
    energy: f32,
    duration: f32,
}

impl<'r, 'a> Talk<'r, 'a> {
    pub fn new(rig: &'r Rigging<'a>, p: &Params) -> Result<Self, PresetError> {
        Ok(Self {
            h: Human::new(rig)?,
            energy: p.get(TALK, "energy"),
            duration: p.get(TALK, "duration"),
        })
    }
}

impl Motion for Talk<'_, '_> {
    fn duration(&self) -> f32 {
        self.duration
    }

    fn pose(&self, u: f32, body: &mut Body) {
        let h = &self.h;
        let e = self.energy;
        body.reset();
        let size = h.hip_height;
        let shift = noise(u, 3, &[1, 2]);
        h.pelvis(body, Vec3::new(0.015 * size * shift, -0.012 * size, 0.0), 0.04 * shift, 0.0, 0.02 * shift);
        h.bend_spine(body, 0.03 + 0.03 * e * noise(u, 5, &[2, 3]), 0.08 * e * noise(u, 9, &[1, 3]), -0.01 * shift);
        for side in Lr::BOTH {
            h.curl(body, side, 0.25);
        }
        body.fk();
        h.stand(body, [Foot::at(0.0, 0.0), Foot::at(0.0, 0.0)]);
        for (i, side) in Lr::BOTH.into_iter().enumerate() {
            let s = side.sign();
            let arm = h.arm_of(side).map_or(0.6, |a| a.length);
            let seed = 20 + i as u32 * 7;
            let g = |k: u32, harm: &[u32]| e * noise(u, seed + k, harm);
            let hand = Vec3::new(
                s * (0.1 + 0.08 * g(0, &[2, 3])),
                -0.62 + 0.12 * g(1, &[3, 4, 5]).max(-0.4),
                0.42 + 0.1 * g(2, &[2, 5]),
            ) * arm;
            h.reach_from_shoulder(body, side, hand, Vec3::new(s * 0.5, -1.0, -0.2), 0.1 * g(3, &[3, 4]));
            h.palm_towards(body, side, h.rig.w(Vec3::new(-s * 0.6, 0.8, 0.2)));
        }
        let nod = 0.07 * e * noise(u, 31, &[4, 6, 8]);
        h.look(body, nod, 0.15 * e * noise(u, 37, &[1, 2]), 0.04 * noise(u, 41, &[2, 3]));
    }
}
