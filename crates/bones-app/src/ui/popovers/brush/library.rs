//! Section « Brush » : grille des outils de peinture (Draw, Blur, Average,
//! Smear, Gradient, Sample) ; sous « Draw », les modes de mélange de Blender
//! ; sous « Gradient », la forme du dégradé. Aperçus en carte de chaleur
//! (aplats, glyphes blancs).

use bones_core::Editor;
use bones_core::weight_paint::{Blend, GradientKind, PaintTool};
use bones_ui::icons;
use bones_ui::{theme, widgets};
use egui::{Color32, CornerRadius, Painter, Rect, Stroke, vec2};

use super::{grid, tool_icon};

pub fn show(ui: &mut egui::Ui, editor: &mut Editor) {
    let weight = editor.weight_paint.brush.weight;
    widgets::section_title(ui, "Tool");
    let tool = editor.weight_paint.brush.tool;
    let mut pick = None;
    grid(ui, PaintTool::ALL.len(), |ui, i, cell| {
        let t = PaintTool::ALL[i];
        let paint = |p: &Painter, r: Rect| tool_preview(p, r, t, weight);
        if widgets::tile(ui, cell, t.label(), t == tool, paint).clicked() {
            pick = Some(t);
        }
    });
    if let Some(t) = pick {
        editor.weight_paint.brush.tool = t;
    }

    match editor.weight_paint.brush.tool {
        PaintTool::Draw => blends(ui, editor, weight),
        PaintTool::Gradient => gradients(ui, editor),
        _ => {}
    }
}

/// Modes de mélange du pinceau Draw, en vignettes marquées d'un opérateur.
fn blends(ui: &mut egui::Ui, editor: &mut Editor, weight: f32) {
    widgets::section_title(ui, "Blend");
    let current = editor.weight_paint.brush.blend;
    let mut pick = None;
    grid(ui, Blend::ALL.len(), |ui, i, cell| {
        let b = Blend::ALL[i];
        let paint = |p: &Painter, r: Rect| {
            p.rect_filled(r, CornerRadius::same(8), theme::heat(weight));
            blend_symbol(p, r, b);
        };
        if widgets::tile(ui, cell, b.label(), b == current, paint).clicked() {
            pick = Some(b);
        }
    });
    if let Some(b) = pick {
        editor.weight_paint.brush.blend = b;
    }
}

/// Forme du dégradé (linéaire ou radial) en aperçu d'aplats.
fn gradients(ui: &mut egui::Ui, editor: &mut Editor) {
    widgets::section_title(ui, "Gradient");
    let current = editor.weight_paint.brush.gradient;
    let mut pick = None;
    grid(ui, GradientKind::ALL.len(), |ui, i, cell| {
        let k = GradientKind::ALL[i];
        let paint = |p: &Painter, r: Rect| gradient_preview(p, r, k);
        if widgets::tile(ui, cell, k.label(), k == current, paint).clicked() {
            pick = Some(k);
        }
    });
    if let Some(k) = pick {
        editor.weight_paint.brush.gradient = k;
    }
}

/// Vignette d'outil : carré de chaleur au poids du pinceau, glyphe blanc.
fn tool_preview(p: &Painter, r: Rect, tool: PaintTool, weight: f32) {
    p.rect_filled(r, CornerRadius::same(8), theme::heat(weight));
    let s = r.height().min(r.width()) * 0.5;
    let glyph = Rect::from_center_size(r.center(), vec2(s, s));
    icons::paint(p, glyph, tool_icon(tool), Color32::WHITE);
}

/// Opérateur du mode de mélange, tracé en traits blancs (sans police).
fn blend_symbol(p: &Painter, r: Rect, blend: Blend) {
    let c = r.center();
    let s = r.height().min(r.width()) * 0.22;
    let stroke = Stroke::new(3.0, Color32::WHITE);
    let line = |a: egui::Pos2, b: egui::Pos2| {
        p.line_segment([a, b], stroke);
    };
    match blend {
        Blend::Mix => {
            line(c + vec2(-s, -s * 0.5), c + vec2(s, -s * 0.5));
            line(c + vec2(-s, s * 0.5), c + vec2(s, s * 0.5));
        }
        Blend::Add => {
            line(c + vec2(-s, 0.0), c + vec2(s, 0.0));
            line(c + vec2(0.0, -s), c + vec2(0.0, s));
        }
        Blend::Subtract => line(c + vec2(-s, 0.0), c + vec2(s, 0.0)),
        Blend::Multiply => {
            line(c + vec2(-s, -s), c + vec2(s, s));
            line(c + vec2(-s, s), c + vec2(s, -s));
        }
        Blend::Lighten => {
            line(c + vec2(-s, s * 0.6), c + vec2(0.0, -s * 0.8));
            line(c + vec2(0.0, -s * 0.8), c + vec2(s, s * 0.6));
        }
        Blend::Darken => {
            line(c + vec2(-s, -s * 0.6), c + vec2(0.0, s * 0.8));
            line(c + vec2(0.0, s * 0.8), c + vec2(s, -s * 0.6));
        }
    }
}

/// Aperçu de dégradé : bandes verticales (linéaire) ou cadres (radial) de la
/// rampe de chaleur, en aplats.
fn gradient_preview(p: &Painter, r: Rect, kind: GradientKind) {
    const N: usize = 6;
    match kind {
        GradientKind::Linear => {
            let w = r.width() / N as f32;
            for i in 0..N {
                let x = r.left() + i as f32 * w;
                let band = Rect::from_min_max(egui::pos2(x, r.top()), egui::pos2(x + w, r.bottom()));
                p.rect_filled(band, CornerRadius::ZERO, theme::heat(i as f32 / (N - 1) as f32));
            }
        }
        GradientKind::Radial => {
            for i in (0..N).rev() {
                let t = i as f32 / (N - 1) as f32;
                let rect = Rect::from_center_size(r.center(), r.size() * (1.0 - t * 0.85));
                p.rect_filled(rect, CornerRadius::same(4), theme::heat(t));
            }
        }
    }
}
