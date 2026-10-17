//! Outil « Gradient » : dégradé linéaire ou radial du début à la fin du geste.
//! Il est recalculé entièrement à chaque mouvement, à partir de la copie de
//! travail figée au début du trait (jamais modifiée par le dégradé), de sorte
//! que déplacer l'extrémité met le résultat à jour sans accumulation.

use glam::Vec2;

use super::caches::MeshCaches;
use super::influence::{self, Influences};
use super::stroke::Stroke;
use super::{Falloff, GradientKind, WeightPaintSettings};
use crate::skinning::MAX_INFLUENCES;
use crate::{BoneId, Skin};

/// Rampe 0 → 1 le long de la ligne (linéaire), selon la courbe d'atténuation.
fn ramp(falloff: Falloff, t: f32) -> f32 {
    match falloff {
        Falloff::Constant => t,
        _ => 1.0 - falloff.strength(t),
    }
}

impl Stroke {
    /// Valeur du dégradé à la position écran `s`, ou `None` hors zone (radial).
    fn gradient_value(
        &self,
        s: Vec2,
        start: Vec2,
        dir: Vec2,
        len: f32,
        set: &WeightPaintSettings,
    ) -> Option<f32> {
        let w = set.brush.weight;
        match set.brush.gradient {
            GradientKind::Linear => {
                let t = ((s - start).dot(dir) / len).clamp(0.0, 1.0);
                Some(w * ramp(set.brush.falloff, t))
            }
            GradientKind::Radial => {
                // Plein au centre, nul au rayon (courbe d'atténuation) ; au-delà
                // du rayon, le sommet n'est pas touché.
                let t = (s - start).length() / len;
                (t <= 1.0).then(|| w * set.brush.falloff.strength(t.min(0.999)))
            }
        }
    }

    fn mix(&self, infl: &mut Influences, bone: BoneId, value: f32, strength: f32, set: &WeightPaintSettings) {
        let base = influence::weight_of(infl, bone);
        influence::set_weight(infl, bone, base + (value - base) * strength);
        influence::limit(infl, MAX_INFLUENCES, Some(self.bone));
        if set.auto_normalize {
            influence::auto_normalize(infl, self.bone, &self.deform);
        }
    }

    /// Applique le dégradé du début du trait jusqu'à `end`.
    pub fn gradient(&mut self, end: Vec2, set: &WeightPaintSettings, caches: &MeshCaches, skin: &mut Skin) {
        self.last_pos = end;
        let start = self.start_pos;
        let dir = end - start;
        let len = dir.length().max(1e-3);
        let dir = dir / len;
        let strength = set.brush.strength;
        let front_only = set.brush.front_faces_only;
        for v in 0..self.screen.len() {
            let mut infl = self.work[v].clone();
            let mut touched = false;
            if let Some(s) = self.screen[v]
                && !(front_only && !self.front[v])
                && let Some(value) = self.gradient_value(s, start, dir, len, set)
            {
                self.mix(&mut infl, self.bone, value, strength, set);
                touched = true;
            }
            if let Some(mb) = self.mirror_bone {
                let p = caches.mirror[v] as usize;
                if p != v
                    && let Some(ps) = self.screen[p]
                    && !(front_only && !self.front[p])
                    && let Some(value) = self.gradient_value(ps, start, dir, len, set)
                {
                    self.mix(&mut infl, mb, value, strength, set);
                    touched = true;
                }
            }
            // Recalcul complet : les sommets hors zone reprennent l'état initial.
            skin.weights[v] = influence::encode(&infl);
            self.changed |= touched;
        }
    }
}
