//! Vue en grille : catégories de la navigation, champ de recherche, grille de
//! cartes à vignette animée, et pour les animations les boutons « + » (créer)
//! et « Import ».

use bones_core::presets::Section;
use bones_ui::icons::{self, Icon};
use bones_ui::{theme, widgets};
use egui::vec2;

use super::library::{Entry, Library};
use super::{Kind, Menu, PresetsState, View, card};

/// Entrée de la colonne de navigation.
pub struct Cat {
    pub label: String,
    pub icon: Icon,
}

/// Catégories affichées pour un genre de galerie.
pub fn categories(kind: Kind, library: &Library) -> Vec<Cat> {
    match kind {
        Kind::Animations => Section::ALL
            .iter()
            .map(|s| Cat {
                label: s.label().to_owned(),
                icon: section_icon(*s),
            })
            .collect(),
        Kind::Expressions => {
            let mut cats = vec![Cat {
                label: "All".to_owned(),
                icon: Icon::Face,
            }];
            for group in expression_groups(library) {
                cats.push(Cat {
                    label: group,
                    icon: Icon::Face,
                });
            }
            cats
        }
    }
}

fn section_icon(section: Section) -> Icon {
    match section {
        Section::Category(c) => match c {
            bones_core::presets::Category::Human => Icon::Pose,
            bones_core::presets::Category::Animals => Icon::Motion,
            bones_core::presets::Category::Vegetation => Icon::Adjust,
            bones_core::presets::Category::Misc => Icon::Chain,
            bones_core::presets::Category::FacialExpression => Icon::Face,
        },
        Section::MyPresets => Icon::Folder,
        Section::Imported => Icon::Import,
    }
}

/// Groupes distincts parmi les expressions faciales (sous-catégories).
fn expression_groups(library: &Library) -> Vec<String> {
    let mut groups: Vec<String> = Vec::new();
    for entry in library.expressions("") {
        let g = library.info(entry).group.trim();
        if !g.is_empty() && !groups.iter().any(|x| x == g) {
            groups.push(g.to_owned());
        }
    }
    groups.sort();
    groups
}

/// Entrées à montrer dans la grille (section courante + recherche).
pub(super) fn entries(p: &PresetsState, kind: Kind) -> Vec<Entry> {
    match kind {
        Kind::Animations => p.library.section(Section::ALL[p.section], &p.search),
        Kind::Expressions => {
            let all = p.library.expressions(&p.search);
            if p.section == 0 {
                return all;
            }
            let groups = expression_groups(&p.library);
            let Some(group) = groups.get(p.section - 1) else {
                return all;
            };
            all.into_iter()
                .filter(|e| &p.library.info(*e).group == group)
                .collect()
        }
    }
}

pub fn show(ui: &mut egui::Ui, p: &mut PresetsState, kind: Kind, body: f32) {
    toolbar(ui, p, kind);
    ui.add_space(8.0);
    let list = entries(p, kind);
    if list.is_empty() {
        empty_state(ui, kind, body);
        return;
    }
    let time = ui.input(|i| i.time) as f32;
    // Les vignettes jouent en continu tant que la galerie est ouverte.
    ui.ctx().request_repaint();
    egui::ScrollArea::vertical()
        .id_salt(("presets-grid", p.section))
        .auto_shrink([false, false])
        .max_height(body - 48.0)
        .show(ui, |ui| {
            let fallback = super::fallback_icon(kind);
            const COLS: usize = 3;
            const GAP: f32 = 12.0;
            let w = ((ui.available_width() - GAP * (COLS as f32 - 1.0)) / COLS as f32).max(120.0);
            let cell = vec2(w, w * 0.78 + 34.0);
            // Cuisson paresseuse des aperçus visibles (budget par image).
            let presets: Vec<_> = list.iter().map(|e| p.library.preset(*e)).collect();
            if p.previews.warm(&presets, 3) {
                ui.ctx().request_repaint();
            }
            for row in list.chunks(COLS) {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = GAP;
                    for &entry in row {
                        card::show(ui, p, entry, cell, time, fallback);
                    }
                });
                ui.add_space(GAP);
            }
        });
}

/// Ligne du haut : recherche à gauche ; pour les animations, « + » et
/// « Import » à droite.
fn toolbar(ui: &mut egui::Ui, p: &mut PresetsState, kind: Kind) {
    ui.horizontal(|ui| {
        search_field(ui, &mut p.search);
        if kind == Kind::Animations {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if widgets::tool_button(ui, Icon::Plus, false, "Create from this animation").clicked() {
                    p.view_slot = Some(View::Create);
                }
                if widgets::tool_button(ui, Icon::Import, false, "Import a motion file").clicked() {
                    p.imports = None;
                    p.view_slot = Some(View::Import);
                }
            });
        }
    });
}

/// Champ de recherche : pastille arrondie avec glyphe de loupe.
fn search_field(ui: &mut egui::Ui, text: &mut String) {
    let width = (ui.available_width() - 110.0).clamp(160.0, 320.0);
    egui::Frame::new()
        .fill(theme::WIDGET)
        .corner_radius(egui::CornerRadius::same(16))
        .inner_margin(egui::Margin::symmetric(12, 6))
        .show(ui, |ui| {
            ui.set_width(width);
            ui.horizontal(|ui| {
                let (glyph, _) = ui.allocate_exact_size(vec2(18.0, 20.0), egui::Sense::hover());
                icons::paint(ui.painter(), glyph, Icon::Eye, theme::ICON_DIM);
                let edit = egui::TextEdit::singleline(text)
                    .hint_text("Search")
                    .frame(egui::Frame::NONE)
                    .desired_width(width - 30.0);
                ui.add(edit);
            });
        });
}

fn empty_state(ui: &mut egui::Ui, kind: Kind, body: f32) {
    let (title, hint) = match kind {
        Kind::Animations => ("No presets here", "Create one with + or import a motion file."),
        Kind::Expressions => (
            "No facial expressions yet",
            "Expressions appear here once the rig has a face.",
        ),
    };
    ui.add_space((body * 0.3).max(40.0));
    ui.vertical_centered(|ui| {
        let (glyph, _) = ui.allocate_exact_size(vec2(44.0, 44.0), egui::Sense::hover());
        icons::paint(ui.painter(), glyph, super::fallback_icon(kind), theme::TEXT_FAINT);
        ui.add_space(10.0);
        ui.label(egui::RichText::new(title).font(theme::semibold(20.0)));
        ui.add_space(4.0);
        ui.label(egui::RichText::new(hint).size(14.0).color(theme::TEXT_DIM));
    });
}

/// Ouvre le menu contextuel d'un préréglage utilisateur (appui long).
pub(super) fn open_menu(p: &mut PresetsState, entry: Entry, pos: egui::Pos2) {
    p.menu = Some(Menu {
        entry,
        pos,
        confirm_delete: false,
    });
}
