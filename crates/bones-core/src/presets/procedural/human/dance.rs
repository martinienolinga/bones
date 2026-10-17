//! Danse simple : pas chassé (« step-touch ») sur quatre temps, rebond des
//! genoux sur chaque temps, bras pliés qui balancent en alternance, tête
//! qui marque le rythme.

use glam::Vec3;

use super::rig::Human;
use super::stance::Foot;
use crate::presets::error::PresetError;
use crate::presets::params::{ParamSpec, Params};
use crate::presets::procedural::Motion;
use crate::presets::procedural::ctx::{Body, Rigging};
use crate::presets::procedural::curves::{Curve, bump, cwave};
use crate::presets::roles::Lr;

pub(crate) const PARAMS: &[ParamSpec] = &[
    ParamSpec::new("bpm", "Tempo (BPM)", 70.0, 160.0, 116.0),
    ParamSpec::new("step", "Step width (m)", 0.1, 0.5, 0.3),
    ParamSpec::new("energy", "Energy", 0.2, 1.5, 1.0),
];

pub(crate) struct Dance<'r, 'a> {
    h: Human<'r, 'a>,
    bpm: f32,
    step: f32,
    energy: f32,
    /// Décalage latéral (en largeurs de pas) des pieds gauche et droit, par temps.
    feet: [Curve; 2],
    /// Pas en cours : (début, fin) en temps, pour lever le pied.
    moves: [[(f32, f32); 2]; 2],
}

impl<'r, 'a> Dance<'r, 'a> {
    pub fn new(rig: &'r Rigging<'a>, p: &Params) -> Result<Self, PresetError> {
        // Droit dehors (temps 0), gauche rejoint (1), gauche revient (2),
        // droit revient (3) ; les pas se posent sur les temps.
        let left = Curve::new(&[
            (0.0, 0.0),
            (0.5, 0.0),
            (1.0, -1.0),
            (1.5, -1.0),
            (2.0, 0.0),
            (4.0, 0.0),
        ]);
        let right = Curve::new(&[(0.0, -1.0), (2.5, -1.0), (3.0, 0.0), (3.5, 0.0), (4.0, -1.0)]);
        Ok(Self {
            h: Human::new(rig)?,
            bpm: p.get(PARAMS, "bpm"),
            step: p.get(PARAMS, "step"),
            energy: p.get(PARAMS, "energy"),
            feet: [left, right],
            moves: [[(0.5, 1.0), (1.5, 2.0)], [(2.5, 3.0), (3.5, 4.0)]],
        })
    }
}

impl Motion for Dance<'_, '_> {
    fn duration(&self) -> f32 {
        4.0 * 60.0 / self.bpm
    }

    fn pose(&self, u: f32, body: &mut Body) {
        let h = &self.h;
        let b = u.rem_euclid(1.0) * 4.0;
        let e = self.energy;
        let leg = h.legs[0].length;
        let x = |i: usize| self.feet[i].at(b) * self.step;
        let center = (x(0) + x(1)) * 0.5;
        let bounce = -0.035 * leg * e * (0.5 + 0.5 * cwave(b));
        let lean = 0.06 * e * cwave(b * 0.5);
        body.reset();
        h.pelvis(
            body,
            Vec3::new(center, bounce - 0.02 * leg, 0.0),
            0.1 * e * cwave(b * 0.5 + 0.25),
            0.04,
            lean,
        );
        h.bend_spine(body, 0.05, -0.2 * e * cwave(b * 0.5 + 0.25), -0.8 * lean);
        for side in Lr::BOTH {
            h.shrug(body, side, 0.06 * e * (0.5 - 0.5 * cwave(b)), 0.0);
            h.curl(body, side, 0.6);
        }
        body.fk();
        let mut feet = [Foot::default(); 2];
        for (i, foot) in feet.iter_mut().enumerate() {
            foot.offset = Vec3::new(x(i), 0.0, 0.0);
            for (start, end) in self.moves[i] {
                if b > start && b < end {
                    foot.lift = 0.07 * leg * bump((b - start) / (end - start), 0.4);
                }
            }
        }
        h.stand(body, feet);
        let swing = 0.45 * e * cwave(b * 0.5);
        for (side, s) in [(Lr::Left, swing), (Lr::Right, -swing)] {
            h.swing_arm(body, side, 0.35, 0.4 + s, 1.5 - 0.2 * s, 0.2);
        }
        h.look(
            body,
            0.07 * e * cwave(b),
            0.1 * e * cwave(b * 0.5 + 0.25),
            -0.5 * lean,
        );
    }
}
