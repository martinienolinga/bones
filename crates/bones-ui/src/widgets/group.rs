//! Listes groupées à la Procreate : un bloc #3A3A3C aux coins de 12 pt, des
//! lignes de 46 pt (texte de 17 pt, retrait de 16 pt) séparées par des
//! filets d'un demi-point. Une ligne sélectionnée devient une pilule bleue
//! pleine avec une case blanche cochée.

use egui::{
    Align2, Color32, CornerRadius, FontId, Frame, Rect, Response, Sense, Stroke, StrokeKind, Ui, pos2, vec2,
};

use super::{Motion, anim, switch};
use crate::icons::{self, Icon};
use crate::theme::{
    ACCENT, HAIRLINE, ICON_DIM, TEXT, TEXT_DIM, TEXT_FAINT, TEXT_LIST, WIDGET, WIDGET_HOVER, semibold,
};

const ROW: f32 = 46.0;
const INSET: f32 = 16.0;

/// Bloc de lignes en cours de construction (voir [`group`]).
pub struct Group<'a> {
    ui: &'a mut Ui,
    rows: usize,
    /// La ligne précédente était sélectionnée : pas de filet sous elle.
    after_selected: bool,
}

/// Bloc de liste groupée ; `add` y ajoute les lignes.
pub fn group<R>(ui: &mut Ui, add: impl FnOnce(&mut Group<'_>) -> R) -> R {
    Frame::new()
        .fill(WIDGET)
        .corner_radius(CornerRadius::same(12))
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 0.0;
            ui.set_width(ui.available_width());
            add(&mut Group {
                ui,
                rows: 0,
                after_selected: false,
            })
        })
        .inner
}

impl Group<'_> {
    /// Réserve une ligne, trace le filet qui la sépare de la précédente.
    fn row(&mut self, height: f32, sense: Sense, selected: bool) -> (Rect, Response) {
        let width = self.ui.available_width();
        let (rect, response) = self.ui.allocate_exact_size(vec2(width, height), sense);
        if self.rows > 0 && !self.after_selected && !selected {
            self.ui.painter().line_segment(
                [
                    pos2(rect.left() + INSET, rect.top()),
                    pos2(rect.right(), rect.top()),
                ],
                Stroke::new(0.5, HAIRLINE),
            );
        }
        self.rows += 1;
        self.after_selected = selected;
        if !selected {
            let motion = Motion::of(self.ui, &response);
            let t = motion.hover * 0.5 + motion.press;
            if t > 0.0 {
                let fill = WIDGET_HOVER.gamma_multiply(t.min(1.0));
                self.ui.painter().rect_filled(rect, CornerRadius::same(10), fill);
            }
        }
        (rect, response)
    }

    /// Ligne à cocher : sélectionnée, pilule bleue et case blanche cochée.
    pub fn check_row(&mut self, label: &str, selected: bool) -> Response {
        let (rect, response) = self.row(ROW, Sense::click(), selected);
        let on = anim(self.ui, response.id.with("sel"), selected);
        let painter = self.ui.painter();
        if on > 0.0 {
            painter.rect_filled(rect, CornerRadius::same(10), ACCENT.gamma_multiply(on));
        }
        let text = if selected { Color32::WHITE } else { TEXT_LIST };
        painter.text(
            pos2(rect.left() + INSET, rect.center().y),
            Align2::LEFT_CENTER,
            label,
            FontId::proportional(17.0),
            text,
        );
        let box_rect = Rect::from_center_size(pos2(rect.right() - 26.0, rect.center().y), vec2(22.0, 22.0));
        if selected {
            painter.rect_filled(box_rect, CornerRadius::same(6), Color32::WHITE);
            icons::paint(painter, box_rect.shrink(3.0), Icon::Check, ACCENT);
        } else {
            painter.rect_stroke(
                box_rect,
                CornerRadius::same(6),
                Stroke::new(1.5, TEXT_FAINT),
                StrokeKind::Inside,
            );
        }
        response
    }

    /// Ligne libellé + valeur grise à droite (lecture seule ou touchable).
    pub fn value_row(&mut self, label: &str, value: &str) -> Response {
        let (rect, response) = self.row(ROW, Sense::click(), false);
        let painter = self.ui.painter();
        let y = rect.center().y;
        painter.text(
            pos2(rect.left() + INSET, y),
            Align2::LEFT_CENTER,
            label,
            FontId::proportional(17.0),
            TEXT_LIST,
        );
        painter.text(
            pos2(rect.right() - INSET, y),
            Align2::RIGHT_CENTER,
            value,
            FontId::proportional(15.0),
            TEXT_DIM,
        );
        response
    }

    /// Ligne avec interrupteur ; description facultative sous le libellé.
    pub fn switch_row(&mut self, value: &mut bool, label: &str, description: Option<&str>) -> Response {
        let height = if description.is_some() { 58.0 } else { ROW };
        let (rect, mut response) = self.row(height, Sense::click(), false);
        let switch_rect =
            Align2::RIGHT_CENTER.anchor_size(rect.right_center() - vec2(INSET - 4.0, 0.0), switch::SIZE);
        if response.clicked() {
            *value = !*value;
            response.mark_changed();
        }
        let on = anim(self.ui, response.id.with("on"), *value);
        let press = Motion::of(self.ui, &response).press;
        switch::paint(self.ui, switch_rect, on, press);
        let painter = self.ui.painter();
        let left = rect.left() + INSET;
        match description {
            Some(d) => {
                painter.text(
                    pos2(left, rect.center().y + 1.0),
                    Align2::LEFT_BOTTOM,
                    label,
                    FontId::proportional(17.0),
                    TEXT_LIST,
                );
                painter.text(
                    pos2(left, rect.center().y + 3.0),
                    Align2::LEFT_TOP,
                    d,
                    FontId::proportional(13.0),
                    TEXT_DIM,
                );
            }
            None => {
                painter.text(
                    pos2(left, rect.center().y),
                    Align2::LEFT_CENTER,
                    label,
                    FontId::proportional(17.0),
                    TEXT_LIST,
                );
            }
        }
        response
    }

    /// Ligne de fichier : glyphe gris, titre, sous-titre et pastille.
    pub fn icon_row(
        &mut self,
        icon: Icon,
        title: &str,
        subtitle: Option<&str>,
        badge: Option<&str>,
    ) -> Response {
        let (rect, response) = self.row(54.0, Sense::click(), false);
        let painter = self.ui.painter();
        let glyph = Rect::from_center_size(
            pos2(rect.left() + INSET + 11.0, rect.center().y),
            vec2(22.0, 22.0),
        );
        icons::paint(painter, glyph, icon, ICON_DIM);
        let mut right = rect.right() - INSET;
        if let Some(badge) = badge {
            let galley = painter.layout_no_wrap(badge.to_owned(), semibold(11.0), TEXT);
            let pill = Align2::RIGHT_CENTER
                .anchor_size(pos2(right, rect.center().y), galley.size() + vec2(14.0, 8.0));
            painter.rect_filled(pill, CornerRadius::same(8), WIDGET_HOVER);
            painter.galley(pill.center() - galley.size() * 0.5, galley, TEXT);
            right = pill.left() - 8.0;
        }
        let left = glyph.right() + 14.0;
        let clip = Rect::from_min_max(pos2(left, rect.top()), pos2(right, rect.bottom()));
        let painter = painter.with_clip_rect(clip.intersect(painter.clip_rect()));
        let y = rect.center().y;
        match subtitle {
            Some(sub) => {
                painter.text(
                    pos2(left, y + 1.0),
                    Align2::LEFT_BOTTOM,
                    title,
                    FontId::proportional(17.0),
                    TEXT_LIST,
                );
                painter.text(
                    pos2(left, y + 3.0),
                    Align2::LEFT_TOP,
                    sub,
                    FontId::proportional(13.0),
                    TEXT_DIM,
                );
            }
            None => {
                painter.text(
                    pos2(left, y),
                    Align2::LEFT_CENTER,
                    title,
                    FontId::proportional(17.0),
                    TEXT_LIST,
                );
            }
        }
        response
    }

    /// Ligne d'arbre (squelette) : guides d'indentation, pastille colorée,
    /// nom ; sélectionnée, pilule bleue et texte blanc.
    pub fn tree_row(&mut self, depth: usize, label: &str, dot: Color32, selected: bool) -> Response {
        let (rect, response) = self.row(40.0, Sense::click(), selected);
        let painter = self.ui.painter();
        if selected {
            painter.rect_filled(rect, CornerRadius::same(10), ACCENT);
        }
        for level in 0..depth {
            let x = rect.left() + INSET + 4.0 + level as f32 * 14.0;
            painter.line_segment(
                [pos2(x, rect.top()), pos2(x, rect.bottom())],
                Stroke::new(1.0, Color32::from_white_alpha(18)),
            );
        }
        let center = pos2(rect.left() + INSET + 4.0 + depth as f32 * 14.0, rect.center().y);
        painter.circle_filled(center, 4.0, dot);
        let (font, color) = if selected {
            (semibold(16.0), Color32::WHITE)
        } else {
            (FontId::proportional(16.0), TEXT_LIST)
        };
        painter.text(center + vec2(12.0, 0.0), Align2::LEFT_CENTER, label, font, color);
        response
    }

    /// Ligne au contenu libre (retrait de 16 pt, centrée verticalement).
    pub fn custom<R>(&mut self, height: f32, add: impl FnOnce(&mut Ui) -> R) -> R {
        let (rect, _) = self.row(height, Sense::hover(), false);
        let inner = rect.shrink2(vec2(INSET, 0.0));
        let mut child = self.ui.new_child(
            egui::UiBuilder::new()
                .max_rect(inner)
                .layout(egui::Layout::left_to_right(egui::Align::Center)),
        );
        add(&mut child)
    }
}
