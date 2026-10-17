//! Trait de pinceau en cours : copie de travail flottante des poids, suivi des
//! atténuations pour l'écrêtage par trait (hors accumulation) et application
//! des touches (dabs) le long du geste.

use glam::Vec2;
use rayon::prelude::*;

use super::caches::MeshCaches;
use super::grid::ScreenGrid;
use super::influence::{self, Influences};
use super::{PaintTool, WeightPaintSettings};
use crate::skinning::MAX_INFLUENCES;
use crate::{BoneId, Skin};

/// Cible d'une touche : mode de mélange (Draw) ou valeur imposée (outils
/// Average / Blur / Smear / Gradient).
#[derive(Clone, Copy)]
pub(super) enum Op {
    Blend,
    Value(f32),
}

pub(crate) struct Stroke {
    pub bone: BoneId,
    /// Os symétrique peint aux sommets miroir (`None` sans symétrie).
    pub(super) mirror_bone: Option<BoneId>,
    pub(super) screen: Vec<Option<Vec2>>,
    pub(super) front: Vec<bool>,
    grid: ScreenGrid,
    pub(super) work: Vec<Influences>,
    pub(super) deform: Vec<bool>,
    // Écrêtage par trait (hors accumulation) : origine et atténuation maximale.
    orig: Vec<f32>,
    applied: Vec<f32>,
    touch: Vec<u32>,
    pub last_pos: Vec2,
    pub last_radius: f32,
    pub(super) start_pos: Vec2,
    residual: f32,
    pub changed: bool,
    pub pending_select: Option<BoneId>,
}

impl Stroke {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        screen: Vec<Option<Vec2>>,
        front: Vec<bool>,
        skin: &Skin,
        bone: BoneId,
        mirror_bone: Option<BoneId>,
        deform: Vec<bool>,
        cell: f32,
        pending_select: Option<BoneId>,
    ) -> Self {
        let n = screen.len();
        let work = (0..n)
            .into_par_iter()
            .map(|i| skin.weights.get(i).map(influence::decode).unwrap_or_default())
            .collect();
        let grid = ScreenGrid::new(&screen, cell);
        Self {
            bone,
            mirror_bone,
            grid,
            work,
            deform,
            orig: vec![0.0; n],
            applied: vec![0.0; n],
            touch: vec![u32::MAX; n],
            last_pos: Vec2::ZERO,
            last_radius: cell,
            start_pos: Vec2::ZERO,
            residual: 0.0,
            changed: false,
            pending_select,
            screen,
            front,
        }
    }

    /// Fixe le point de départ du trait (première touche et dégradé).
    pub fn begin_at(&mut self, pos: Vec2) {
        self.start_pos = pos;
        self.last_pos = pos;
    }

    /// Applique une touche à `bone` au sommet `v` (atténuation `alpha`), écrit
    /// le résultat quantifié dans `skin`.
    fn apply(
        &mut self,
        v: usize,
        bone: BoneId,
        alpha: f32,
        op: Op,
        s: &WeightPaintSettings,
        skin: &mut Skin,
    ) {
        if alpha <= 1e-6 {
            return;
        }
        let cur = influence::weight_of(&self.work[v], bone);
        let (base, a) = if s.brush.accumulate {
            (cur, alpha.min(1.0))
        } else {
            if self.touch[v] != bone.0 {
                self.touch[v] = bone.0;
                self.orig[v] = cur;
                self.applied[v] = 0.0;
            }
            let a = self.applied[v].max(alpha).min(1.0);
            self.applied[v] = a;
            (self.orig[v], a)
        };
        let tgt = match op {
            Op::Blend => s.brush.blend.target(base, s.brush.weight),
            Op::Value(x) => x,
        };
        let new = (base + (tgt - base) * a).clamp(0.0, 1.0);
        influence::set_weight(&mut self.work[v], bone, new);
        influence::limit(&mut self.work[v], MAX_INFLUENCES, Some(bone));
        if s.auto_normalize {
            influence::auto_normalize(&mut self.work[v], bone, &self.deform);
        }
        skin.weights[v] = influence::encode(&self.work[v]);
        self.changed = true;
    }

    /// Peint le sommet direct puis, en symétrie, le sommet miroir avec l'os
    /// symétrique (même atténuation ; la valeur Draw est recalculée côté miroir).
    fn paint(
        &mut self,
        v: usize,
        alpha: f32,
        op: Op,
        mirror: &[u32],
        s: &WeightPaintSettings,
        skin: &mut Skin,
    ) {
        self.apply(v, self.bone, alpha, op, s, skin);
        if let Some(mb) = self.mirror_bone {
            let m = mirror[v] as usize;
            if m != v {
                self.apply(m, mb, alpha, op, s, skin);
            }
        }
    }

    /// Sommet le plus proche d'un point écran, à moins de `max` pixels.
    fn nearest(&self, p: Vec2, max: f32) -> Option<usize> {
        let mut cand = Vec::new();
        self.grid.query(p, max, &mut cand);
        cand.iter()
            .filter_map(|&j| self.screen[j as usize].map(|s| (j as usize, (s - p).length())))
            .filter(|(_, d)| *d <= max)
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(v, _)| v)
    }

    /// Une touche de pinceau centrée en `center`, de rayon `radius`.
    pub fn dab(
        &mut self,
        center: Vec2,
        radius: f32,
        pressure: f32,
        s: &WeightPaintSettings,
        caches: &MeshCaches,
        skin: &mut Skin,
    ) {
        let prev = self.last_pos;
        self.last_pos = center;
        self.last_radius = radius;
        let brush = &s.brush;
        let inv_r = 1.0 / radius.max(1e-3);
        let mut cand = Vec::new();
        self.grid.query(center, radius, &mut cand);
        let affected: Vec<(usize, f32)> = cand
            .iter()
            .filter_map(|&vi| {
                let v = vi as usize;
                if brush.front_faces_only && !self.front[v] {
                    return None;
                }
                let d = (self.screen[v]? - center).length();
                (d < radius).then_some((v, d * inv_r))
            })
            .collect();
        if affected.is_empty() {
            return;
        }
        let pstr = if brush.pressure_strength { pressure } else { 1.0 };
        let mirror = &caches.mirror;
        match brush.tool {
            PaintTool::Draw => {
                for (v, t) in affected {
                    let alpha = brush.falloff.strength(t) * brush.strength * pstr;
                    self.paint(v, alpha, Op::Blend, mirror, s, skin);
                }
            }
            PaintTool::Average => {
                let sum: f32 = affected
                    .iter()
                    .map(|(v, _)| influence::weight_of(&self.work[*v], self.bone))
                    .sum();
                let tgt = sum / affected.len() as f32;
                for (v, t) in affected {
                    let alpha = brush.falloff.strength(t) * brush.strength * pstr;
                    self.paint(v, alpha, Op::Value(tgt), mirror, s, skin);
                }
            }
            PaintTool::Blur => {
                for (v, t) in affected {
                    let nb = caches.neighbors_of(v);
                    if nb.is_empty() {
                        continue;
                    }
                    let tgt = nb
                        .iter()
                        .map(|&r| influence::weight_of(&self.work[r as usize], self.bone))
                        .sum::<f32>()
                        / nb.len() as f32;
                    let alpha = brush.falloff.strength(t) * brush.strength * pstr;
                    self.paint(v, alpha, Op::Value(tgt), mirror, s, skin);
                }
            }
            PaintTool::Smear => {
                let delta = center - prev;
                for (v, t) in affected {
                    let Some(src) = self.screen[v].and_then(|sp| self.nearest(sp - delta, radius)) else {
                        continue;
                    };
                    let tgt = influence::weight_of(&self.work[src], self.bone);
                    let alpha = brush.falloff.strength(t) * brush.strength * pstr;
                    self.paint(v, alpha, Op::Value(tgt), mirror, s, skin);
                }
            }
            PaintTool::Gradient | PaintTool::Sample => {}
        }
    }

    /// Place des touches régulièrement espacées de `prev` à `to` (espacement
    /// `spacing` px), en gardant le reliquat d'une portion à l'autre.
    #[allow(clippy::too_many_arguments)]
    pub fn segment(
        &mut self,
        to: Vec2,
        radius: f32,
        pressure: f32,
        spacing: f32,
        s: &WeightPaintSettings,
        caches: &MeshCaches,
        skin: &mut Skin,
    ) {
        let from = self.last_pos;
        let dist = (to - from).length();
        let dir = if dist > 1e-6 {
            (to - from) / dist
        } else {
            Vec2::ZERO
        };
        let spacing = spacing.max(1.0);
        let mut acc = self.residual;
        let mut walked = 0.0;
        loop {
            let step = spacing - acc;
            if walked + step > dist {
                break;
            }
            walked += step;
            acc = 0.0;
            self.dab(from + dir * walked, radius, pressure, s, caches, skin);
        }
        self.residual = acc + (dist - walked);
        self.last_pos = to;
    }
}
