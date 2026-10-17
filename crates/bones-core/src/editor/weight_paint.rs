//! Mode « Weight Paint » : traits de pinceau, outils du menu « Weights » et
//! protection des poids peints. L'éditeur tient le trait en cours et les
//! caches ; le moteur ([`crate::weight_paint`]) fait les calculs sur les
//! sommets. Messages affichés en anglais.

use std::sync::Arc;

use glam::Vec2;

use super::Editor;
use crate::weight_paint::{self, MeshCaches, PaintTool, Stroke};
use crate::{BoneId, Viewport, WeightsTool, mirror_name};

impl Editor {
    /// Os dont on peint les poids : l'os sélectionné.
    pub fn paint_bone(&self) -> Option<BoneId> {
        self.selected.filter(|b| self.rig.skeleton.contains(*b))
    }

    /// Pression du crayon (0 à 1 ; 1 au doigt ou à la souris), appliquée à la
    /// force et au rayon selon les réglages du pinceau.
    pub fn set_pointer_pressure(&mut self, pressure: f32) {
        self.paint.pressure = if pressure.is_finite() {
            pressure.clamp(0.0, 1.0)
        } else {
            1.0
        };
    }

    /// Position écran et rayon (pixels) du pinceau pendant un trait.
    pub fn brush_cursor(&self) -> Option<(Vec2, f32)> {
        self.paint.stroke.as_ref().map(|s| (s.last_pos, s.last_radius))
    }

    /// Vrai si les poids ont été peints (ou retouchés par un outil) : ils ne
    /// sont plus recalculés automatiquement quand le squelette change.
    pub fn weights_painted(&self) -> bool {
        self.rig.weights_painted()
    }

    /// Outil du menu « Weights » (une étape d'annulation).
    pub fn apply_weights_tool(&mut self, tool: WeightsTool) {
        if matches!(tool, WeightsTool::Automatic) {
            // Efface le drapeau et relance la liaison : le calcul (parfois long,
            // p. ex. diffusion de chaleur) repart sur un fil de fond via
            // `bind_job`, sans bloquer l'interface.
            self.checkpoint();
            self.rig.set_painted(false);
            self.request_rebind();
            self.status = Some("Recomputing weights…".into());
            return;
        }
        let bone = self.paint_bone();
        let needs_bone = !matches!(
            tool,
            WeightsTool::NormalizeAll | WeightsTool::LimitTotal { .. } | WeightsTool::Mirror
        );
        let usable =
            self.rig.mesh.vertex_count() > 0 && self.rig.skin.weights.len() == self.rig.mesh.vertex_count();
        if (needs_bone && bone.is_none()) || !usable {
            if needs_bone && bone.is_none() {
                self.status = Some("Select a bone first".into());
            }
            return;
        }
        self.ensure_caches();
        self.checkpoint();
        let changed = {
            let caches = self.paint.caches.as_ref().unwrap();
            let skin = Arc::make_mut(&mut self.rig.skin);
            weight_paint::run_weights_tool(
                skin,
                &self.rig.skeleton,
                &self.rig.mesh,
                caches,
                bone,
                tool,
                &self.weight_paint,
            )
        };
        if changed {
            self.rig.bump_skin();
            self.rig.set_painted(true);
            self.status = Some(tool.label().to_string());
        } else {
            self.drop_checkpoint(false);
        }
    }

    /// Début de geste du mode « Weight Paint ».
    pub(super) fn paint_down(&mut self, pos: Vec2, vp: &Viewport, radius: f32) -> bool {
        let hit = self.pick(pos, vp, radius);
        let Some(bone) = self.paint_bone() else {
            // Pas encore d'os : un toucher en sélectionne un.
            if let Some(h) = hit {
                self.selected = Some(h);
            }
            return false;
        };
        if self.weight_paint.brush.tool == PaintTool::Sample {
            self.sample_at(pos, vp);
            return false;
        }
        if self.rig.mesh.vertex_count() == 0 || self.rig.skin.weights.len() != self.rig.mesh.vertex_count() {
            if let Some(h) = hit {
                self.selected = Some(h);
            }
            return false;
        }
        self.ensure_caches();
        self.checkpoint();
        self.begin_stroke(bone, pos, vp, hit);
        true
    }

    pub(super) fn paint_move(&mut self, pos: Vec2, vp: &Viewport) {
        if self.paint.stroke.is_none() {
            return;
        }
        let _ = vp;
        match self.weight_paint.brush.tool {
            PaintTool::Gradient => self.gradient_move(pos),
            PaintTool::Sample => {}
            _ => self.segment_move(pos),
        }
    }

    pub(super) fn paint_up(&mut self) {
        let Some(stroke) = self.paint.stroke.take() else {
            return;
        };
        if !self.drag_moved {
            // Simple toucher : sélectionne l'os sous le doigt, annule l'éventuelle touche.
            if let Some(b) = stroke.pending_select {
                self.selected = Some(b);
            }
            self.drop_checkpoint(true);
        } else if !stroke.changed {
            // Trait sans effet : rien à retenir.
            self.drop_checkpoint(true);
        }
        // Sinon : le point de reprise devient une étape d'annulation.
    }

    pub(super) fn paint_cancel(&mut self) {
        if self.paint.stroke.take().is_some() {
            self.drop_checkpoint(true);
        }
    }

    // ------------------------------------------------------------------

    /// Reconstruit les caches du maillage si sa révision a changé.
    fn ensure_caches(&mut self) {
        let rev = self.rig.revisions().mesh;
        if self.paint.caches.as_ref().map(|c| c.rev) != Some(rev) {
            self.paint.caches = Some(MeshCaches::build(&self.rig.mesh, rev));
        }
    }

    /// Os symétrique peint aux sommets miroir, ou `None`.
    fn mirror_paint_bone(&self, bone: BoneId) -> Option<BoneId> {
        if !self.weight_paint.mirror_x {
            return None;
        }
        let name = &self.rig.skeleton.bone(bone).name;
        match mirror_name(name) {
            Some(n) => self.rig.skeleton.find(&n),
            // Os central (`.x`) : peint symétriquement sur lui-même.
            None => Some(bone),
        }
    }

    fn begin_stroke(&mut self, bone: BoneId, pos: Vec2, vp: &Viewport, hit: Option<BoneId>) {
        let matrices = self.skinning_matrices();
        let (screen, front) =
            weight_paint::project_vertices(&self.rig.mesh, &self.rig.skin, &matrices, &self.camera, vp);
        let mirror_bone = self.mirror_paint_bone(bone);
        let deform = self
            .rig
            .skeleton
            .ids()
            .map(|id| self.rig.skeleton.bone(id).deform)
            .collect();
        let cell = self.weight_paint.brush.radius.max(8.0);
        let mut stroke = Stroke::new(
            screen,
            front,
            &self.rig.skin,
            bone,
            mirror_bone,
            deform,
            cell,
            hit,
        );
        stroke.begin_at(pos);
        self.paint.stroke = Some(stroke);
        if self.weight_paint.brush.tool != PaintTool::Gradient {
            self.apply_dab(pos);
        }
    }

    fn brush_radius_pressure(&self) -> (f32, f32) {
        let b = &self.weight_paint.brush;
        let r = (b.radius
            * if b.pressure_radius {
                self.paint.pressure
            } else {
                1.0
            })
        .max(1.0);
        (r, self.paint.pressure)
    }

    fn apply_dab(&mut self, center: Vec2) {
        let (radius, pressure) = self.brush_radius_pressure();
        {
            let stroke = self.paint.stroke.as_mut().unwrap();
            let caches = self.paint.caches.as_ref().unwrap();
            let skin = Arc::make_mut(&mut self.rig.skin);
            stroke.dab(center, radius, pressure, &self.weight_paint, caches, skin);
        }
        self.after_paint();
    }

    fn segment_move(&mut self, pos: Vec2) {
        let (radius, pressure) = self.brush_radius_pressure();
        let spacing = (self.weight_paint.brush.spacing.max(0.01) * 2.0 * radius).max(1.0);
        {
            let stroke = self.paint.stroke.as_mut().unwrap();
            let caches = self.paint.caches.as_ref().unwrap();
            let skin = Arc::make_mut(&mut self.rig.skin);
            stroke.segment(pos, radius, pressure, spacing, &self.weight_paint, caches, skin);
        }
        self.after_paint();
    }

    fn gradient_move(&mut self, pos: Vec2) {
        {
            let stroke = self.paint.stroke.as_mut().unwrap();
            let caches = self.paint.caches.as_ref().unwrap();
            let skin = Arc::make_mut(&mut self.rig.skin);
            stroke.gradient(pos, &self.weight_paint, caches, skin);
        }
        self.after_paint();
    }

    fn after_paint(&mut self) {
        self.rig.bump_skin();
        if self.paint.stroke.as_ref().is_some_and(|s| s.changed) {
            self.rig.set_painted(true);
        }
    }

    /// Pipette : le poids de l'os sous le doigt devient celui du pinceau (pas
    /// d'historique).
    fn sample_at(&mut self, pos: Vec2, vp: &Viewport) {
        let Some(bone) = self.paint_bone() else { return };
        if self.rig.mesh.vertex_count() == 0 {
            return;
        }
        let matrices = self.skinning_matrices();
        let (screen, front) =
            weight_paint::project_vertices(&self.rig.mesh, &self.rig.skin, &matrices, &self.camera, vp);
        let radius = self.weight_paint.brush.radius.max(8.0);
        // Sommet le plus proche sous le doigt, faces avant prioritaires.
        let key = |(_, d, f): &(usize, f32, bool)| if *f { *d } else { *d + 1.0e6 };
        let best = (0..screen.len())
            .filter_map(|v| {
                let s = screen[v]?;
                let d = (s - pos).length();
                (d <= radius).then_some((v, d, front[v]))
            })
            .min_by(|a, b| key(a).total_cmp(&key(b)));
        if let Some((v, _, _)) = best {
            let w = self.rig.skin.weights.get(v).map_or(0.0, |vw| vw.weight_of(bone));
            self.weight_paint.brush.weight = w;
            self.status = Some(format!("Sampled weight {w:.3}"));
        }
    }
}
