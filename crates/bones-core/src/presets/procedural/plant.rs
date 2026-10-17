//! Végétaux dans le vent : flexion statique dans le sens du vent,
//! oscillation qui remonte la hiérarchie avec un retard de phase (l'onde
//! part du tronc), amplitude croissante vers les extrémités souples,
//! mouvement elliptique, rafales, frémissement des feuilles et poids des
//! fleurs. Toutes les fréquences sont des harmoniques de la durée : la
//! boucle est parfaite.

use glam::Vec3;

use super::Motion;
use super::ctx::{Body, Rigging};
use super::curves::{bump, noise, wave};
use crate::BoneId;
use crate::presets::error::{PresetError, incompatible};
use crate::presets::params::{ParamSpec, Params};

pub(crate) const PARAMS: &[ParamSpec] = &[
    ParamSpec::new("wind", "Wind strength", 0.0, 2.0, 0.4),
    ParamSpec::new("gust", "Gust", 0.0, 1.5, 0.0),
    ParamSpec::new("frequency", "Sway frequency (Hz)", 0.05, 3.0, 0.45),
    ParamSpec::new("delay", "Wave delay", 0.0, 2.0, 0.35),
    ParamSpec::new("flutter", "Leaf flutter", 0.0, 2.0, 0.3),
    ParamSpec::new("stiffness", "Stiffness", 0.2, 3.0, 1.0),
    ParamSpec::new("droop", "Heavy top (droop)", 0.0, 1.0, 0.0),
    ParamSpec::new("direction", "Wind direction (deg)", -180.0, 180.0, 0.0),
    ParamSpec::new("duration", "Loop duration (s)", 2.0, 20.0, 8.0),
];

/// Os et sa place dans la plante.
struct Part {
    bone: BoneId,
    /// Abscisse le long de la plante (0 au pied, 1 aux extrémités).
    s: f32,
    /// Part de la hauteur portée par l'os (longueur relative).
    weight: f32,
    /// Extrémité fine (feuille, pétale, rameau terminal).
    leaf: bool,
}

pub(crate) struct Sway {
    parts: Vec<Part>,
    p: [f32; 8],
    duration: f32,
}

impl Sway {
    pub fn new(rig: &Rigging, p: &Params) -> Result<Self, PresetError> {
        let skeleton = rig.skeleton;
        if skeleton.is_empty() {
            return Err(incompatible("empty skeleton"));
        }
        // Distance depuis le pied de la plante le long de la hiérarchie.
        let mut start = vec![0.0f32; skeleton.len()];
        let mut has_children = vec![false; skeleton.len()];
        for (i, b) in skeleton.bones().iter().enumerate() {
            if let Some(par) = b.parent() {
                start[i] = start[par.index()] + rig.len(par);
                has_children[par.index()] = true;
            }
        }
        let total = skeleton
            .ids()
            .map(|b| start[b.index()] + rig.len(b))
            .fold(1e-3f32, f32::max);
        let parts = skeleton
            .ids()
            .map(|b| Part {
                bone: b,
                s: (start[b.index()] + 0.5 * rig.len(b)) / total,
                weight: rig.len(b) / total,
                leaf: !has_children[b.index()] && skeleton.depth(b) >= 2,
            })
            .collect();
        let get = |k| p.get(PARAMS, k);
        Ok(Self {
            parts,
            p: [
                get("wind"),
                get("gust"),
                get("frequency"),
                get("delay"),
                get("flutter"),
                get("stiffness"),
                get("droop"),
                get("direction").to_radians(),
            ],
            duration: get("duration"),
        })
    }
}

impl Motion for Sway {
    fn duration(&self) -> f32 {
        self.duration
    }

    fn pose(&self, u: f32, body: &mut Body) {
        let [wind, gust, freq, delay, flutter, stiffness, droop, dir] = self.p;
        body.reset();
        let w = Vec3::new(dir.sin(), 0.0, dir.cos());
        let bend_axis = Vec3::Y.cross(w);
        // Nombre entier d'oscillations par boucle.
        let cycles = (freq * self.duration).round().max(1.0);
        let fast = (4.0 * freq * self.duration).round().max(2.0);
        let g = 1.0 + 2.5 * gust * bump(u.rem_euclid(1.0), 0.3).powi(2);
        for part in &self.parts {
            let s = part.s;
            let soft = (0.3 + s) / stiffness;
            let seed = part.bone.0;
            let phase = cycles * u - delay * s;
            let lean = 0.5 * wind * g * soft;
            let sway = 0.35 * wind * g * soft * (0.8 * wave(phase) + 0.4 * noise(u, seed, &[cycles as u32 + 1, 2 * cycles as u32 + 1]));
            let cross = 0.15 * wind * g * soft * wave(phase + 0.25 + 0.1 * noise(u, seed + 7, &[1, 2]));
            body.turn(Some(part.bone), bend_axis, (lean + sway) * part.weight * 2.0);
            body.turn(Some(part.bone), w, cross * part.weight * 2.0);
            if droop > 0.0 {
                // Tête lourde : elle ploie vers le sol et hoche avec retard.
                let nod = droop * s * s * (1.0 + 0.6 * wind * wave(phase - 0.2));
                body.turn(Some(part.bone), bend_axis, nod * part.weight * 3.0);
            }
            if part.leaf && flutter > 0.0 {
                let f = 0.3 * flutter * (0.5 + wind) * noise(u, seed + 31, &[fast as u32, fast as u32 + 3, 2 * fast as u32 + 1]);
                body.turn(Some(part.bone), Vec3::Y, f);
                body.turn(Some(part.bone), bend_axis, 0.6 * f);
            }
        }
        body.fk();
    }
}
