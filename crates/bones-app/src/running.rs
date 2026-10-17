//! État de l'application une fois la fenêtre créée : éditeur, GPU,
//! interface, gestes, tâches de fond, galerie et projets ([`Session`]).

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use bones_core::{Editor, Mesh, Rig, Viewport};
use glam::Vec2;
use winit::event::WindowEvent;
use winit::event_loop::EventLoopProxy;
use winit::window::Window;

use crate::UserEvent;
use crate::files;
use crate::gpu::Gpu;
use crate::input::{GestureAction, Gestures, InputContext};
use crate::jobs::{JobResult, Jobs};
use crate::scene::SceneBuilder;
use crate::session::{Screen, Session};
use crate::settings::Settings;
use crate::ui::{UiAction, UiState};

pub struct Running {
    pub window: Arc<Window>,
    pub gpu: Gpu,
    pub(crate) renderer: bones_render::Renderer,
    pub(crate) egui_ctx: egui::Context,
    pub(crate) egui_state: egui_winit::State,
    pub(crate) egui_renderer: egui_wgpu::Renderer,
    /// Flou des panneaux en verre dépoli.
    pub(crate) glass: crate::glass::Glass,
    pub(crate) editor: Editor,
    pub(crate) ui: UiState,
    pub(crate) gestures: Gestures,
    pub(crate) scene: SceneBuilder,
    pub(crate) jobs: Jobs,
    pub(crate) session: Session,
    pub(crate) last_frame: Instant,
    pub(crate) repaint_at: Option<Instant>,
}

impl Running {
    pub fn new(window: Arc<Window>, proxy: EventLoopProxy<UserEvent>) -> Self {
        let gpu = Gpu::new(window.clone());
        let renderer = bones_render::Renderer::new(&gpu.device, gpu.config.format);
        let egui_ctx = egui::Context::default();
        bones_ui::theme::apply(&egui_ctx);
        let egui_state = egui_winit::State::new(
            egui_ctx.clone(),
            egui::ViewportId::ROOT,
            &*window,
            Some(window.scale_factor() as f32),
            None,
            Some(gpu.device.limits().max_texture_dimension_2d as usize),
        );
        let egui_renderer = egui_wgpu::Renderer::new(
            &gpu.device,
            gpu.config.format,
            egui_wgpu::RendererOptions::default(),
        );
        let copyable = gpu.config.usage.contains(wgpu::TextureUsages::COPY_SRC);
        let glass = crate::glass::Glass::new(&gpu.device, gpu.config.format, copyable);

        let docs = files::documents_dir();
        let session = Session::new(&docs, proxy.clone());

        Self {
            window,
            gpu,
            renderer,
            egui_ctx,
            egui_state,
            egui_renderer,
            glass,
            // Éditeur vide en attendant l'ouverture d'un projet : l'app
            // démarre toujours sur la galerie.
            editor: Editor::new(Rig::new("", Mesh::default())),
            ui: UiState::new(docs.clone(), Settings::load(&docs)),
            gestures: Gestures::default(),
            scene: SceneBuilder::default(),
            jobs: Jobs::new(proxy),
            session,
            last_frame: Instant::now(),
            repaint_at: None,
        }
    }

    /// Transmet l'événement à egui ; vrai s'il l'a consommé.
    pub fn egui_event(&mut self, event: &WindowEvent) -> bool {
        self.egui_state.on_window_event(&self.window, event).consumed
    }

    pub fn viewport(&self) -> Viewport {
        let [w, h] = self.gpu.size();
        Viewport::new(w as f32, h as f32)
    }

    /// Lecture en cours, geste actif ou tâches : on redessine en continu.
    pub fn wants_continuous_redraw(&self) -> bool {
        let editing = self.session.screen == Screen::Editor;
        editing && (self.editor.playing || self.gestures.is_active()) || self.jobs.running() > 0
    }

    /// Prochaine image demandée (egui, enregistrement automatique).
    pub fn next_repaint(&self) -> Option<Instant> {
        self.repaint_at.into_iter().chain(self.session.deadline()).min()
    }

    /// Les gestes ne pilotent la vue 3D que dans l'éditeur.
    fn editing(&self) -> bool {
        self.session.screen == Screen::Editor
    }

    // ------------------------------------------------------------------
    // Gestes

    fn with_input<R: Default>(&mut self, f: impl FnOnce(&mut Gestures, InputContext) -> R) -> R {
        if !self.editing() {
            return R::default();
        }
        let ppp = self.egui_ctx.pixels_per_point();
        let pencil_required = self.ui.settings.pencil.requires_pencil(&self.editor);
        let rects = &self.ui.rects;
        let over_ui = |p: Vec2| rects.iter().any(|r| r.contains(egui::pos2(p.x / ppp, p.y / ppp)));
        let [w, h] = self.gpu.size();
        let cx = InputContext {
            editor: &mut self.editor,
            viewport: Viewport::new(w as f32, h as f32),
            pick_radius: self.ui.settings.pick_radius * ppp,
            tap_slop: 12.0 * ppp,
            over_ui: &over_ui,
            pencil_required,
            pixels_per_point: ppp,
        };
        f(&mut self.gestures, cx)
    }

    pub fn touch_start(&mut self, id: u64, pos: Vec2, pencil: bool) {
        self.with_input(|g, cx| g.touch_start(id, pos, pencil, cx));
    }

    pub fn touch_move(&mut self, id: u64, pos: Vec2) {
        self.with_input(|g, cx| g.touch_move(id, pos, cx));
    }

    pub fn touch_end(&mut self, id: u64, cancelled: bool) {
        match self.with_input(|g, cx| g.touch_end(id, cancelled, cx)) {
            Some(GestureAction::Undo) if self.editor.can_undo() => {
                self.editor.undo();
                self.ui.notify("Undo");
            }
            Some(GestureAction::Redo) if self.editor.can_redo() => {
                self.editor.redo();
                self.ui.notify("Redo");
            }
            _ => {}
        }
    }

    pub fn mouse_move(&mut self, pos: Vec2) {
        self.with_input(|g, cx| g.mouse_move(pos, cx));
    }

    pub fn mouse_down(&mut self, pan: bool) {
        self.with_input(|g, cx| g.mouse_down(pan, cx));
    }

    pub fn mouse_up(&mut self) {
        self.with_input(|g, cx| g.mouse_up(cx));
    }

    pub fn mouse_wheel(&mut self, lines: f32) {
        self.with_input(|g, cx| g.mouse_wheel(lines, cx));
    }

    // ------------------------------------------------------------------
    // Fichiers et tâches

    pub fn apply(&mut self, action: UiAction) {
        let docs = self.ui.docs_dir.clone();
        match action {
            UiAction::Gallery => self.leave_editor(),
            UiAction::Export(format) => {
                let dir = docs.join(files::EXPORTS);
                if let Err(e) = std::fs::create_dir_all(&dir) {
                    self.ui.notify(format!("Error: {e}"));
                    return;
                }
                let path = files::free_path(&dir, &self.editor.rig.name, format.extension());
                self.jobs.export(self.editor.rig.clone(), format, path);
            }
        }
    }

    pub fn poll_jobs(&mut self) {
        for result in self.jobs.poll() {
            match result {
                JobResult::Bound(skin, key) => self.editor.finish_bind(skin, key),
                JobResult::Exported(Ok(files)) => {
                    let main = files.first().map(|p| file_name(p)).unwrap_or_default();
                    let extra = files.len().saturating_sub(1);
                    self.ui.notify(match extra {
                        0 => format!("Exported to {}: {main}", files::EXPORTS),
                        n => format!("Exported to {}: {main} (+{n} files)", files::EXPORTS),
                    });
                }
                JobResult::Exported(Err(e)) => self.ui.notify(format!("Error: {e}")),
            }
        }
    }

    /// Enregistre les réglages s'ils ont changé.
    pub fn persist_settings(&mut self) {
        if self.ui.settings_dirty {
            self.ui.settings.save(&self.ui.docs_dir);
            self.ui.settings_dirty = false;
        }
    }
}

fn file_name(path: &std::path::Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| PathBuf::from(path).display().to_string())
}
