//! Quadrupèdes : pas, trot, galop. Chaque patte suit son cycle avec un
//! décalage de phase propre à l'allure (Hildebrand 1965 : pas latéral,
//! paires diagonales au trot, galop rotatoire) ; la foulée suit la loi de
//! similitude dynamique d'Alexander & Jayes (1983) : λ = 2,3·h·(v²/gh)^0,3.
//! Les pattes sont posées en IK, le dos ondule (latéralement au pas, en
//! flexion-extension au galop), la tête hoche, la queue suit avec retard.

use glam::{Quat, Vec3};

use super::limb::Limb;
use crate::BoneId;
use crate::presets::error::{PresetError, incompatible};
use crate::presets::params::{ParamSpec, Params};
use crate::presets::procedural::Motion;
use crate::presets::procedural::ctx::{Body, Rigging};
use crate::presets::procedural::curves::{bump, cwave, envelope, minjerk, periodic, smooth, wave};
use crate::presets::roles::{Chain, Lr};

/// Quadrupède vu par les générateurs.
pub(crate) struct Quad<'r, 'a> {
    pub rig: &'r Rigging<'a>,
    pub root: BoneId,
    pub spine: Vec<BoneId>,
    pub neck: Vec<BoneId>,
    pub head: Option<BoneId>,
    pub tail: Vec<BoneId>,
    /// Pattes présentes : avant gauche, avant droite, arrière gauche, arrière droite.
    pub limbs: Vec<Limb>,
    /// Hauteur du bassin au-dessus du sol.
    pub height: f32,
}

impl<'r, 'a> Quad<'r, 'a> {
    pub fn new(rig: &'r Rigging<'a>, legs: usize) -> Result<Self, PresetError> {
        let root = rig.bone(Chain::Root, 0).ok_or_else(|| incompatible("no hips bone"))?;
        let mut limbs = Vec::new();
        for (front, chain) in [(true, Chain::Arm as fn(Lr) -> Chain), (false, Chain::Leg)] {
            for side in Lr::BOTH {
                limbs.extend(Limb::new(rig, chain(side), side, front));
            }
        }
        if limbs.len() < legs {
            return Err(incompatible(format!("{legs} legs needed, {} found", limbs.len())));
        }
        Ok(Self {
            rig,
            root,
            spine: rig.chain(Chain::Spine),
            neck: rig.chain(Chain::Neck),
            head: rig.bone(Chain::Head, 0),
            tail: rig.chain(Chain::Tail),
            limbs,
            height: rig.height(rig.head(root)).max(0.05),
        })
    }

    /// Rotation du point d'appui autour de l'axe gauche (positif : le
    /// talon ou le poignet avance par rapport à l'appui).
    pub fn flex(&self, angle: f32) -> Quat {
        Quat::from_axis_angle(self.rig.w(Vec3::X), angle)
    }
}

pub(crate) const PARAMS: &[ParamSpec] = &[
    ParamSpec::new("speed", "Speed (m/s)", 0.2, 15.0, 1.0),
    ParamSpec {
        step: 1.0,
        ..ParamSpec::new("gait", "Gait (0 walk, 1 trot, 2 gallop)", 0.0, 2.0, 0.0)
    },
    ParamSpec::new("stride", "Stride length", 0.6, 1.5, 1.0),
    ParamSpec::new("bounce", "Bounce", 0.0, 2.0, 1.0),
    ParamSpec::new("tail", "Tail motion", 0.0, 2.0, 1.0),
];

/// Allure : facteur d'appui, phases (AvG, AvD, ArG, ArD), lever, rebond.
struct Style {
    duty: f32,
    phases: [f32; 4],
    lift: f32,
    bob: f32,
}

pub(crate) struct Gait<'r, 'a> {
    q: Quad<'r, 'a>,
    gait: u8,
    style: Style,
    stride: f32,
    period: f32,
    tail: f32,
    drops: Vec<f32>,
}

impl<'r, 'a> Gait<'r, 'a> {
    pub fn new(rig: &'r Rigging<'a>, p: &Params) -> Result<Self, PresetError> {
        let q = Quad::new(rig, 4)?;
        let gait = p.get(PARAMS, "gait") as u8;
        let style = match gait {
            0 => Style { duty: 0.65, phases: [0.25, 0.75, 0.0, 0.5], lift: 0.16, bob: 0.012 },
            1 => Style { duty: 0.42, phases: [0.5, 0.0, 0.0, 0.5], lift: 0.25, bob: 0.03 },
            _ => Style { duty: 0.36, phases: [0.62, 0.52, 0.0, 0.1], lift: 0.3, bob: 0.05 },
        };
        let v = p.get(PARAMS, "speed");
        let h = q.height;
        let froude = v * v / (9.81 * h);
        // Pattes plus courtes que l'animal réel (raideur de la colonne
        // ignorée) : foulée réduite de 20 %, 32 % au galop.
        let scale = if gait >= 2 { 0.68 } else { 0.8 };
        let stride = scale * 2.3 * h * froude.powf(0.3) * p.get(PARAMS, "stride");
        let mut g = Self {
            style: Style {
                bob: style.bob * p.get(PARAMS, "bounce"),
                ..style
            },
            q,
            gait,
            stride,
            period: stride / v,
            tail: p.get(PARAMS, "tail"),
            drops: Vec::new(),
        };
        let n = 96;
        let need: Vec<f32> = (0..n).map(|i| g.reach_drop(i as f32 / n as f32) + 0.002).collect();
        let accel = if g.gait >= 2 { 9.0 } else { 5.0 };
        g.drops = envelope(&need, g.period / n as f32, accel);
        Ok(g)
    }

    /// Bassin : déplacement canonique, tangage.
    fn body(&self, u: f32) -> (Vec3, f32) {
        let h = self.q.height;
        let s = &self.style;
        let y = match self.gait {
            0 => s.bob * h * cwave(2.0 * u),
            1 => -s.bob * h * cwave(2.0 * (u - s.duty * 0.5)),
            _ => s.bob * h * wave(u),
        };
        let pitch = if self.gait >= 2 { 0.1 * cwave(u - 0.3) } else { 0.0 };
        let sway = if self.gait == 0 { 0.01 * h * wave(u) } else { 0.0 };
        (Vec3::new(sway, y, self.stride * u), pitch)
    }

    /// Appui de la patte `i` à l'instant `u` : position, flexion distale,
    /// doigts à plat.
    fn paw(&self, i: usize, u: f32) -> (Vec3, f32, f32) {
        let limb = &self.q.limbs[i];
        let s = &self.style;
        let phase = s.phases[i];
        let q = u - phase;
        let k = q.floor();
        let phi = q - k;
        let forward = self.q.rig.w(Vec3::Z);
        let at = |k: f32| limb.contact + forward * self.stride * (k + s.duty * 0.5 + phase);
        if phi < s.duty {
            let lift = smooth((phi / s.duty - 0.6) / 0.4);
            (at(k), 0.35 * lift, 1.0)
        } else {
            let w = (phi - s.duty) / (1.0 - s.duty);
            let (a, b) = (at(k), at(k + 1.0));
            let lift = s.lift * limb.reach * bump(w, 0.45);
            let fold = if limb.front { 1.1 } else { 0.7 };
            let flex = 0.35 * (1.0 - smooth(w / 0.3)) + fold * bump(w, 0.35);
            (a.lerp(b, minjerk(w)) + self.q.rig.frame.up * lift, flex, 1.0 - smooth(w / 0.3))
        }
    }

    fn reach_drop(&self, u: f32) -> f32 {
        let rig = self.q.rig;
        let root = rig.head(self.q.root);
        let (offset, pitch) = self.body(u);
        let r = Quat::from_axis_angle(rig.w(Vec3::X), pitch);
        let up = rig.frame.up;
        let mut drop: f32 = 0.0;
        for (i, limb) in self.q.limbs.iter().enumerate() {
            let attach = root + rig.w(offset) + r * (limb.attach(rig) - root);
            let (paw, flex, _) = self.paw(i, u);
            let c = &limb.chain;
            let hock = if c.len() > 2 { paw - self.q.flex(flex) * (limb.contact - rig.head(c[2])) } else { paw };
            let d = attach - hock;
            let dv = d.dot(up);
            let dh = (d - up * dv).length();
            let max = 0.98 * limb.reach;
            drop = drop.max(if dh < max { dv - (max * max - dh * dh).sqrt() } else { dv });
        }
        drop.max(0.01 * self.q.height)
    }
}

impl Motion for Gait<'_, '_> {
    fn duration(&self) -> f32 {
        self.period
    }

    fn pose(&self, u: f32, body: &mut Body) {
        let q = &self.q;
        body.reset();
        let (mut offset, pitch) = self.body(u);
        offset.y -= periodic(&self.drops, u);
        let walk = self.gait == 0;
        body.pelvis(q.root, offset, if walk { 0.04 * wave(u) } else { 0.0 }, pitch, 0.0);
        let flexion = if self.gait >= 2 { 0.3 * wave(u - 0.15) } else { 0.0 };
        let lateral = if walk { 0.08 * wave(u + 0.25) } else { 0.02 * wave(u) };
        body.spread(&q.spine, flexion - 0.5 * pitch, lateral, 0.0);
        body.ripple(&q.tail, Vec3::Y, 0.25 * self.tail, u, 0.35);
        body.ripple(&q.tail, Vec3::X, 0.08 * self.tail, 2.0 * u, 0.3);
        body.fk();
        for i in 0..q.limbs.len() {
            let (paw, flex, toe) = self.paw(i, u);
            q.limbs[i].place(body, paw, q.flex(flex), toe);
        }
        let nod = match self.gait {
            0 => 0.06 * cwave(2.0 * u),
            1 => 0.02 * cwave(2.0 * u),
            _ => 0.12 * cwave(u - 0.1),
        };
        body.look(&q.neck, q.head, nod - 0.05, -lateral * 0.3, 0.0);
    }
}
