//! Carte d'un préréglage : vignette (le mannequin qui joue l'animation en
//! boucle, ou son visage pour une expression ; squelette en bâtons pour un
//! préréglage d'animal ou de végétal) surmontant le nom et un sous-titre.
//! Toucher ouvre la feuille d'application ; appui long sur un préréglage
//! utilisateur, son menu.

use std::time::Duration;

use bones_core::presets::Preset;
use bones_ui::icons::{self, Icon};
use bones_ui::theme;
use egui::{Align2, Color32, CornerRadius, FontId, Rect, Sense, Stroke, StrokeKind, Vec2, pos2, vec2};

use super::library::Entry;
use super::stage::Tile;
use super::{Kind, PresetsState, View, grid};

/// Cadence des vignettes animées (le cycle est cuit à 30 images/s).
pub const FRAME: Duration = Duration::from_millis(33);
/// Délai de relève des vignettes en préparation.
const POLL: Duration = Duration::from_millis(60);

/// Présentation des cartes d'une galerie.
#[derive(Clone, Copy)]
pub struct Look {
    /// Hauteur de la vignette rapportée à sa largeur.
    pub ratio: f32,
    /// Marge autour du cadrage du mannequin.
    pub margin: f32,
    pub time: f32,
    pub fallback: Icon,
}

impl Look {
    /// Cartes d'une galerie : portrait pour un corps entier, carré pour un
    /// visage.
    pub fn of(kind: Kind, time: f32) -> Self {
        let (ratio, margin) = match kind {
            Kind::Animations => (1.22, 0.07),
            Kind::Expressions => (1.0, 0.05),
        };
        Self {
            ratio,
            margin,
            time,
            fallback: super::fallback_icon(kind),
        }
    }
}

/// Durée d'appui qui ouvre le menu de la carte.
const LONG_PRESS: f64 = 0.5;

/// Dessine la carte ; retourne le délai avant la prochaine image voulue
/// (vignette animée ou en préparation), `None` si elle est fixe ou cachée.
pub fn show(
    ui: &mut egui::Ui,
    p: &mut PresetsState,
    entry: Entry,
    cell: Vec2,
    look: Look,
) -> Option<Duration> {
    let (rect, response) = ui.allocate_exact_size(cell, Sense::click());
    let is_user = matches!(entry, Entry::User(_));
    interact(ui, p, entry, is_user, &response);
    if !ui.is_rect_visible(rect) {
        return None;
    }

    let press = ui.ctx().animate_bool_with_time(
        response.id.with("press"),
        response.is_pointer_button_down_on(),
        0.12,
    );
    let hover = ui
        .ctx()
        .animate_bool_with_time(response.id.with("hover"), response.hovered(), 0.14);

    let full = Rect::from_min_size(rect.min, vec2(rect.width(), rect.width() * look.ratio));
    let thumb = scale(full, 1.0 - 0.03 * press);
    let radius = CornerRadius::same(12);
    let bg = theme::mix(theme::WIDGET, theme::WIDGET_HOVER, hover.max(press));
    ui.painter().rect_filled(thumb, radius, bg);

    // Vignette rendue (mannequin), sinon squelette ; repli sur un glyphe
    // tant qu'elle se prépare.
    let preset = p.library.preset(entry);
    let name = preset.info.name.clone();
    let subtitle = subtitle(&preset.info);
    let wait = thumbnail(ui, p, entry, full.size(), thumb, look);
    // Pastille des préréglages utilisateur (coin haut droit).
    if is_user {
        ui.painter()
            .circle_filled(thumb.right_top() + vec2(-11.0, 11.0), 4.0, theme::ACCENT);
    }
    if press > 0.0 {
        ui.painter()
            .rect_filled(thumb, radius, Color32::from_black_alpha((50.0 * press) as u8));
    }
    if hover > 0.0 {
        ui.painter().rect_stroke(
            thumb,
            radius,
            Stroke::new(1.5, theme::ACCENT.gamma_multiply(hover)),
            StrokeKind::Inside,
        );
    }

    // Nom (blanc) et sous-titre (gris) sous la vignette, alignés sur la
    // vignette au repos (ils ne suivent pas l'enfoncement).
    let painter = ui.painter();
    let name_galley = painter.layout_no_wrap(name, theme::semibold(13.0), theme::TEXT);
    painter.galley(pos2(full.left(), full.bottom() + 5.0), name_galley, theme::TEXT);
    painter.text(
        pos2(full.left(), full.bottom() + 22.0),
        Align2::LEFT_TOP,
        subtitle,
        FontId::proportional(11.0),
        theme::TEXT_DIM,
    );
    wait
}

/// Vignette d'un préréglage dans `thumb` (rendue à la taille `size` de la
/// vignette au repos, pour ne pas la redessiner pendant l'enfoncement).
/// Retourne le délai avant la prochaine image voulue.
pub fn thumbnail(
    ui: &egui::Ui,
    p: &mut PresetsState,
    entry: Entry,
    size: Vec2,
    thumb: Rect,
    look: Look,
) -> Option<Duration> {
    let preset: &Preset = p.library.preset(entry);
    match p.stage.tile(ui.ctx(), preset, size, look.time, look.margin) {
        Tile::Texture(texture) => {
            egui::Image::from_texture((texture, size))
                .corner_radius(CornerRadius::same(12))
                .paint_at(ui, thumb);
            p.stage.animated(preset).then_some(FRAME)
        }
        Tile::Pending => {
            placeholder(ui, thumb, look.fallback);
            Some(POLL)
        }
        Tile::Unsupported => {
            let painter = ui.painter().with_clip_rect(thumb);
            if p.previews.draw(&painter, thumb, preset, look.time) {
                Some(FRAME)
            } else {
                placeholder(ui, thumb, look.fallback);
                Some(POLL)
            }
        }
    }
}

/// Repère d'une vignette en préparation : glyphe estompé au centre.
fn placeholder(ui: &egui::Ui, thumb: Rect, fallback: Icon) {
    let g = Rect::from_center_size(thumb.center(), vec2(26.0, 26.0));
    icons::paint(ui.painter(), g, fallback, theme::TEXT_FAINT);
}

/// Toucher (ouvre la feuille) et appui long (menu d'un préréglage utilisateur).
fn interact(ui: &egui::Ui, p: &mut PresetsState, entry: Entry, is_user: bool, response: &egui::Response) {
    let held = response.is_pointer_button_down_on()
        && ui.input(|i| {
            !i.pointer.is_decidedly_dragging()
                && i.pointer
                    .press_start_time()
                    .is_some_and(|t| i.time - t >= LONG_PRESS)
        });
    if is_user && (held || response.secondary_clicked()) && p.menu.is_none() {
        let pos = response.interact_pointer_pos().unwrap_or(response.rect.center());
        grid::open_menu(p, entry, pos);
    }
    if response.clicked() && p.menu.is_none() {
        p.view_slot = Some(View::Apply(entry));
    }
}

/// Sous-titre d'une carte : le groupe, ou à défaut la durée.
fn subtitle(info: &bones_core::presets::PresetInfo) -> String {
    if !info.group.trim().is_empty() {
        info.group.clone()
    } else if info.frames > 0 {
        format!("{:.1} s", info.duration())
    } else {
        "Pose".to_owned()
    }
}

fn scale(rect: Rect, factor: f32) -> Rect {
    Rect::from_center_size(rect.center(), rect.size() * factor)
}
