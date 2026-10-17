//! Marche des animaux à plus de quatre pattes (insectes, araignées) :
//! tripode alterné à six pattes, tétrapode alterné à huit (Wilson 1966).
//! Chaque patte : rotation de la hanche autour de la verticale vers l'appui,
//! IK à deux os fémur–tibia genou en haut, segments distaux gardant leur
//! orientation de repos.

use glam::{Quat, Vec3};

use crate::BoneId;
use crate::presets::error::{PresetError, incompatible};
use crate::presets::params::{ParamSpec, Params};
use crate::presets::procedural::Motion;
use crate::presets::procedural::ctx::{Body, Rigging};
use crate::presets::procedural::curves::{bump, cwave, minjerk, wave};
use crate::presets::roles::{Chain, Lr};

pub(crate) const PARAMS: &[ParamSpec] = &[
    ParamSpec::new("speed", "Speed (m/s)", 0.01, 3.0, 0.12),
    ParamSpec::new("stride", "Stride length", 0.5, 1.5, 1.0),
    ParamSpec::new("lift", "Leg lift", 0.0, 2.0, 1.0),
];

struct Leg {
    bones: Vec<BoneId>,
    phase: f32,
    contact: Vec3,
    reach: f32,
}

pub(crate) struct Crawl<'r, 'a> {
    rig: &'r Rigging<'a>,
    root: BoneId,
    abdomen: Vec<BoneId>,
    head: Option<BoneId>,
    legs: Vec<Leg>,
    stride: f32,
    period: f32,
    duty: f32,
    lift: f32,
}

impl<'r, 'a> Crawl<'r, 'a> {
    pub fn new(rig: &'r Rigging<'a>, p: &Params) -> Result<Self, PresetError> {
        let root = rig.bone(Chain::Root, 0).ok_or_else(|| incompatible("no body bone"))?;
        let mut legs = Vec::new();
        for chain in rig.roles.chains() {
            if let Chain::LegPair(pair, side) = chain {
                let bones = rig.chain(chain);
                let Some(&last) = bones.last() else { continue };
                let contact = rig.tail(last);
                let flat = |v: Vec3| v - rig.frame.up * v.dot(rig.frame.up);
                let reach = flat(contact - rig.head(bones[0])).length().max(1e-3);
                let phase = ((pair as usize + (side == Lr::Right) as usize) % 2) as f32 * 0.5;
                legs.push(Leg { bones, phase, contact, reach });
            }
        }
        if legs.len() < 6 {
            return Err(incompatible("at least six legs needed"));
        }
        let reach = legs.iter().map(|l| l.reach).sum::<f32>() / legs.len() as f32;
        let stride = 0.7 * reach * p.get(PARAMS, "stride");
        Ok(Self {
            rig,
            root,
            abdomen: rig.chain(Chain::Spine),
            head: rig.bone(Chain::Head, 0),
            stride,
            period: stride / p.get(PARAMS, "speed"),
            duty: 0.55,
            lift: 0.35 * reach * p.get(PARAMS, "lift"),
            legs,
        })
    }

    fn foot(&self, leg: &Leg, u: f32) -> Vec3 {
        let q = u - leg.phase;
        let k = q.floor();
        let phi = q - k;
        let forward = self.rig.w(Vec3::Z);
        let at = |k: f32| leg.contact + forward * self.stride * (k + self.duty * 0.5 + leg.phase);
        if phi < self.duty {
            at(k)
        } else {
            let w = (phi - self.duty) / (1.0 - self.duty);
            at(k).lerp(at(k + 1.0), minjerk(w)) + self.rig.frame.up * self.lift * bump(w, 0.45)
        }
    }

    fn place(&self, body: &mut Body, leg: &Leg, target: Vec3) {
        let rig = self.rig;
        let up = rig.frame.up;
        let b = &leg.bones;
        let flat = |v: Vec3| (v - up * v.dot(up)).normalize_or(Vec3::Z);
        let carried = body.rotation(self.root) * rig.rest[self.root.index()].rotation.inverse();
        match b.len() {
            1 => body.aim(b[0], target - body.head(b[0])),
            2 => {
                let pole = body.head(b[0]) + up * leg.reach;
                body.two_bone(b[0], b[1], target, pole);
            }
            _ => {
                // Hanche : tournée vers l'appui autour de la verticale.
                let now = flat(body.tail(b[0]) - body.head(b[0]));
                let want = flat(target - body.head(b[0]));
                let angle = now.cross(want).dot(up).atan2(now.dot(want));
                body.rotate(b[0], Quat::from_axis_angle(up, angle));
                // Segments distaux : orientation de repos (suivant le corps).
                let distal: Vec3 = b[3..].iter().map(|d| carried * (rig.tail(*d) - rig.head(*d))).sum();
                let knee = target - distal;
                let pole = (body.head(b[1]) + knee) * 0.5 + up * leg.reach;
                body.two_bone(b[1], b[2], knee, pole);
                for d in &b[3..] {
                    body.set_rotation(*d, carried * rig.rest[d.index()].rotation);
                }
            }
        }
    }
}

impl Motion for Crawl<'_, '_> {
    fn duration(&self) -> f32 {
        self.period
    }

    fn pose(&self, u: f32, body: &mut Body) {
        body.reset();
        let bob = 0.04 * self.lift * cwave(2.0 * u);
        body.pelvis(self.root, Vec3::new(0.0, bob, self.stride * u), 0.03 * wave(u), 0.0, 0.02 * wave(u));
        body.spread(&self.abdomen, 0.03 * cwave(2.0 * u), -0.06 * wave(u), 0.0);
        body.fk();
        for leg in &self.legs {
            let target = self.foot(leg, u);
            self.place(body, leg, target);
        }
        body.look(&[], self.head, 0.0, 0.0, 0.0);
    }
}
