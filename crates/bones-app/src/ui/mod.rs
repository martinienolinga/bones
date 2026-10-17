//! Interface tactile calquée sur Procreate : la vue 3D occupe tout l'écran,
//! une barre pleine court le long du bord supérieur et deux bandeaux sont
//! collés aux bords latéraux.
//!
//! - barre du haut : Galerie, menus en puits ronds, outils du mode et choix
//!   du mode ([`top`]) ;
//! - bandeau gauche : curseurs verticaux, cadrage, annuler/rétablir
//!   ([`sidebar`]) ;
//! - bandeau droit : actions et bascules du mode ([`toolbox`]) ;
//! - en bas : frise d'animation ([`timeline`]) ou panneau d'auto-rig
//!   ([`autorig_panel`]) ;
//! - fenêtres contextuelles et studios ([`popovers`]), informations
//!   ([`overlay`]).

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
use bones_core::{EditTool, Editor, Mode, Viewport};

pub use popovers::Popover;

use crate::files::{self, FileEntry};
use crate::settings::Settings;

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
    /// Réglages persistants (Apple Pencil, rayon de sélection).
    pub settings: Settings,
    /// Vrai quand un réglage a changé et doit être enregistré.
    pub settings_dirty: bool,
    /// Abscisse du bouton qui a ouvert la fenêtre (pointe de l'encoche).
    popover_anchor: f32,
    /// Réglages à l'ouverture du studio, rétablis par « Cancel ».
    settings_backup: Settings,
    /// Onglet des actions, section des studios (réglages, os).
    actions_tab: usize,
    settings_section: usize,
    bones_section: usize,
    /// Nombre de morceaux pour « Subdivide ».
    subdivide_parts: usize,
    /// Consigne de l'outil « Parent » (pastille en bas de l'écran).
    hint: Option<String>,
    rename: String,
    toast: Option<(String, f64)>,
    /// Documents `.bones` et fichiers importables du dossier Documents.
    files: Option<(Vec<FileEntry>, Vec<FileEntry>)>,
}

impl UiState {
    pub fn new(docs_dir: PathBuf, settings: Settings) -> Self {
        Self {
            rects: Vec::new(),
            popover: None,
            docs_dir,
            settings,
            settings_dirty: false,
            popover_anchor: 0.0,
            settings_backup: settings,
            actions_tab: 0,
            settings_section: 0,
            bones_section: 0,
            subdivide_parts: 2,
            hint: None,
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

    /// Ouvre ou ferme une fenêtre ; `anchor_x` est le centre du bouton.
    fn toggle(&mut self, popover: Popover, anchor_x: f32) {
        self.popover = if self.popover == Some(popover) {
            None
        } else {
            Some(popover)
        };
        self.popover_anchor = anchor_x;
        match self.popover {
            Some(Popover::Gallery) => self.refresh_files(),
            Some(Popover::Settings) => self.settings_backup = self.settings,
            _ => {}
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
    route_status(editor, state);
    shortcuts::handle(ctx, editor, state);

    rects.push(top::show(ctx, editor, state));
    rects.push(sidebar::show(ctx, editor, state));
    rects.push(toolbox::show(ctx, editor, state));
    rects.extend(crate::gizmo::show(ctx, editor, state.settings.gizmo));
    match editor.mode {
        Mode::Pose => rects.push(timeline::show(ctx, editor)),
        Mode::AutoRig => rects.push(autorig_panel::show(ctx, editor)),
        Mode::Edit => {}
    }
    if let Some(rect) = popovers::show(ctx, editor, state, &mut actions) {
        rects.push(rect);
    }
    rects.extend(overlay::show(ctx, editor, state, viewport, busy));

    state.rects = rects;
    actions
}

/// Messages de l'éditeur : ceux de l'outil « Parent » restent affichés en
/// consigne tant que l'outil est actif ; les autres sont temporaires.
fn route_status(editor: &mut Editor, state: &mut UiState) {
    let reparenting = editor.mode == Mode::Edit && editor.edit_tool == EditTool::Reparent;
    if let Some(message) = editor.status.take() {
        if reparenting && is_reparent_message(&message) {
            state.hint = Some(message);
        } else {
            state.notify(message);
        }
    }
    if !reparenting {
        state.hint = None;
    }
}

/// Consigne ou résultat de l'outil « Parent » (textes de `bones-core`).
fn is_reparent_message(message: &str) -> bool {
    message.contains("tap the new parent") || message.contains(" → ") || message.ends_with(" is now a root")
}

/// Marge entre les blocs flottants et les bords de l'écran.
const EDGE: f32 = 10.0;
/// Hauteur de la barre du haut.
const BAR_H: f32 = 48.0;

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

#[cfg(test)]
mod tests;
