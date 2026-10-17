//! Commandes de la galerie, à la Procreate : capsule de boutons texte
//! (« Select », « Import »), bouton rond à glyphe (« + ») et pastille de
//! sélection cochée.

use egui::{Color32, CornerRadius, FontId, Painter, Pos2, Rect, Response, Sense, Stroke, Ui, vec2};

use super::{Motion, anim};
use crate::icons::{self, Icon};
use crate::theme::{ACCENT, DANGER, DISABLED, ICON, PANEL, TEXT, TEXT_LIST, WIDGET, mix, semibold};

/// Hauteur de la capsule et diamètre du bouton rond.
const HEIGHT: f32 = 44.0;
/// Marge horizontale de chaque libellé.
const PAD: f32 = 16.0;

/// Ton d'un libellé de la capsule.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CapsuleTone {
    Plain,
    /// Bouton dont la fenêtre est ouverte : bleu d'accent.
    Active,
    /// Action de validation (« Done ») : bleu d'accent, semi-gras.
    Accent,
    /// Action destructrice (« Delete ») : rouge.
    Danger,
}

/// Bouton d'une capsule.
#[derive(Clone, Copy, Debug)]
pub struct CapsuleItem<'a> {
    pub label: &'a str,
    pub enabled: bool,
    pub tone: CapsuleTone,
}

impl<'a> CapsuleItem<'a> {
    pub fn new(label: &'a str) -> Self {
        Self {
            label,
            enabled: true,
            tone: CapsuleTone::Plain,
        }
    }

    pub fn tone(self, tone: CapsuleTone) -> Self {
        Self { tone, ..self }
    }

    pub fn enabled(self, enabled: bool) -> Self {
        Self { enabled, ..self }
    }
}

/// Résultat d'une capsule : bouton touché et zone de chaque bouton (pour
/// ancrer une fenêtre contextuelle).
pub struct CapsuleResponse {
    pub clicked: Option<usize>,
    pub rects: Vec<Rect>,
}

fn font(tone: CapsuleTone) -> FontId {
    match tone {
        CapsuleTone::Accent => semibold(17.0),
        _ => FontId::proportional(17.0),
    }
}

/// Capsule pleine de 44 pt groupant des boutons texte de 17 pt (barre de
/// la galerie). Un bouton pressé s'assombrit ; un bouton désactivé est gris.
pub fn capsule(ui: &mut Ui, items: &[CapsuleItem<'_>]) -> CapsuleResponse {
    let galleys: Vec<_> = items
        .iter()
        .map(|item| {
            ui.painter()
                .layout_no_wrap(item.label.to_owned(), font(item.tone), TEXT)
        })
        .collect();
    let width: f32 = galleys.iter().map(|g| g.size().x + 2.0 * PAD).sum::<f32>() + 8.0;
    let (rect, response) = ui.allocate_exact_size(vec2(width, HEIGHT), Sense::hover());
    let mut result = CapsuleResponse {
        clicked: None,
        rects: Vec::with_capacity(items.len()),
    };
    if ui.is_rect_visible(rect) {
        ui.painter().rect_filled(rect, CornerRadius::same(22), PANEL);
    }
    let mut x = rect.left() + 4.0;
    for (i, (item, galley)) in items.iter().zip(&galleys).enumerate() {
        let w = galley.size().x + 2.0 * PAD;
        let cell = Rect::from_min_size(egui::pos2(x, rect.top()), vec2(w, HEIGHT));
        x += w;
        let sense = if item.enabled {
            Sense::click()
        } else {
            Sense::hover()
        };
        let cell_response = ui.interact(cell, response.id.with(i), sense);
        let motion = Motion::of(ui, &cell_response);
        if motion.hover > 0.0 {
            let fill = Color32::from_white_alpha((8.0 * motion.hover) as u8);
            ui.painter()
                .rect_filled(cell.shrink2(vec2(0.0, 4.0)), CornerRadius::same(18), fill);
        }
        let base = match (item.enabled, item.tone) {
            (false, _) => DISABLED,
            (true, CapsuleTone::Plain) => TEXT_LIST,
            (true, CapsuleTone::Active | CapsuleTone::Accent) => ACCENT,
            (true, CapsuleTone::Danger) => DANGER,
        };
        let color = base.gamma_multiply(1.0 - 0.45 * motion.press);
        let galley = ui
            .painter()
            .layout_no_wrap(item.label.to_owned(), font(item.tone), color);
        ui.painter()
            .galley(cell.center() - galley.size() * 0.5, galley, color);
        if cell_response.clicked() {
            result.clicked = Some(i);
        }
        result.rects.push(cell);
    }
    result
}

/// Bouton rond de 44 pt, même fond que la capsule, glyphe de 22 pt (« + ») ;
/// actif : glyphe bleu.
pub fn round_button(ui: &mut Ui, icon: Icon, active: bool, hint: &str) -> Response {
    let (rect, response) = ui.allocate_exact_size(vec2(HEIGHT, HEIGHT), Sense::click());
    if ui.is_rect_visible(rect) {
        let motion = Motion::of(ui, &response);
        let on = anim(ui, response.id.with("active"), active);
        let fill = mix(PANEL, WIDGET, motion.hover.max(motion.press));
        ui.painter().circle_filled(rect.center(), HEIGHT * 0.5, fill);
        let size = 22.0 * (1.0 - 0.08 * motion.press);
        icons::paint(
            ui.painter(),
            Rect::from_center_size(rect.center(), vec2(size, size)),
            icon,
            mix(ICON, ACCENT, on),
        );
    }
    response.on_hover_text(hint)
}

/// Pastille de sélection : cercle cerclé de blanc sur fond sombre, puis
/// disque bleu et coche blanche quand `on` (0 → 1) atteint 1.
pub fn check_badge(painter: &Painter, center: Pos2, radius: f32, on: f32) {
    let on = on.clamp(0.0, 1.0);
    painter.circle_filled(center, radius, mix(Color32::from_black_alpha(90), ACCENT, on));
    painter.circle_stroke(
        center,
        radius - 0.75,
        Stroke::new(1.5, mix(Color32::WHITE, ACCENT, on)),
    );
    if on > 0.0 {
        let size = radius * 1.25 * (0.6 + 0.4 * on);
        let glyph = Rect::from_center_size(center, vec2(size, size));
        icons::paint(painter, glyph, Icon::Check, Color32::WHITE.gamma_multiply(on));
    }
}
