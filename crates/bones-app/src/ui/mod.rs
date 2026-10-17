//! Interface tactile inspirée de Procreate et Nomad Sculpt : la vue 3D occupe
//! tout l'écran, les commandes sont regroupées en blocs posés sur les bords.
//!
//! - en haut à gauche : Galerie, Actions, Vue, Os ([`top`]) ;
//! - en haut à droite : modes Pose / Édition / Auto-Rig ([`top`]) ;
//! - à gauche : curseurs verticaux et annuler/rétablir ([`sidebar`]) ;
//! - à droite : boîte à outils du mode courant ([`toolbox`]) ;
//! - en bas : frise d'animation ([`timeline`]) ou panneau d'auto-rig
//!   ([`autorig_panel`]) ;
//! - fenêtres contextuelles ([`popovers`]) et informations ([`overlay`]).

mod autorig_panel;
mod overlay;
mod popovers;
mod shortcuts;
mod sidebar;
mod timeline;
mod toolbox;
mod top;

use std::path::PathBuf;

use bones_core::io::Format;
use bones_core::{Editor, Mode, Viewport};

pub use popovers::Popover;

use crate::files::{self, FileEntry};

/// Demandes de l'interface qui passent par l'application (fichiers, tâches).
pub enum UiAction {
    NewDemo,
    /// Ouvre un `.bones` ou importe un fichier 3D.
    Import(PathBuf),
    Save,
    Export(Format),
}

/// État des tâches de fond, pour les indicateurs.
#[derive(Clone, Copy)]
pub struct Busy {
    pub binding: bool,
    pub jobs: usize,
}

pub struct UiState {
    /// Zones occupées par l'interface à l'image précédente (en points) :
    /// un geste qui y commence n'est pas transmis à la vue 3D.
    pub rects: Vec<egui::Rect>,
    pub popover: Option<Popover>,
    pub docs_dir: PathBuf,
    /// Rayon de sélection tactile (points).
    pub pick_radius: f32,
    /// Seul l'Apple Pencil manipule les os ; les doigts naviguent.
    pub pencil_only: bool,
    rename: String,
    toast: Option<(String, f64)>,
    /// Documents `.bones` et fichiers importables du dossier Documents.
    files: Option<(Vec<FileEntry>, Vec<FileEntry>)>,
}

impl UiState {
    pub fn new(docs_dir: PathBuf) -> Self {
        Self {
            rects: Vec::new(),
            popover: None,
            docs_dir,
            pick_radius: 22.0,
            pencil_only: false,
            rename: String::new(),
            toast: None,
            files: None,
        }
    }

    /// Affiche un message temporaire.
    pub fn notify(&mut self, message: impl Into<String>) {
        self.toast = Some((message.into(), f64::NAN));
    }

    /// La liste des fichiers sera relue à la prochaine ouverture de la galerie.
    pub fn refresh_files(&mut self) {
        self.files = None;
    }

    fn files(&mut self) -> &(Vec<FileEntry>, Vec<FileEntry>) {
        let dir = &self.docs_dir;
        self.files.get_or_insert_with(|| {
            (
                files::list(dir, |f| f == Format::Bones),
                files::list(dir, |f| f != Format::Bones && f.can_import()),
            )
        })
    }

    fn toggle(&mut self, popover: Popover) {
        self.popover = if self.popover == Some(popover) {
            None
        } else {
            Some(popover)
        };
        if self.popover == Some(Popover::Gallery) {
            self.refresh_files();
        }
    }
}

/// Construit toute l'interface de l'image.
pub fn show(
    ctx: &egui::Context,
    editor: &mut Editor,
    state: &mut UiState,
    viewport: &Viewport,
    busy: Busy,
) -> Vec<UiAction> {
    let mut actions = Vec::new();
    let mut rects = Vec::new();
    if let Some(message) = editor.status.take() {
        state.notify(message);
    }
    shortcuts::handle(ctx, editor, state);

    rects.push(top::left(ctx, editor, state));
    rects.push(top::right(ctx, editor, state));
    rects.extend(sidebar::show(ctx, editor, state));
    rects.push(toolbox::show(ctx, editor, state));
    match editor.mode {
        Mode::Pose => rects.push(timeline::show(ctx, editor)),
        Mode::AutoRig => rects.push(autorig_panel::show(ctx, editor)),
        Mode::Edit => {}
    }
    if let Some(rect) = popovers::show(ctx, editor, state, &mut actions) {
        rects.push(rect);
    }
    overlay::show(ctx, editor, state, viewport, busy);

    state.rects = rects;
    actions
}

/// Bloc flottant ancré sur un bord de l'écran ; retourne sa zone.
fn block(
    ctx: &egui::Context,
    id: &str,
    anchor: egui::Align2,
    offset: [f32; 2],
    frame: egui::Frame,
    add: impl FnOnce(&mut egui::Ui),
) -> egui::Rect {
    egui::Area::new(egui::Id::unique(id))
        .anchor(anchor, offset)
        .order(egui::Order::Foreground)
        .movable(false)
        .show(ctx, |ui| frame.show(ui, add))
        .response
        .rect
}
