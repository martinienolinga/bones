//! Galerie des projets, écran d'accueil de l'app, calquée sur celle de
//! Procreate : grand titre, capsule « Select · Import » et bouton « + » en
//! haut, puis la grille des projets (vignette, nom, résumé), du plus récent
//! au plus ancien.
//!
//! - toucher une carte ouvre le projet, toucher son nom le renomme ;
//! - appui long (ou clic droit) : menu Rename / Duplicate / Delete ;
//! - « Select » : cartes à cocher, puis Duplicate / Delete / Done ;
//! - « + » : mannequin, projet vide, import d'un modèle 3D ;
//! - « Import » : modèles 3D et projets `.kbrig` du dossier Documents.
//!
//! Modules : [`layout`] (grille), `top` (barre), `grid`, `card` et
//! `caption` (cartes), `menus` et `popover` (fenêtres), `dialog`
//! (confirmation), [`library`] (liste), [`thumbs`] (vignettes), [`zoom`]
//! (transitions vers l'éditeur).
//! La galerie ne touche jamais au disque : elle émet des [`GalleryAction`].

mod caption;
mod card;
mod dialog;
mod grid;
pub mod layout;
pub mod library;
mod menus;
mod popover;
mod resize;
pub mod thumbs;
mod top;
pub mod zoom;

#[cfg(test)]
mod tests;

use std::collections::HashMap;
use std::path::PathBuf;

use egui::{Color32, Rect};

pub use library::Library;
pub use thumbs::Thumbs;

use crate::files::ImportFile;
use crate::projects::{Template, Waker};

/// Fond de la galerie.
pub const BACKGROUND: Color32 = Color32::from_rgb(0x12, 0x12, 0x14);

/// Demandes de la galerie, exécutées par l'application.
#[derive(Clone, Debug, PartialEq)]
pub enum GalleryAction {
    Open(PathBuf),
    New(Template),
    Import(ImportFile),
    /// Relire la liste des fichiers importables (fenêtre « Import »).
    ListImports,
    Rename(PathBuf, String),
    Duplicate(Vec<PathBuf>),
    Delete(Vec<PathBuf>),
}

/// Fenêtre contextuelle ouverte.
#[derive(Clone, Debug, PartialEq)]
enum Menu {
    /// Menu « + », ancré sous le bouton.
    New(Rect),
    /// Fichiers importables, ancrés sous « Import ».
    Import(Rect),
    /// Menu d'une carte (appui long), ancré sur sa vignette.
    Card(PathBuf, Rect),
}

/// Renommage en place d'une carte.
struct Rename {
    path: PathBuf,
    text: String,
    /// Le champ doit prendre le focus (première image).
    focus: bool,
}

pub struct Gallery {
    pub library: Library,
    pub thumbs: Thumbs,
    /// Mode sélection : projets cochés.
    selection: Option<Vec<PathBuf>>,
    menu: Option<Menu>,
    /// Projets dont la suppression attend confirmation.
    confirm: Option<Vec<PathBuf>>,
    rename: Option<Rename>,
    /// Projet en cours d'ouverture (indicateur sur sa carte).
    pub opening: Option<PathBuf>,
    /// Fichiers importables ; `None` pendant leur lecture.
    pub imports: Option<Vec<ImportFile>>,
    /// Carte dont l'appui long a ouvert le menu : son relâcher ne l'ouvre pas.
    long_pressed: Option<PathBuf>,
    /// Vignettes affichées à la dernière image (départ du zoom d'ouverture).
    shown: HashMap<PathBuf, Rect>,
    /// Carte dont la vignette est en transition (dessinée par le zoom).
    pub zooming: Option<PathBuf>,
    /// Revenir en haut de la grille à la prochaine image.
    pub scroll_to_top: bool,
    /// Création ou import en cours.
    busy: bool,
    /// Zones des boutons de la barre à la dernière image (tests).
    #[cfg(test)]
    buttons: Vec<Rect>,
}

impl Gallery {
    pub fn new(wake: Waker) -> Self {
        Self {
            library: Library::default(),
            thumbs: Thumbs::new(wake),
            selection: None,
            menu: None,
            confirm: None,
            rename: None,
            opening: None,
            imports: None,
            long_pressed: None,
            shown: HashMap::new(),
            zooming: None,
            scroll_to_top: false,
            busy: false,
            #[cfg(test)]
            buttons: Vec::new(),
        }
    }

    /// Zone de la vignette d'un projet à la dernière image.
    pub fn thumb_rect(&self, path: &std::path::Path) -> Option<Rect> {
        self.shown.get(path).copied()
    }

    /// Ferme menus, dialogue, renommage et mode sélection (retour de
    /// l'éditeur).
    pub fn reset(&mut self) {
        self.selection = None;
        self.menu = None;
        self.confirm = None;
        self.rename = None;
        self.long_pressed = None;
    }

    /// Les cartes réagissent (aucune ouverture, création ni transition en
    /// cours).
    fn is_idle(&self) -> bool {
        self.opening.is_none() && self.zooming.is_none() && !self.busy
    }

    fn is_selecting(&self) -> bool {
        self.selection.is_some()
    }

    fn is_selected(&self, path: &std::path::Path) -> bool {
        self.selection
            .as_ref()
            .is_some_and(|s| s.iter().any(|p| p == path))
    }

    fn toggle_selected(&mut self, path: &std::path::Path) {
        if let Some(selection) = &mut self.selection {
            match selection.iter().position(|p| p == path) {
                Some(i) => {
                    selection.remove(i);
                }
                None => selection.push(path.to_owned()),
            }
        }
    }

    fn start_rename(&mut self, path: PathBuf) {
        let text = self
            .library
            .get(&path)
            .map(|c| c.name.clone())
            .unwrap_or_default();
        self.rename = Some(Rename {
            path,
            text,
            focus: true,
        });
    }
}

/// Construit la galerie (plein écran) ; retourne les demandes de l'image.
/// `busy` : création ou import en cours, affiché en pastille.
pub fn show(ctx: &egui::Context, gallery: &mut Gallery, busy: Option<&str>) -> Vec<GalleryAction> {
    let mut actions = Vec::new();
    gallery.busy = busy.is_some();
    for preview in gallery.thumbs.poll(ctx) {
        if let Some(info) = preview.info {
            gallery.library.set_info(&preview.path, preview.modified, info);
        }
    }
    let screen = ctx.content_rect();
    egui::Area::new(egui::Id::unique("gallery"))
        .fixed_pos(screen.min)
        .order(egui::Order::Background)
        .show(ctx, |ui| {
            ui.set_min_size(screen.size());
            ui.painter().rect_filled(screen, 0, BACKGROUND);
            grid::show(ui, gallery, &mut actions);
            top::show(ui, gallery, &mut actions);
        });
    menus::show(ctx, gallery, &mut actions);
    dialog::show(ctx, gallery, &mut actions);
    if let Some(text) = busy {
        busy_pill(ctx, text);
    }
    if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
        if gallery.menu.is_some() || gallery.confirm.is_some() || gallery.rename.is_some() {
            gallery.menu = None;
            gallery.confirm = None;
            gallery.rename = None;
        } else {
            gallery.selection = None;
        }
    }
    actions
}

/// Pastille « Importing “Fox”… » en bas de l'écran, avec indicateur.
fn busy_pill(ctx: &egui::Context, text: &str) {
    egui::Area::new(egui::Id::unique("gallery-busy"))
        .anchor(egui::Align2::CENTER_BOTTOM, [0.0, -28.0])
        .order(egui::Order::Foreground)
        .interactable(false)
        .show(ctx, |ui| {
            bones_ui::theme::panel_frame()
                .corner_radius(22)
                .inner_margin(egui::Margin::symmetric(18, 12))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.add(egui::Spinner::new().size(18.0).color(bones_ui::theme::ACCENT));
                        ui.label(egui::RichText::new(text).font(bones_ui::theme::semibold(15.0)));
                    });
                });
        });
}
