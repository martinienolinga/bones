//! Ligne générique d'un arbre de scène (outliner), à la Procreate / Blender :
//! guides d'indentation, chevron de repli, glyphe teinté, nom, et à droite
//! deux bascules monochromes (œil de visibilité, cadenas de verrou).
//!
//! Le widget ne décide rien : il dessine et renvoie les réponses des
//! sous-zones (ligne entière, chevron, œil, cadenas). L'appelant traite
//! d'abord les bascules et le chevron, et ne sélectionne la ligne que si
//! aucune d'elles n'a été touchée (les zones se recouvrent).

use egui::{Align2, Color32, CornerRadius, FontId, Rect, Response, Sense, Stroke, Ui, pos2, vec2};

use super::{Motion, anim};
use crate::icons::{self, Icon};
use crate::theme::{ACCENT, ICON, TEXT_FAINT, TEXT_LIST, WIDGET_HOVER, mix, semibold};

const ROW: f32 = 44.0;
const INSET: f32 = 14.0;
const STEP: f32 = 16.0;
/// Largeur tactile d'une bascule de droite.
const TOGGLE: f32 = 34.0;

/// Description d'une ligne d'outliner.
pub struct RowSpec<'a> {
    pub depth: usize,
    pub icon: Icon,
    /// Teinte du glyphe au repos (couleur de côté pour un os, grise sinon).
    pub tint: Color32,
    pub name: &'a str,
    pub selected: bool,
    pub has_children: bool,
    pub collapsed: bool,
    /// Affiche l'œil et le cadenas (faux pour l'armature et les os).
    pub toggles: bool,
    pub visible: bool,
    pub locked: bool,
}

/// Réponses des sous-zones d'une ligne.
pub struct RowResponse {
    pub row: Response,
    pub chevron: Option<Response>,
    pub eye: Option<Response>,
    pub lock: Option<Response>,
}

impl RowResponse {
    /// Vrai si une bascule ou le chevron a été touché (la ligne ne doit
    /// alors pas se sélectionner).
    pub fn handle_consumed(&self) -> bool {
        let hit = |r: &Option<Response>| r.as_ref().is_some_and(|r| r.clicked());
        hit(&self.chevron) || hit(&self.eye) || hit(&self.lock)
    }
}

/// Dessine une ligne d'outliner sur toute la largeur disponible.
pub fn outliner_row(ui: &mut Ui, spec: RowSpec<'_>) -> RowResponse {
    let width = ui.available_width();
    let (rect, row) = ui.allocate_exact_size(vec2(width, ROW), Sense::click());
    let painter = ui.painter();

    // Fond : pilule bleue si sélectionnée, sinon léger retour au survol.
    if spec.selected {
        painter.rect_filled(rect, CornerRadius::same(10), ACCENT);
    } else {
        let motion = Motion::of(ui, &row);
        let t = motion.hover * 0.5 + motion.press;
        if t > 0.0 {
            ui.painter().rect_filled(
                rect,
                CornerRadius::same(10),
                WIDGET_HOVER.gamma_multiply(t.min(1.0)),
            );
        }
    }
    let painter = ui.painter();

    // Guides d'indentation.
    let base = rect.left() + INSET;
    for level in 0..spec.depth {
        let x = base + 7.0 + level as f32 * STEP;
        painter.line_segment(
            [pos2(x, rect.top() + 6.0), pos2(x, rect.bottom() - 6.0)],
            Stroke::new(
                1.0,
                Color32::from_white_alpha(if spec.selected { 40 } else { 18 }),
            ),
        );
    }

    let on_fill = Color32::WHITE;
    // Chevron de repli.
    let chevron_x = base + spec.depth as f32 * STEP;
    let chevron = spec.has_children.then(|| {
        let hit = Rect::from_min_size(pos2(chevron_x - 2.0, rect.top()), vec2(22.0, ROW));
        let r = ui.interact(hit, row.id.with("chevron"), Sense::click());
        let color = if spec.selected { on_fill } else { TEXT_FAINT };
        chevron(
            ui.painter(),
            pos2(chevron_x + 7.0, rect.center().y),
            spec.collapsed,
            color,
        );
        r
    });

    // Glyphe de l'élément.
    let painter = ui.painter();
    let glyph_c = pos2(chevron_x + 22.0, rect.center().y);
    let glyph = Rect::from_center_size(glyph_c, vec2(19.0, 19.0));
    let tint = if spec.selected { on_fill } else { spec.tint };
    icons::paint(painter, glyph, spec.icon, tint);

    // Bascules de droite (cadenas puis œil), si demandées.
    let mut right = rect.right() - 4.0;
    let (mut eye, mut lock) = (None, None);
    if spec.toggles {
        let lock_rect = Rect::from_min_size(pos2(right - TOGGLE, rect.top()), vec2(TOGGLE, ROW));
        lock = Some(toggle(
            ui,
            lock_rect,
            row.id.with("lock"),
            Icon::Lock,
            spec.locked,
            spec.selected,
        ));
        right -= TOGGLE;
        let eye_rect = Rect::from_min_size(pos2(right - TOGGLE, rect.top()), vec2(TOGGLE, ROW));
        eye = Some(toggle(
            ui,
            eye_rect,
            row.id.with("eye"),
            Icon::Eye,
            spec.visible,
            spec.selected,
        ));
        right -= TOGGLE + 2.0;
    }

    // Nom, tronqué avant les bascules.
    let name_left = glyph.right() + 10.0;
    let (font, color) = if spec.selected {
        (semibold(16.0), on_fill)
    } else if spec.visible {
        (FontId::proportional(16.0), TEXT_LIST)
    } else {
        (FontId::proportional(16.0), TEXT_FAINT)
    };
    let clip = Rect::from_min_max(pos2(name_left, rect.top()), pos2(right, rect.bottom()));
    let painter = ui
        .painter()
        .with_clip_rect(clip.intersect(ui.painter().clip_rect()));
    painter.text(
        pos2(name_left, rect.center().y),
        Align2::LEFT_CENTER,
        spec.name,
        font,
        color,
    );

    RowResponse {
        row,
        chevron,
        eye,
        lock,
    }
}

/// Petit triangle de repli : pointe à droite (replié) ou vers le bas.
fn chevron(painter: &egui::Painter, center: egui::Pos2, collapsed: bool, color: Color32) {
    let s = 4.0;
    let pts = if collapsed {
        vec![
            center + vec2(-s * 0.6, -s),
            center + vec2(s * 0.8, 0.0),
            center + vec2(-s * 0.6, s),
        ]
    } else {
        vec![
            center + vec2(-s, -s * 0.6),
            center + vec2(s, -s * 0.6),
            center + vec2(0.0, s * 0.8),
        ]
    };
    painter.add(egui::Shape::convex_polygon(pts, color, Stroke::NONE));
}

/// Bascule monochrome de droite : glyphe actif (blanc / accent) ou estompé.
fn toggle(ui: &mut Ui, rect: Rect, id: egui::Id, icon: Icon, on: bool, selected: bool) -> Response {
    let r = ui.interact(rect, id, Sense::click());
    let anim_on = anim(ui, id.with("on"), on);
    let glyph = Rect::from_center_size(rect.center(), vec2(19.0, 19.0));
    let color = if selected {
        Color32::WHITE.gamma_multiply(0.55 + 0.45 * anim_on)
    } else if icon == Icon::Lock {
        mix(TEXT_FAINT, ACCENT, anim_on)
    } else {
        mix(TEXT_FAINT, ICON, anim_on)
    };
    let press = Motion::of(ui, &r).press;
    icons::paint(ui.painter(), glyph, icon, color.gamma_multiply(1.0 - 0.3 * press));
    r
}
