//! Saut humain : préparation (flexion, bras en arrière), impulsion (talons
//! levés, bras lancés vers le haut), vol balistique exact du bassin
//! (parabole sous la gravité, comme la correction balistique de Cascadeur),
//! réception amortie puis redressement.

use glam::Vec3;

use super::rig::Human;
use super::stance::Foot;
use crate::anim::physics::GRAVITY;
use crate::presets::error::PresetError;
use crate::presets::params::{ParamSpec, Params};
use crate::presets::procedural::Motion;
use crate::presets::procedural::ctx::{Body, Rigging};
use crate::presets::procedural::curves::{Curve, bump, hermite, minjerk, smooth};
use crate::presets::roles::Lr;

pub(crate) const PARAMS: &[ParamSpec] = &[
    ParamSpec::new("height", "Jump height (m)", 0.05, 1.0, 0.35),
    ParamSpec::new("distance", "Distance (m)", 0.0, 2.5, 0.6),
];

pub(crate) struct Jump<'r, 'a> {
    h: Human<'r, 'a>,
    /// Instants : fin de la préparation, décollage, atterrissage, fin de
    /// l'amorti, fin.
    times: [f32; 5],
    v0: f32,
    crouch: f32,
    distance: f32,
    arms: Curve,
    lean: Curve,
}

impl<'r, 'a> Jump<'r, 'a> {
    pub fn new(rig: &'r Rigging<'a>, p: &Params) -> Result<Self, PresetError> {
        let h = Human::new(rig)?;
        let height = p.get(PARAMS, "height");
        let v0 = (2.0 * GRAVITY * height).sqrt();
        let flight = 2.0 * v0 / GRAVITY;
        let leg = h.legs[0].length;
        let crouch = (0.14 + 0.1 * height / 0.35).min(0.32) * leg;
        let t1 = 0.45;
        let t2 = t1 + 0.2;
        let t3 = t2 + flight;
        let t4 = t3 + 0.3;
        let t5 = t4 + 0.55;
        let arms = Curve::new(&[
            (0.0, 0.0),
            (t1 - 0.05, -0.9),
            (t2, 2.4),
            (t2 + flight * 0.5, 2.0),
            (t3, 1.0),
            (t4, 0.5),
            (t5, 0.0),
        ]);
        let lean = Curve::new(&[(0.0, 0.0), (t1, 0.45), (t2, -0.05), (t3, 0.1), (t4, 0.35), (t5, 0.0)]);
        Ok(Self {
            h,
            times: [t1, t2, t3, t4, t5],
            v0,
            crouch,
            distance: p.get(PARAMS, "distance"),
            arms,
            lean,
        })
    }

    /// Bassin à l'instant `t` : (avance, hauteur relative au repos).
    fn hips(&self, t: f32) -> (f32, f32) {
        let [t1, t2, t3, t4, t5] = self.times;
        let leg = self.h.legs[0].length;
        let rise = 0.05 * leg;
        let flight = t3 - t2;
        let vx = self.distance / flight;
        let (x_to, x_land) = (0.06 * leg, 0.06 * leg + self.distance);
        let land = 0.75 * self.crouch;
        if t < t1 {
            (hermite(0.0, 0.0, -0.04 * leg, 0.0, t, t1), hermite(0.0, 0.0, -self.crouch, 0.0, t, t1))
        } else if t < t2 {
            let s = t - t1;
            (hermite(-0.04 * leg, 0.0, x_to, vx, s, t2 - t1), hermite(-self.crouch, 0.0, rise, self.v0, s, t2 - t1))
        } else if t < t3 {
            let s = t - t2;
            (x_to + vx * s, rise + self.v0 * s - 0.5 * GRAVITY * s * s)
        } else if t < t4 {
            let s = t - t3;
            // Le bassin revient au-dessus des pieds en amortissant.
            let end = self.distance + 0.02 * leg;
            (hermite(x_land, vx, end, 0.0, s, t4 - t3), hermite(rise, -self.v0, -land, 0.0, s, t4 - t3))
        } else {
            let s = t - t4;
            (self.distance + 0.02 * leg, hermite(-land, 0.0, -0.01 * leg, 0.0, s, t5 - t4))
        }
    }
}

impl Motion for Jump<'_, '_> {
    fn duration(&self) -> f32 {
        self.times[4]
    }

    fn pose(&self, u: f32, body: &mut Body) {
        let h = &self.h;
        let [t1, t2, t3, _, _] = self.times;
        let t = u.clamp(0.0, 1.0) * self.times[4];
        body.reset();
        let (x, y) = self.hips(t);
        let lean = self.lean.at(t);
        h.pelvis(body, Vec3::new(0.0, y, x), 0.0, 0.5 * lean, 0.0);
        h.bend_spine(body, 0.5 * lean, 0.0, 0.0);
        for side in Lr::BOTH {
            h.curl(body, side, 0.35);
        }
        body.fk();
        // Pieds : au départ jusqu'au décollage (talons levés pendant
        // l'impulsion), repliés en vol, posés à l'arrivée pointe d'abord.
        let landed = self.distance;
        let foot = if t < t1 {
            Foot::default()
        } else if t < t2 {
            Foot {
                pitch: -0.7 * smooth((t - t1) / (t2 - t1)),
                ..Foot::default()
            }
        } else if t < t3 {
            let s = (t - t2) / (t3 - t2);
            let tuck = 0.18 * h.legs[0].length * bump(s, 0.5);
            Foot {
                offset: Vec3::new(0.0, 0.0, landed * minjerk(s)),
                pitch: -0.7 + 0.5 * smooth(s),
                lift: (y - self.hips(t2).1).max(0.0) * 0.9 + tuck,
                yaw: 0.0,
            }
        } else {
            let s = ((t - t3) / 0.1).min(1.0);
            Foot {
                offset: Vec3::new(0.0, 0.0, landed),
                pitch: -0.2 * (1.0 - smooth(s)),
                ..Foot::default()
            }
        };
        h.stand(body, [foot, foot]);
        let swing = self.arms.at(t);
        h.relax_arms(body, [swing, swing], 0.15 + 0.1 * swing.max(0.0), 0.25);
        h.look(body, -0.3 * lean, 0.0, 0.0);
    }
}
