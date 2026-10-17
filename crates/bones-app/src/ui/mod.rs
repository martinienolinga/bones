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
//!   ([`autorig_panel`]), et la barre du manipulateur 3D
//!   ([`transform_bar`]) ;
//! - fenêtres contextuelles et studios ([`popovers`]), informations
//!   ([`overlay`]).

mod autorig_panel;
mod face_markers;
mod face_view;
mod overlay;
mod popovers;
mod shortcuts;
mod sidebar;
mod timeline;
mod toolbox;
mod top;
mod transform_bar;

use std::path::PathBuf;

use bones_core::io::Format;
use bones_core::{EditTool, Editor, Mode, Viewport};

pub use popovers::{Popover, PresetsState};

use crate::settings::Settings;

/// Demandes de l'interface qui passent par l'application (fichiers, tâches).
pub enum UiAction {
    /// Retour à la galerie des projets (le projet est enregistré).
    Gallery,
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
    /// Les réglages de pinceau ont été chargés dans l'éditeur courant. Faux
    /// à l'ouverture d'un projet (éditeur neuf) : rechargés une seule fois.
    weight_paint_loaded: bool,
    /// Abscisse du bouton qui a ouvert la fenêtre (pointe de l'encoche).
    popover_anchor: f32,
    /// Réglages à l'ouverture du studio, rétablis par « Cancel ».
    settings_backup: Settings,
    /// Onglet des actions, section des studios (réglages, os, pinceau).
    actions_tab: usize,
    settings_section: usize,
    bones_section: usize,
    brush_section: usize,
    /// Réglages et confirmation des outils du menu « Weights ».
    weights_tools: popovers::WeightsToolsState,
    /// Nombre de morceaux pour « Subdivide ».
    subdivide_parts: usize,
    /// Consigne de l'outil « Parent » (pastille en bas de l'écran).
    hint: Option<String>,
    /// Barre du manipulateur 3D (fenêtre d'options).
    transform: transform_bar::BarState,
    rename: String,
    toast: Option<(String, f64)>,
    /// Bouton « Gallery » touché à cette image.
    gallery_requested: bool,
    /// État des galeries de préréglages (animations et expressions).
    presets: PresetsState,
    /// Étape courante de l'auto-rig (corps ou visage).
    autorig_step: autorig_panel::Step,
    /// Gros plan automatique sur la tête à l'étape « Face ».
    face_view: face_view::FaceView,
}

impl UiState {
    pub fn new(docs_dir: PathBuf, settings: Settings) -> Self {
        Self {
            rects: Vec::new(),
            popover: None,
            docs_dir,
            settings_backup: settings.clone(),
            settings,
            settings_dirty: false,
            weight_paint_loaded: false,
            popover_anchor: 0.0,
            actions_tab: 0,
            settings_section: 0,
            bones_section: 0,
            brush_section: 0,
            weights_tools: popovers::WeightsToolsState::default(),
            subdivide_parts: 2,
            hint: None,
            transform: transform_bar::BarState::default(),
            rename: String::new(),
            toast: None,
            gallery_requested: false,
            presets: PresetsState::default(),
            autorig_step: autorig_panel::Step::Body,
            face_view: face_view::FaceView::default(),
        }
    }

    /// Affiche un message temporaire.
    pub fn notify(&mut self, message: impl Into<String>) {
        self.toast = Some((message.into(), f64::NAN));
    }

    /// Projet ouvert ou refermé : fenêtres fermées, consignes effacées.
    pub fn reset_for_project(&mut self) {
        self.popover = None;
        self.hint = None;
        self.weights_tools.confirm_automatic = false;
        self.weight_paint_loaded = false;
        self.gallery_requested = false;
        self.presets.reset();
        self.autorig_step = autorig_panel::Step::Body;
        self.face_view.reset();
    }

    /// Ouvre ou ferme une fenêtre ; `anchor_x` est le centre du bouton.
    fn toggle(&mut self, popover: Popover, anchor_x: f32) {
        self.popover = if self.popover == Some(popover) {
            None
        } else {
            Some(popover)
        };
        self.popover_anchor = anchor_x;
        if self.popover == Some(Popover::Settings) {
            self.settings_backup = self.settings.clone();
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
    apply_manipulator(editor, state);
    load_weight_paint(editor, state);
    shortcuts::handle(ctx, editor, state);

    rects.push(top::show(ctx, editor, state));
    if std::mem::take(&mut state.gallery_requested) {
        state.face_view.settle(editor);
        actions.push(UiAction::Gallery);
    }
    close_mode_popovers(editor, state);
    rects.push(sidebar::show(ctx, editor, state));
    rects.push(toolbox::show(ctx, editor, state));
    rects.extend(crate::gizmo::show(ctx, editor, state.settings.gizmo));
    let mut bottom = ctx.content_rect().bottom() - EDGE;
    let mut autorig_top = None;
    match editor.mode {
        Mode::Pose => {
            let timeline = timeline::show(ctx, editor);
            bottom = timeline.top();
            rects.push(timeline);
        }
        Mode::AutoRig => {
            let panel = autorig_panel::show(ctx, editor, state);
            autorig_top = Some(panel.top());
            rects.push(panel);
        }
        Mode::Edit | Mode::WeightPaint => {}
    }
    // Gros plan sur la tête (avant les marqueurs, qui suivent la caméra).
    let face = editor.mode == Mode::AutoRig && state.autorig_step == autorig_panel::Step::Face;
    state.face_view.update(ctx, editor, face, viewport, autorig_top);
    if face && autorig_top.is_some() {
        rects.extend(face_markers::show(ctx, editor, viewport));
    }
    if transform_bar::available(editor) {
        rects.extend(transform_bar::show(ctx, editor, state, bottom));
    } else {
        state.transform.options = false;
    }
    if let Some(rect) = popovers::show(ctx, editor, state, &mut actions) {
        rects.push(rect);
    }
    if let Some(rect) = popovers::confirm_automatic(ctx, editor, state) {
        rects.push(rect);
    }
    rects.extend(overlay::show(ctx, editor, state, viewport, busy));

    store_manipulator(editor, state);
    store_weight_paint(editor, state);
    state.rects = rects;
    actions
}

/// Les réglages du manipulateur vivent dans les réglages de l'application
/// (enregistrés) : ils sont appliqués à l'éditeur au début de chaque image…
fn apply_manipulator(editor: &mut Editor, state: &UiState) {
    let invert = editor.manipulator.invert_snap;
    editor.manipulator = state.settings.manipulator;
    editor.manipulator.invert_snap = invert;
}

/// … et les changements faits pendant l'image (barre, raccourcis) y sont
/// recopiés.
fn store_manipulator(editor: &Editor, state: &mut UiState) {
    let mut current = editor.manipulator;
    current.invert_snap = false;
    if current != state.settings.manipulator {
        state.settings.manipulator = current;
        state.settings_dirty = true;
    }
}

/// Charge les réglages de pinceau persistants dans l'éditeur, une seule fois
/// par projet (l'éditeur est ensuite la source de vérité, car le moteur y
/// écrit aussi — la pipette change le poids du pinceau).
fn load_weight_paint(editor: &mut Editor, state: &mut UiState) {
    if !state.weight_paint_loaded {
        editor.weight_paint = state.settings.weight_paint.clone();
        state.weight_paint_loaded = true;
    }
}

/// Les retouches faites à l'image (pinceau, bascules, picker, pipette) sont
/// recopiées dans les réglages pour être enregistrées.
fn store_weight_paint(editor: &Editor, state: &mut UiState) {
    if editor.weight_paint != state.settings.weight_paint {
        state.settings.weight_paint = editor.weight_paint.clone();
        state.settings_dirty = true;
    }
}

/// Ferme les fenêtres propres à un mode quand on change de mode (après la
/// barre du haut, qui applique le changement) : la bibliothèque de préréglages
/// et les expressions n'existent qu'en pose, les outils de poids qu'en
/// peinture.
fn close_mode_popovers(editor: &Editor, state: &mut UiState) {
    if let Some(popover) = state.popover {
        let keep = match popover {
            Popover::Presets | Popover::Expressions => editor.mode == Mode::Pose,
            Popover::Brush | Popover::WeightsTools => editor.mode == Mode::WeightPaint,
            _ => true,
        };
        if !keep {
            state.popover = None;
        }
    }
}

/// Message temporaire seul (galerie).
pub fn show_toast(ctx: &egui::Context, state: &mut UiState) {
    overlay::toast(ctx, state);
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
