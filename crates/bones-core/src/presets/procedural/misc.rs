//! Créatures et objets : queue qui remue, tentacule, corde qui se balance,
//! pendule (période physique), mécanismes (roue et piston synchronisés).

use std::f32::consts::{PI, TAU};

use glam::{Quat, Vec3};

use super::Motion;
use super::ctx::{Body, Rigging};
use super::curves::{cwave, wave};
use crate::BoneId;
use crate::anim::physics::GRAVITY;
use crate::presets::error::{PresetError, incompatible};
use crate::presets::params::{ParamSpec, Params};
use crate::presets::roles::Chain;

/// Chaînes animées : la queue si elle existe, sinon les chaînes libres,
/// sinon tous les os depuis la racine.
fn chains(rig: &Rigging, prefer_tail: bool) -> Vec<Vec<BoneId>> {
    if prefer_tail {
        let tail = rig.chain(Chain::Tail);
        if !tail.is_empty() {
            return vec![tail];
        }
    }
    let mut out: Vec<Vec<BoneId>> = rig
        .roles
        .chains()
        .into_iter()
        .filter(|c| matches!(c, Chain::Branch(_) | Chain::Tail | Chain::Spine))
        .map(|c| rig.chain(c))
        .collect();
    if out.is_empty() {
        out.push(rig.skeleton.ids().collect());
    }
    // La racine fait partie de la première chaîne (pied d'une plante, tête d'une corde).
    if let Some(r) = rig.bone(Chain::Root, 0)
        && !prefer_tail
        && rig.skeleton.bone(r).parent().is_none()
    {
        out[0].insert(0, r);
    }
    out
}

pub(crate) const WAG: &[ParamSpec] = &[
    ParamSpec::new("rate", "Wags per second", 0.3, 8.0, 3.0),
    ParamSpec::new("amplitude", "Amplitude", 0.1, 2.0, 1.0),
    ParamSpec::new("lag", "Whip (phase lag)", 0.0, 1.0, 0.3),
];

pub(crate) const TENTACLE: &[ParamSpec] = &[
    ParamSpec::new("rate", "Waves per second", 0.1, 3.0, 0.5),
    ParamSpec::new("amplitude", "Amplitude", 0.1, 2.0, 1.0),
    ParamSpec::new("wavelength", "Waves along the chain", 0.3, 3.0, 1.0),
];

/// Articulation d'une chaîne ondulante : os, position normalisée le long de
/// la chaîne, poids (longueur relative) et deux axes de flexion canoniques.
type WaveJoint = (BoneId, f32, f32, Vec3, Vec3);

/// Ondulation d'une ou plusieurs chaînes : queue qui remue (dans le plan
/// horizontal) ou tentacule (deux axes, mouvement tournoyant).
pub(crate) struct Wave {
    chains: Vec<Vec<WaveJoint>>,
    rate: f32,
    amplitude: f32,
    lag: f32,
    swirl: bool,
}

impl Wave {
    pub fn new(rig: &Rigging, p: &Params, tentacle: bool) -> Result<Self, PresetError> {
        let specs = if tentacle { TENTACLE } else { WAG };
        let lists = chains(rig, !tentacle);
        let chains = lists
            .into_iter()
            .map(|bones| {
                let total: f32 = bones.iter().map(|b| rig.len(*b)).sum::<f32>().max(1e-4);
                let mut at = 0.0;
                bones
                    .into_iter()
                    .map(|b| {
                        let len = rig.len(b);
                        let s = (at + 0.5 * len) / total;
                        at += len;
                        // Axes canoniques ⟂ à l'os : flexion principale et secondaire.
                        let dir = rig.c(rig.dir(b));
                        let a = dir.cross(Vec3::Y).try_normalize().unwrap_or(Vec3::X);
                        let (a1, a2) = if tentacle { (a, dir.cross(a)) } else { (Vec3::Y, a) };
                        (b, s, len / total, a1, a2)
                    })
                    .collect()
            })
            .collect();
        Ok(Self {
            chains,
            rate: p.get(specs, "rate"),
            amplitude: p.get(specs, "amplitude"),
            lag: if tentacle {
                p.get(TENTACLE, "wavelength")
            } else {
                p.get(WAG, "lag")
            },
            swirl: tentacle,
        })
    }
}

impl Motion for Wave {
    fn duration(&self) -> f32 {
        if self.swirl {
            1.0 / self.rate
        } else {
            2.0 / self.rate
        }
    }

    fn pose(&self, u: f32, body: &mut Body) {
        body.reset();
        let cycles = if self.swirl { 1.0 } else { 2.0 };
        for chain in &self.chains {
            for &(b, s, weight, a1, a2) in chain {
                let phase = cycles * u - self.lag * s;
                let k = self.amplitude * weight * (0.4 + s) * if self.swirl { 2.5 } else { 1.6 };
                body.turn(Some(b), a1, k * wave(phase));
                if self.swirl {
                    body.turn(Some(b), a2, 0.6 * k * cwave(phase));
                }
            }
        }
        body.fk();
    }
}

pub(crate) const SWING: &[ParamSpec] = &[
    ParamSpec::new("amplitude", "Amplitude (deg)", 2.0, 80.0, 25.0),
    ParamSpec::new("bend", "Rope bend", 0.0, 2.0, 1.0),
];

/// Corde ou pendule : la chaîne oscille autour de son point d'attache avec
/// la période d'un pendule pesant, T = 2π·√(L/g)·(1 + θ₀²/16) ; une corde
/// se courbe en plus avec retard (mode d'onde le long de la corde).
pub(crate) struct Swing {
    chains: Vec<Vec<BoneId>>,
    amplitude: f32,
    bend: f32,
    period: f32,
}

impl Swing {
    pub fn new(rig: &Rigging, p: &Params, rope: bool) -> Result<Self, PresetError> {
        let chains = chains(rig, false);
        let length: f32 = chains[0].iter().map(|b| rig.len(*b)).sum();
        if length < 1e-4 {
            return Err(incompatible("chain too short"));
        }
        let amplitude = p.get(SWING, "amplitude").to_radians();
        // Corde : pendule composé (centre d'oscillation aux 2/3) ; pendule : masse au bout.
        let l = if rope { 2.0 / 3.0 * length } else { length };
        let period = TAU * (l / GRAVITY).sqrt() * (1.0 + amplitude * amplitude / 16.0);
        Ok(Self {
            chains,
            amplitude,
            bend: if rope { p.get(SWING, "bend") } else { 0.0 },
            period,
        })
    }
}

impl Motion for Swing {
    fn duration(&self) -> f32 {
        self.period
    }

    fn pose(&self, u: f32, body: &mut Body) {
        body.reset();
        let a = self.amplitude;
        for chain in &self.chains {
            let n = chain.len().max(1) as f32;
            for (i, b) in chain.iter().enumerate() {
                if i == 0 {
                    body.turn(Some(*b), Vec3::X, a * cwave(u));
                    body.turn(Some(*b), Vec3::Z, 0.08 * a * self.bend * wave(u));
                } else {
                    let s = i as f32 / n;
                    let lag = 0.25 * self.bend * a / n * (1.0 + s) * wave(u - 0.15 - 0.2 * s);
                    body.turn(Some(*b), Vec3::X, -lag);
                }
            }
        }
        body.fk();
    }
}

pub(crate) const MACHINE: &[ParamSpec] = &[
    ParamSpec::new("rate", "Turns per second", 0.05, 20.0, 0.5),
    ParamSpec::new("stroke", "Piston stroke (m)", 0.0, 2.0, 0.15),
    ParamSpec::toggle("reverse", "Reverse", false),
];

/// Mécanisme : les roues (os nommés wheel, gear, rotor, fan, propeller…)
/// tournent autour de leur axe, les pistons (piston, slider, plunger…)
/// coulissent en phase, comme entraînés par une bielle. Sans os nommés,
/// la racine tourne autour de la verticale.
pub(crate) struct Machine {
    wheels: Vec<BoneId>,
    pistons: Vec<BoneId>,
    spin_root: Option<BoneId>,
    rate: f32,
    stroke: f32,
    sign: f32,
}

impl Machine {
    pub fn new(rig: &Rigging, p: &Params) -> Result<Self, PresetError> {
        let named = |keys: &[&str]| -> Vec<BoneId> {
            rig.skeleton
                .ids()
                .filter(|b| {
                    let n = rig.skeleton.bone(*b).name.to_lowercase();
                    keys.iter().any(|k| n.contains(k))
                })
                .collect()
        };
        let wheels = named(&[
            "wheel",
            "gear",
            "rotor",
            "fan",
            "propeller",
            "turbine",
            "cog",
            "spin",
            "axle",
        ]);
        let pistons = named(&["piston", "slider", "plunger", "pump", "press"]);
        let spin_root = (wheels.is_empty() && pistons.is_empty())
            .then(|| rig.bone(Chain::Root, 0).or(rig.skeleton.ids().next()))
            .flatten();
        Ok(Self {
            wheels,
            pistons,
            spin_root,
            rate: p.get(MACHINE, "rate"),
            stroke: p.get(MACHINE, "stroke"),
            sign: if p.flag(MACHINE, "reverse") { -1.0 } else { 1.0 },
        })
    }
}

impl Motion for Machine {
    fn duration(&self) -> f32 {
        1.0 / self.rate
    }

    fn pose(&self, u: f32, body: &mut Body) {
        body.reset();
        let angle = self.sign * TAU * u;
        for b in &self.wheels {
            // Rotation autour de l'axe propre de l'os (Y local).
            body.pose.bones[b.index()].rotation = Quat::from_rotation_y(angle);
        }
        for b in &self.pistons {
            // Course d'une bielle-manivelle : (1 − cos θ) / 2.
            let x = self.stroke * 0.5 * (1.0 - (angle + PI).cos());
            body.pose.bones[b.index()].translation = Vec3::Y * x;
        }
        if let Some(r) = self.spin_root {
            body.turn(Some(r), Vec3::Y, angle);
        }
        body.fk();
    }
}
