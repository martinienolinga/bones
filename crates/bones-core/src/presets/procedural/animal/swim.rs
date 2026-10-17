//! Nage du poisson et reptation du serpent.
//!
//! Poisson : onde de courbure qui parcourt le corps de la tête vers la
//! queue, d'amplitude croissante (nage carangiforme, Lighthill 1960).
//! Serpent : ondulation latérale « suivi du meneur » — chaque point du corps
//! passe exactement par le chemin tracé par la tête, une courbe serpenoïde
//! (Hirose 1993) : pas de glissement latéral.

use glam::{Vec2, Vec3};

use crate::BoneId;
use crate::presets::error::{PresetError, incompatible};
use crate::presets::params::{ParamSpec, Params};
use crate::presets::procedural::Motion;
use crate::presets::procedural::ctx::{Body, Rigging};
use crate::presets::procedural::curves::wave;
use crate::presets::roles::{Chain, Lr};

pub(crate) const SWIM: &[ParamSpec] = &[
    ParamSpec::new("rate", "Tail beats per second", 0.3, 6.0, 1.6),
    ParamSpec::new("amplitude", "Amplitude", 0.2, 2.0, 1.0),
    ParamSpec::new("speed", "Speed (body lengths/s)", 0.0, 4.0, 0.0),
];

/// Os du corps (racine, colonne, cou, tête, queue), dans l'ordre de la hiérarchie.
fn body_bones(rig: &Rigging) -> Vec<BoneId> {
    let mut bones: Vec<BoneId> = [Chain::Root, Chain::Spine, Chain::Neck, Chain::Head, Chain::Tail]
        .iter()
        .flat_map(|c| rig.chain(*c))
        .collect();
    bones.sort();
    bones
}

pub(crate) struct Swim {
    root: Option<BoneId>,
    /// (os, abscisse de la tête à la queue 0..1, longueur relative, signe).
    segments: Vec<(BoneId, f32, f32, f32)>,
    fins: Vec<(BoneId, f32)>,
    length: f32,
    rate: f32,
    amplitude: f32,
    speed: f32,
}

impl Swim {
    pub fn new(rig: &Rigging, p: &Params) -> Result<Self, PresetError> {
        let bones = body_bones(rig);
        if bones.len() < 2 {
            return Err(incompatible("no body chain"));
        }
        let fwd = rig.w(Vec3::Z);
        let z = |v: Vec3| v.dot(fwd);
        let front = bones.iter().map(|b| z(rig.head(*b)).max(z(rig.tail(*b)))).fold(f32::MIN, f32::max);
        let back = bones.iter().map(|b| z(rig.head(*b)).min(z(rig.tail(*b)))).fold(f32::MAX, f32::min);
        let length = (front - back).max(1e-3);
        let segments = bones
            .iter()
            .map(|b| {
                let (h, t) = (z(rig.head(*b)), z(rig.tail(*b)));
                let s = (front - (h + t) * 0.5) / length;
                let sign = if t < h { 1.0 } else { -1.0 };
                (*b, s, rig.len(*b) / length, sign)
            })
            .collect();
        let fins = Lr::BOTH
            .iter()
            .flat_map(|s| rig.chain(Chain::Arm(*s)).into_iter().map(move |b| (b, s.sign())))
            .collect();
        Ok(Self {
            root: rig.bone(Chain::Root, 0),
            segments,
            fins,
            length,
            rate: p.get(SWIM, "rate"),
            amplitude: p.get(SWIM, "amplitude"),
            speed: p.get(SWIM, "speed"),
        })
    }
}

impl Motion for Swim {
    fn duration(&self) -> f32 {
        1.0 / self.rate
    }

    fn pose(&self, u: f32, body: &mut Body) {
        body.reset();
        let a = self.amplitude;
        let travel = self.speed * self.length * self.duration() * u;
        if let Some(r) = self.root {
            body.shift(Some(r), Vec3::new(0.0, 0.0, travel));
        }
        for &(b, s, len, sign) in &self.segments {
            // Courbure : amplitude croissante vers la queue, une longueur
            // d'onde par longueur de corps.
            let envelope = 0.15 + 0.85 * s * s;
            let k = 2.4 * a * envelope * len * wave(u - s);
            body.turn(Some(b), Vec3::Y, sign * k);
        }
        // Recul de la tête et nageoires pectorales qui battent.
        body.turn(self.root, Vec3::Y, -0.04 * a * wave(u + 0.1));
        for &(b, side) in &self.fins {
            body.turn(Some(b), Vec3::Z, side * 0.25 * wave(2.0 * u));
        }
        body.fk();
    }
}

pub(crate) const SLITHER: &[ParamSpec] = &[
    ParamSpec::new("speed", "Speed (m/s)", 0.05, 3.0, 0.4),
    ParamSpec::new("wavelength", "Wavelength (body lengths)", 0.2, 1.0, 0.45),
    ParamSpec::new("amplitude", "Amplitude", 0.2, 1.5, 1.0),
];

pub(crate) struct Slither<'r, 'a> {
    rig: &'r Rigging<'a>,
    root: BoneId,
    /// Os du corps et abscisses curvilignes de leur tête et de leur queue
    /// depuis le bout avant du corps.
    bones: Vec<(BoneId, f32, f32)>,
    head: Option<BoneId>,
    front: Vec3,
    lambda: f32,
    speed: f32,
    /// Chemin serpenoïde sur une longueur d'onde (x latéral, y avant).
    path: Vec<Vec2>,
}

impl<'r, 'a> Slither<'r, 'a> {
    pub fn new(rig: &'r Rigging<'a>, p: &Params) -> Result<Self, PresetError> {
        let root = rig.bone(Chain::Root, 0).ok_or_else(|| incompatible("no root bone"))?;
        let mut chain = vec![root];
        chain.extend(rig.chain(Chain::Spine));
        if chain.len() < 3 {
            return Err(incompatible("body chain too short"));
        }
        let fwd = rig.w(Vec3::Z);
        let (first, last) = (rig.head(chain[0]), rig.tail(*chain.last().unwrap()));
        let front = if first.dot(fwd) >= last.dot(fwd) { first } else { last };
        let bones: Vec<(BoneId, f32, f32)> = chain
            .iter()
            .map(|b| ((*b), (rig.head(*b) - front).length(), (rig.tail(*b) - front).length()))
            .collect();
        let length = bones.iter().map(|(_, h, t)| h.max(*t)).fold(0.0, f32::max).max(1e-3);
        let lambda = p.get(SLITHER, "wavelength") * length;
        let theta = 0.75 * p.get(SLITHER, "amplitude");
        let n = 512;
        let mut path = Vec::with_capacity(n + 1);
        let mut pos = Vec2::ZERO;
        path.push(pos);
        for i in 0..n {
            let sigma = (i as f32 + 0.5) / n as f32 * lambda;
            let a = theta * (std::f32::consts::TAU * sigma / lambda).sin();
            pos += Vec2::new(a.sin(), a.cos()) * lambda / n as f32;
            path.push(pos);
        }
        Ok(Self {
            rig,
            root,
            bones,
            head: rig.bone(Chain::Head, 0),
            front,
            lambda,
            speed: p.get(SLITHER, "speed"),
            path,
        })
    }

    /// Point du chemin à l'abscisse `sigma` (canonique : x latéral, z avant).
    fn at(&self, sigma: f32) -> Vec3 {
        let n = self.path.len() - 1;
        let drift = self.path[n].y;
        let turns = (sigma / self.lambda).floor();
        let x = (sigma / self.lambda - turns) * n as f32;
        let i = (x.floor() as usize).min(n - 1);
        let p = self.path[i].lerp(self.path[i + 1], x - i as f32);
        Vec3::new(p.x, 0.0, p.y + turns * drift)
    }
}

impl Motion for Slither<'_, '_> {
    fn duration(&self) -> f32 {
        self.lambda / self.speed
    }

    fn pose(&self, u: f32, body: &mut Body) {
        let rig = self.rig;
        body.reset();
        let head = u * self.lambda;
        let point = |d: f32| self.front + rig.w(self.at(head - d));
        let (_, d_head, _) = self.bones[0];
        body.shift(Some(self.root), rig.c(point(d_head) - rig.head(self.root)));
        body.fk();
        for &(b, _, d_tail) in &self.bones {
            let target = point(d_tail);
            body.aim(b, target - body.head(b));
        }
        if let Some(h) = self.head {
            let ahead = rig.w(self.at(head + 0.05 * self.lambda) - self.at(head));
            body.aim(h, ahead);
        }
    }
}
