//! Oiseaux : battement d'ailes (l'aile se replie à la remontée, le corps
//! monte à chaque abaissée), vol plané (inclinaison en virage), sautillement,
//! picorage. La tête reste stabilisée comme chez les vrais oiseaux.

use glam::Vec3;

use super::quadruped::Quad;
use crate::BoneId;
use crate::anim::physics::GRAVITY;
use crate::presets::error::{PresetError, incompatible};
use crate::presets::params::{ParamSpec, Params};
use crate::presets::procedural::Motion;
use crate::presets::procedural::ctx::{Body, Rigging};
use crate::presets::procedural::curves::{Curve, bump, cwave, hermite, minjerk, noise, smooth, wave};
use crate::presets::roles::{Chain, Lr};

pub(crate) const FLAP: &[ParamSpec] = &[
    ParamSpec::new("rate", "Wingbeats per second", 0.5, 12.0, 3.0),
    ParamSpec::new("amplitude", "Amplitude", 0.2, 1.5, 1.0),
    ParamSpec::new("speed", "Flight speed (m/s)", 0.0, 20.0, 0.0),
];

pub(crate) const GLIDE: &[ParamSpec] = &[
    ParamSpec::new("bank", "Banking", 0.0, 2.0, 1.0),
    ParamSpec::new("speed", "Flight speed (m/s)", 0.0, 20.0, 0.0),
];

pub(crate) const HOP: &[ParamSpec] = &[
    ParamSpec::new("height", "Hop height (m)", 0.01, 0.3, 0.05),
    ParamSpec::new("distance", "Hop distance (m)", 0.0, 0.5, 0.1),
];

pub(crate) const PECK: &[ParamSpec] = &[ParamSpec::new("rate", "Pecks per second", 0.5, 4.0, 1.5)];

/// Ailes : chaînes des membres antérieurs.
fn wings(rig: &Rigging) -> Result<[Vec<BoneId>; 2], PresetError> {
    let w = [rig.chain(Chain::Arm(Lr::Left)), rig.chain(Chain::Arm(Lr::Right))];
    if w.iter().any(|c| c.is_empty()) {
        return Err(incompatible("wings not found"));
    }
    Ok(w)
}

/// Pose une aile : élévation (vers le haut), balayage vers l'arrière et
/// repli en Z (0 : déployée, 1 : repliée le long du corps — humérus vers
/// l'arrière, avant-bras vers l'avant, main vers l'arrière).
fn wing(body: &mut Body, bones: &[BoneId], side: Lr, elevation: f32, sweep: f32, fold: f32) {
    let s = side.sign();
    for (i, b) in bones.iter().enumerate() {
        let (e, f) = match i {
            0 => (elevation, sweep + fold),
            1 => (-0.35 * elevation, -2.6 * fold),
            _ => (-0.25 * elevation, 2.7 * fold),
        };
        body.turn(Some(*b), Vec3::Z, s * e);
        body.turn(Some(*b), Vec3::Y, s * f);
    }
}

/// Pattes ramenées vers l'arrière sous la queue (en vol).
fn tuck_legs(body: &mut Body, q: &Quad) {
    for limb in q.limbs.iter().filter(|l| !l.front) {
        let angles = [0.5, 1.2, 0.3, 0.5];
        for (b, a) in limb.chain.iter().zip(angles) {
            body.turn(Some(*b), Vec3::X, a);
        }
    }
}

pub(crate) struct Flap<'r, 'a> {
    q: Quad<'r, 'a>,
    wings: [Vec<BoneId>; 2],
    rate: f32,
    amplitude: f32,
    speed: f32,
    glide: bool,
    bank: f32,
}

impl<'r, 'a> Flap<'r, 'a> {
    pub fn new(rig: &'r Rigging<'a>, p: &Params, glide: bool) -> Result<Self, PresetError> {
        let specs = if glide { GLIDE } else { FLAP };
        Ok(Self {
            q: Quad::new(rig, 0)?,
            wings: wings(rig)?,
            rate: if glide { 0.25 } else { p.get(FLAP, "rate") },
            amplitude: if glide { 0.0 } else { p.get(FLAP, "amplitude") },
            speed: p.get(specs, "speed"),
            glide,
            bank: if glide { p.get(GLIDE, "bank") } else { 0.0 },
        })
    }
}

impl Motion for Flap<'_, '_> {
    fn duration(&self) -> f32 {
        1.0 / self.rate
    }

    fn pose(&self, u: f32, body: &mut Body) {
        let q = &self.q;
        let h = q.height;
        body.reset();
        let travel = self.speed * self.duration() * u;
        if self.glide {
            // Virage incliné lent, ailes tendues qui frémissent.
            let roll = 0.35 * self.bank * wave(u);
            body.pelvis(q.root, Vec3::new(0.0, 0.03 * h * cwave(2.0 * u), travel), 0.1 * roll, -0.05, roll);
            for (bones, side) in self.wings.iter().zip(Lr::BOTH) {
                let lift = 0.12 + 0.04 * noise(u, side as u32, &[3, 5]) + 0.08 * side.sign() * roll;
                wing(body, bones, side, lift, 0.05, 0.0);
            }
            body.ripple(&q.tail, Vec3::Y, 0.15 * roll, 0.25, 0.0);
        } else {
            // Abaissée sur la première moitié, remontée aile repliée.
            let a = self.amplitude;
            let elevation = a * (0.25 + 0.75 * cwave(u));
            let fold = a * 0.35 * bump((u.rem_euclid(1.0) - 0.5).max(0.0) * 2.0, 0.5);
            let lift = 0.06 * h * a * wave(u - 0.1);
            body.pelvis(q.root, Vec3::new(0.0, lift, travel), 0.0, -0.1 - 0.05 * wave(u), 0.0);
            for (bones, side) in self.wings.iter().zip(Lr::BOTH) {
                wing(body, bones, side, elevation, 0.2 * a * wave(u), fold);
            }
            body.ripple(&q.tail, Vec3::X, 0.1 * a, u, 0.2);
        }
        tuck_legs(body, q);
        body.fk();
        body.look(&q.neck, q.head, 0.0, 0.0, 0.0);
    }
}

pub(crate) struct Hop<'r, 'a> {
    q: Quad<'r, 'a>,
    wings: Vec<Vec<BoneId>>,
    height: f32,
    distance: f32,
}

impl<'r, 'a> Hop<'r, 'a> {
    pub fn new(rig: &'r Rigging<'a>, p: &Params) -> Result<Self, PresetError> {
        Ok(Self {
            q: Quad::new(rig, 2)?,
            wings: wings(rig).map(Vec::from).unwrap_or_default(),
            height: p.get(HOP, "height"),
            distance: p.get(HOP, "distance"),
        })
    }

    fn flight(&self) -> f32 {
        2.0 * (2.0 * self.height / GRAVITY).sqrt()
    }
}

impl Motion for Hop<'_, '_> {
    /// Accroupi 0,2 s, vol balistique, réception 0,25 s.
    fn duration(&self) -> f32 {
        0.45 + self.flight()
    }

    fn pose(&self, u: f32, body: &mut Body) {
        let q = &self.q;
        let h = q.height;
        let t = u.rem_euclid(1.0) * self.duration();
        let k = u.floor();
        let fl = self.flight();
        let v0 = GRAVITY * fl * 0.5;
        let crouch = 0.25 * h;
        let (y, x) = if t < 0.2 {
            (hermite(0.0, 0.0, -crouch, 0.0, t, 0.2), 0.0)
        } else if t < 0.2 + fl {
            let s = t - 0.2;
            let p = s / fl;
            (-crouch * (1.0 - smooth(p * 4.0)) + v0 * s - 0.5 * GRAVITY * s * s, self.distance * minjerk(p))
        } else {
            let s = t - 0.2 - fl;
            (-crouch * 0.6 * bump(s / 0.25, 0.4), self.distance)
        };
        let travel = self.distance * k + x;
        body.reset();
        body.pelvis(q.root, Vec3::new(0.0, y, travel), 0.0, 0.15 * bump(t / (0.4 + fl), 0.3), 0.0);
        let open = if t > 0.15 && t < 0.25 + fl { bump((t - 0.15) / (0.1 + fl), 0.5) } else { 0.0 };
        for (bones, side) in self.wings.iter().zip(Lr::BOTH) {
            wing(body, bones, side, -0.2 + 0.8 * open, 0.4 * (1.0 - open), 1.0 - 0.8 * open);
        }
        body.fk();
        let lift = if t > 0.2 && t < 0.2 + fl { (y + crouch).max(0.0) * 0.8 } else { 0.0 };
        for limb in q.limbs.iter().filter(|l| !l.front) {
            let step = self.distance * k + if t < 0.2 { 0.0 } else { x };
            let paw = limb.contact + q.rig.w(Vec3::new(0.0, 0.0, step)) + q.rig.frame.up * lift;
            limb.place(body, paw, q.flex(0.0), 1.0);
        }
        body.look(&q.neck, q.head, 0.0, 0.0, 0.0);
    }
}

pub(crate) struct Peck<'r, 'a> {
    q: Quad<'r, 'a>,
    wings: Vec<Vec<BoneId>>,
    rate: f32,
    dip: Curve,
}

impl<'r, 'a> Peck<'r, 'a> {
    pub fn new(rig: &'r Rigging<'a>, p: &Params) -> Result<Self, PresetError> {
        Ok(Self {
            q: Quad::new(rig, 2)?,
            wings: wings(rig).map(Vec::from).unwrap_or_default(),
            rate: p.get(PECK, "rate"),
            // Deux coups rapides puis la tête se relève et observe.
            dip: Curve::new(&[(0.0, 0.0), (0.12, 1.0), (0.2, 0.75), (0.3, 1.0), (0.42, 0.0), (1.0, 0.0)]),
        })
    }
}

impl Motion for Peck<'_, '_> {
    fn duration(&self) -> f32 {
        2.0 / self.rate
    }

    fn pose(&self, u: f32, body: &mut Body) {
        let q = &self.q;
        let d = self.dip.at(u.rem_euclid(1.0));
        let h = q.height;
        body.reset();
        body.pelvis(q.root, Vec3::new(0.0, -0.15 * h * d, 0.0), 0.0, 0.5 * d, 0.0);
        body.spread(&q.spine, 0.3 * d, 0.0, 0.0);
        body.spread(&q.neck, 2.1 * d, 0.0, 0.0);
        for (bones, side) in self.wings.iter().zip(Lr::BOTH) {
            wing(body, bones, side, -0.2, 0.4, 1.0);
        }
        body.ripple(&q.tail, Vec3::X, -0.3 * d, 0.25, 0.0);
        body.fk();
        for limb in q.limbs.iter().filter(|l| !l.front) {
            limb.place(body, limb.contact, q.flex(0.0), 1.0);
        }
        // Entre deux coups, la tête tourne pour regarder d'un œil.
        let glance = 0.4 * noise(u, 4, &[1]) * (1.0 - d);
        body.look(&[], q.head, 1.3 * d, glance, 0.0);
    }
}
