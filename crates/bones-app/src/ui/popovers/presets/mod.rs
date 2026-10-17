//! Galerie de préréglages, dans l'esprit de la Brush Library de Procreate :
//! une colonne de navigation à gauche (catégories), et à droite un champ de
//! recherche puis une grille de cartes à vignette animée (squelette joué en
//! boucle). Toucher une carte ouvre la feuille d'application ; « + » crée un
//! préréglage depuis l'animation, « Import » liste les fichiers importables,
//! un appui long sur un préréglage utilisateur ouvre son menu.
//!
//! La même galerie sert aux animations ([`Kind::Animations`]) et aux
//! expressions faciales ([`Kind::Expressions`]), le filtre et la feuille
//! d'application changeant selon le genre.

mod apply;
mod card;
mod create;
mod grid;
mod library;
mod manage;
mod preview;

use bones_core::Editor;
use bones_ui::icons::Icon;
use bones_ui::{theme, widgets};

pub use library::DIR;

use self::apply::{ApplyForm, Report};
use self::create::CreateForm;
use self::library::{Entry, Importable, Library};
use self::preview::PreviewCache;
use crate::ui::UiState;

/// Genre de galerie ouverte.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Animations,
    Expressions,
}

impl Kind {
    fn title(self) -> &'static str {
        match self {
            Kind::Animations => "Animations",
            Kind::Expressions => "Expressions",
        }
    }
}

/// Vue affichée dans la zone de droite.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum View {
    Grid,
    Apply(Entry),
    Create,
    Import,
}

/// Menu contextuel d'un préréglage utilisateur (appui long).
struct Menu {
    entry: Entry,
    pos: egui::Pos2,
    confirm_delete: bool,
}

#[derive(Default)]
pub struct PresetsState {
    section: usize,
    search: String,
    view_slot: Option<View>,
    pub(crate) library: Library,
    previews: PreviewCache,
    apply: ApplyForm,
    create: CreateForm,
    imports: Option<Vec<Importable>>,
    menu: Option<Menu>,
    /// Renommage en place d'un préréglage utilisateur (entrée + texte saisi).
    rename: Option<(Entry, String)>,
    report: Option<Report>,
    /// Message temporaire à relayer à l'overlay (vidé par l'interface).
    pub pending_toast: Option<String>,
}

impl PresetsState {
    fn view(&self) -> View {
        self.view_slot.unwrap_or(View::Grid)
    }

    /// Prépare la galerie à l'ouverture (bibliothèque chargée, vue remise à la
    /// grille).
    fn open(&mut self, dir: &std::path::Path) {
        self.library.ensure(dir);
        self.view_slot = Some(View::Grid);
        self.menu = None;
        self.report = None;
    }

    /// Referme la galerie (changement de projet) : la bibliothèque sera
    /// relue à la prochaine ouverture.
    pub fn reset(&mut self) {
        self.view_slot = None;
        self.menu = None;
        self.rename = None;
        self.report = None;
    }

    fn notify(&mut self, message: impl Into<String>) {
        self.pending_toast = Some(message.into());
    }

    fn refresh(&mut self, root: &std::path::Path) {
        self.library.refresh(&presets_dir(root));
        self.previews.clear();
        self.imports = None;
    }
}

/// Dossier des préréglages de l'utilisateur dans les Documents.
fn presets_dir(root: &std::path::Path) -> std::path::PathBuf {
    root.join(DIR)
}

/// Affiche la galerie ; retourne vrai pour refermer la fenêtre (bouton
/// « Done »). `docs` est le dossier Documents (la galerie y range ses
/// préréglages dans `Presets` et y lit les fichiers importables).
pub fn show(ui: &mut egui::Ui, editor: &mut Editor, state: &mut UiState, kind: Kind, height: f32) -> bool {
    let docs = state.docs_dir.clone();
    let _ = std::fs::create_dir_all(presets_dir(&docs));
    let p = &mut state.presets;
    if p.view_slot.is_none() {
        p.open(&presets_dir(&docs));
    }
    let mut close = false;
    ui.horizontal_top(|ui| {
        nav(ui, p, kind, height);
        ui.add_space(10.0);
        ui.vertical(|ui| {
            ui.set_height(height);
            close = content(ui, editor, p, &docs, kind, height);
        });
    });
    manage::context_menu(ui, p, &docs);
    close
}

/// Colonne de navigation : titre, puis les catégories en pilules avec le
/// nombre de préréglages.
fn nav(ui: &mut egui::Ui, p: &mut PresetsState, kind: Kind, height: f32) {
    let items = grid::categories(kind, &p.library);
    p.section = p.section.min(items.len().saturating_sub(1));
    egui::Frame::new()
        .fill(theme::NAV)
        .corner_radius(egui::CornerRadius::same(16))
        .inner_margin(egui::Margin::same(12))
        .show(ui, |ui| {
            ui.set_width(196.0);
            ui.set_height(height - 24.0);
            // Le cadre hérite de la disposition horizontale du parent : il faut
            // empiler le titre et les pilules dans une colonne verticale (comme
            // le studio des os), sinon elles se chevauchent en haut.
            ui.vertical(|ui| {
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    ui.add_space(8.0);
                    ui.label(egui::RichText::new(kind.title()).font(theme::semibold(24.0)));
                });
                ui.add_space(14.0);
                ui.spacing_mut().item_spacing.y = 6.0;
                for (i, cat) in items.iter().enumerate() {
                    if widgets::nav_item(ui, cat.icon, &cat.label, p.section == i).clicked() {
                        p.section = i;
                        p.view_slot = Some(View::Grid);
                    }
                }
            });
        });
}

/// Zone de droite : barre de titre (retour et « Done »), puis la vue courante.
fn content(
    ui: &mut egui::Ui,
    editor: &mut Editor,
    p: &mut PresetsState,
    docs: &std::path::Path,
    kind: Kind,
    height: f32,
) -> bool {
    let (title, back) = match p.view() {
        View::Grid => (grid::categories(kind, &p.library)[p.section].label.clone(), false),
        View::Apply(e) => (p.library.info(e).name.clone(), true),
        View::Create => ("New Preset".to_owned(), true),
        View::Import => ("Import".to_owned(), true),
    };
    let mut close = false;
    ui.horizontal(|ui| {
        ui.set_min_height(44.0);
        if back && widgets::cancel_button(ui, "‹  Back").clicked() {
            p.view_slot = Some(View::Grid);
            p.report = None;
        }
        widgets::studio_title(ui, &title);
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            close = widgets::confirm_button(ui, "Done").clicked();
        });
    });
    ui.add_space(8.0);
    let body = height - 60.0;
    match p.view() {
        View::Grid => grid::show(ui, p, kind, body),
        View::Apply(e) => apply::show(ui, editor, p, e, kind, body),
        View::Create => create::show(ui, editor, p, docs, body),
        View::Import => manage::import(ui, p, docs, body),
    }
    close
}

/// Icône d'un préréglage sans aperçu cuit (repli de la carte).
fn fallback_icon(kind: Kind) -> Icon {
    match kind {
        Kind::Animations => Icon::Motion,
        Kind::Expressions => Icon::Face,
    }
}

/// Points d'entrée pour les tests d'interface (pilotage sans toucher aux
/// coordonnées des cartes).
#[cfg(test)]
impl PresetsState {
    pub(crate) fn test_section(&mut self, i: usize) {
        self.section = i;
        self.view_slot = Some(View::Grid);
    }

    /// Ouvre la feuille d'application sur la première entrée de la section
    /// courante ; faux si la section est vide.
    pub(crate) fn test_open_first(&mut self, kind: Kind) -> bool {
        match grid::entries(self, kind).first() {
            Some(&entry) => {
                self.view_slot = Some(View::Apply(entry));
                true
            }
            None => false,
        }
    }

    pub(crate) fn test_apply(&mut self, editor: &mut Editor, kind: Kind) {
        if let View::Apply(entry) = self.view() {
            apply::run(editor, self, entry, kind);
        }
    }

    pub(crate) fn test_create_save(&mut self, editor: &mut Editor, docs: &std::path::Path) {
        create::test_save(editor, self, docs);
    }

    pub(crate) fn test_open_import(&mut self) {
        self.view_slot = Some(View::Import);
    }

    pub(crate) fn test_is_grid(&self) -> bool {
        matches!(self.view(), View::Grid)
    }

    pub(crate) fn test_report_is_some(&self) -> bool {
        self.report.is_some()
    }

    pub(crate) fn test_user_count(&self) -> usize {
        self.library.user_len()
    }
}
