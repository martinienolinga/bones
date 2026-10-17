//! Quadrupède au repos : attente (respiration, regard, queue) et
//! s'asseoir (l'arrière-train descend, les jarrets se posent au sol, le
//! dos se redresse, la tête reste à l'horizontale).

use glam::Vec3;

use super::quadruped::Quad;
use crate::presets::error::PresetError;
use crate::presets::params::{ParamSpec, Params};
use crate::presets::procedural::Motion;
use crate::presets::procedural::ctx::{Body, Rigging};
use crate::presets::procedural::curves::{minjerk, noise, wave};

pub(crate) const IDLE: &[ParamSpec] = &[
    ParamSpec::new("breath", "Breathing", 0.0, 2.0, 1.0),
    ParamSpec::new("look", "Look around", 0.0, 2.0, 1.0),
    ParamSpec::new("tail", "Tail motion", 0.0, 2.0, 1.0),
];

pub(crate) struct QuadIdle<'r, 'a> {
    q: Quad<'r, 'a>,
    breath: f32,
    look: f32,
    tail: f32,
}

impl<'r, 'a> QuadIdle<'r, 'a> {
    pub fn new(rig: &'r Rigging<'a>, p: &Params) -> Result<Self, PresetError> {
        Ok(Self {
            q: Quad::new(rig, 2)?,
            breath: p.get(IDLE, "breath"),
            look: p.get(IDLE, "look"),
            tail: p.get(IDLE, "tail"),
        })
    }
}

impl Motion for QuadIdle<'_, '_> {
    fn duration(&self) -> f32 {
        6.0
    }

    fn pose(&self, u: f32, body: &mut Body) {
        let q = &self.q;
        let h = q.height;
        body.reset();
        // Trois respirations, un léger transfert de poids.
        let breath = wave(3.0 * u) * self.breath;
        let shift = noise(u, 2, &[1, 2]);
        body.pelvis(q.root, Vec3::new(0.01 * h * shift, -0.01 * h + 0.004 * h * breath, 0.0), 0.0, 0.0, 0.03 * shift);
        body.spread(&q.spine, -0.02 * breath, 0.03 * shift, -0.02 * shift);
        body.ripple(&q.tail, Vec3::Y, 0.18 * self.tail, 2.0 * u, 0.3);
        body.ripple(&q.tail, Vec3::X, 0.05 * self.tail, u, 0.2);
        body.fk();
        for limb in &q.limbs {
            limb.place(body, limb.contact, glam::Quat::IDENTITY, 1.0);
        }
        let l = self.look;
        body.look(
            &q.neck,
            q.head,
            0.08 * l * noise(u, 5, &[1, 2, 3]),
            0.45 * l * noise(u, 9, &[1, 2]),
            0.05 * l * noise(u, 13, &[2, 3]),
        );
    }
}

pub(crate) const SIT: &[ParamSpec] = &[
    ParamSpec::new("depth", "Depth", 0.3, 1.0, 1.0),
    ParamSpec::toggle("reverse", "Stand up instead", false),
];

pub(crate) struct QuadSit<'r, 'a> {
    q: Quad<'r, 'a>,
    depth: f32,
    reverse: bool,
}

impl<'r, 'a> QuadSit<'r, 'a> {
    pub fn new(rig: &'r Rigging<'a>, p: &Params) -> Result<Self, PresetError> {
        Ok(Self {
            q: Quad::new(rig, 2)?,
            depth: p.get(SIT, "depth"),
            reverse: p.flag(SIT, "reverse"),
        })
    }
}

impl Motion for QuadSit<'_, '_> {
    fn duration(&self) -> f32 {
        2.4
    }

    fn pose(&self, u: f32, body: &mut Body) {
        let q = &self.q;
        let u = u.clamp(0.0, 1.0);
        let u = if self.reverse { 1.0 - u } else { u };
        let k = minjerk(u / 0.75) * self.depth;
        let h = q.height;
        body.reset();
        // Le bassin descend et recule ; le corps se redresse autour des épaules.
        let tilt = -0.55 * k;
        body.pelvis(q.root, Vec3::new(0.0, -0.62 * h * k, -0.08 * h * k), 0.0, tilt, 0.0);
        body.spread(&q.spine, 0.1 * k, 0.0, 0.0);
        body.ripple(&q.tail, Vec3::X, 0.3 * k, 0.25, 0.0);
        body.fk();
        for limb in &q.limbs {
            // Arrière : le métatarse se couche au sol (jarret posé).
            let flex = if limb.front { 0.0 } else { -1.35 * k };
            let paw = limb.contact + q.rig.w(Vec3::Z) * if limb.front { -0.03 * h * k } else { 0.0 };
            limb.place(body, paw, q.flex(flex), 1.0);
        }
        body.look(&q.neck, q.head, 0.1 * k, 0.0, 0.0);
    }
}
