//! Attente humaine : respiration et transferts de poids ; regard alentour.

use std::f32::consts::TAU;

use glam::Vec3;

use super::rig::Human;
use super::stance::Foot;
use crate::presets::error::PresetError;
use crate::presets::params::{ParamSpec, Params};
use crate::presets::procedural::Motion;
use crate::presets::procedural::ctx::{Body, Rigging};
use crate::presets::procedural::curves::{Curve, noise, wave};
use crate::presets::roles::Lr;

pub(crate) const IDLE: &[ParamSpec] = &[
    ParamSpec::new("breath", "Breathing", 0.0, 2.0, 1.0),
    ParamSpec::new("sway", "Weight shift", 0.0, 2.0, 1.0),
    ParamSpec::new("duration", "Duration (s)", 4.0, 16.0, 8.0),
];

/// Attente : deux respirations, un transfert de poids d'une jambe sur
/// l'autre (la jambe libre se relâche), micro-mouvements de la tête.
pub(crate) struct Idle<'r, 'a> {
    h: Human<'r, 'a>,
    breath: f32,
    sway: f32,
    duration: f32,
}

impl<'r, 'a> Idle<'r, 'a> {
    pub fn new(rig: &'r Rigging<'a>, p: &Params) -> Result<Self, PresetError> {
        Ok(Self {
            h: Human::new(rig)?,
            breath: p.get(IDLE, "breath"),
            sway: p.get(IDLE, "sway"),
            duration: p.get(IDLE, "duration"),
        })
    }
}

impl Motion for Idle<'_, '_> {
    fn duration(&self) -> f32 {
        self.duration
    }

    fn pose(&self, u: f32, body: &mut Body) {
        let h = &self.h;
        body.reset();
        let size = h.hip_height;
        // Le poids passe d'une jambe à l'autre (le bassin se décale et
        // bascule du côté de la jambe d'appui) ; deux respirations.
        let shift = wave(u) * self.sway;
        let breath = (0.5 - 0.5 * (TAU * 2.0 * u).cos()) * self.breath;
        let offset = Vec3::new(
            0.035 * size * shift,
            -0.012 * size - 0.006 * size * shift.abs(),
            0.0,
        );
        let roll = 0.045 * shift;
        h.pelvis(body, offset, 0.03 * noise(u, 1, &[1, 2]), 0.0, roll);
        h.bend_spine(body, -0.03 * breath, -0.02 * shift, -0.7 * roll);
        for side in Lr::BOTH {
            h.shrug(body, side, 0.03 * breath, 0.0);
            h.curl(body, side, 0.3);
        }
        body.fk();
        h.stand(body, [Foot::at(0.0, 0.0), Foot::at(0.0, 0.0)]);
        let lag = wave(u - 0.06) * self.sway;
        h.relax_arms(body, [0.03 * lag, -0.03 * lag], 0.12 + 0.02 * breath, 0.15);
        h.look(
            body,
            0.05 * noise(u, 7, &[1, 3, 4]),
            0.12 * noise(u, 11, &[1, 2, 3]),
            0.04 * noise(u, 13, &[1, 2]) - 0.3 * roll,
        );
    }
}

pub(crate) const LOOK: &[ParamSpec] = &[
    ParamSpec::new("angle", "Turn angle (deg)", 10.0, 90.0, 65.0),
    ParamSpec::new("duration", "Duration (s)", 3.0, 12.0, 6.0),
];

/// Regard alentour : la tête mène, le buste suit avec retard ; à gauche,
/// au centre, à droite, au centre, avec des temps d'arrêt.
pub(crate) struct LookAround<'r, 'a> {
    h: Human<'r, 'a>,
    angle: f32,
    duration: f32,
    yaw: Curve,
    pitch: Curve,
}

impl<'r, 'a> LookAround<'r, 'a> {
    pub fn new(rig: &'r Rigging<'a>, p: &Params) -> Result<Self, PresetError> {
        let keys = [
            (0.0, 0.0),
            (0.06, 0.0),
            (0.2, 1.0),
            (0.36, 1.0),
            (0.48, 0.0),
            (0.56, 0.0),
            (0.7, -1.0),
            (0.86, -1.0),
            (0.98, 0.0),
            (1.0, 0.0),
        ];
        let pitch = [
            (0.0, 0.0),
            (0.2, -0.15),
            (0.36, 0.1),
            (0.48, 0.0),
            (0.7, 0.05),
            (0.86, -0.1),
            (1.0, 0.0),
        ];
        Ok(Self {
            h: Human::new(rig)?,
            angle: p.get(LOOK, "angle").to_radians(),
            duration: p.get(LOOK, "duration"),
            yaw: Curve::new(&keys),
            pitch: Curve::new(&pitch),
        })
    }
}

impl Motion for LookAround<'_, '_> {
    fn duration(&self) -> f32 {
        self.duration
    }

    fn pose(&self, u: f32, body: &mut Body) {
        let h = &self.h;
        let u = u.rem_euclid(1.0);
        body.reset();
        // Le buste suit la tête avec 0,25 s de retard.
        let lag = 0.25 / self.duration;
        let chest = self.yaw.at(u - lag) * self.angle * 0.3;
        let size = h.hip_height;
        h.pelvis(
            body,
            Vec3::new(
                -0.01 * size * chest.signum() * chest.abs().min(0.3),
                -0.012 * size,
                0.0,
            ),
            0.2 * chest,
            0.0,
            0.0,
        );
        h.bend_spine(body, 0.0, 0.8 * chest, 0.0);
        for side in Lr::BOTH {
            h.curl(body, side, 0.3);
        }
        body.fk();
        h.stand(body, [Foot::at(0.0, 0.0), Foot::at(0.0, 0.0)]);
        h.relax_arms(body, [0.02 * chest, -0.02 * chest], 0.12, 0.15);
        h.look(body, self.pitch.at(u) * 0.5, self.yaw.at(u) * self.angle, 0.0);
    }
}
