//! Reconnaissance des gestes, façon Procreate / Nomad Sculpt :
//! - un doigt sur un os : manipulation (selon l'outil) ;
//! - un doigt dans le vide : rotation de la vue ; tape : désélection ;
//! - deux doigts : pincer pour zoomer, glisser pour déplacer la vue ;
//! - tape à deux doigts : annuler ; tape à trois doigts : rétablir.
//!
//! Option « Apple Pencil pour manipuler » (comme Procreate) : les doigts ne
//! font plus que naviguer, seul le crayon agit sur les os.
//!
//! La souris (développement sur ordinateur) suit les mêmes règles :
//! clic gauche = crayon, clic droit / molette = déplacement / zoom.

use std::collections::HashMap;
use std::time::{Duration, Instant};

use bones_core::{Editor, Viewport};
use glam::Vec2;

const TAP_MAX: Duration = Duration::from_millis(280);

/// Action déclenchée par un geste global.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GestureAction {
    Undo,
    Redo,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Mode {
    Idle,
    /// Geste commencé sur l'interface : on ne s'en occupe pas.
    Ui,
    Editor,
    Orbit,
    Pan,
    TwoFinger {
        center: Vec2,
        distance: f32,
    },
    /// Doigts restants après un geste multi-doigts : ignorés.
    Ignore,
}

struct Touch {
    start: Vec2,
    pos: Vec2,
}

/// Contexte fourni à chaque événement.
pub struct InputContext<'a> {
    pub editor: &'a mut Editor,
    pub viewport: Viewport,
    /// Rayon de sélection en pixels physiques.
    pub pick_radius: f32,
    /// Distance en pixels au-delà de laquelle un appui n'est plus une tape.
    pub tap_slop: f32,
    /// Vrai si le point est sur un élément d'interface.
    pub over_ui: &'a dyn Fn(Vec2) -> bool,
    /// Seul l'Apple Pencil manipule les os ; les doigts naviguent.
    pub pencil_only: bool,
}

pub struct Gestures {
    touches: HashMap<u64, Touch>,
    primary: Option<u64>,
    mode: Mode,
    max_touches: usize,
    moved: bool,
    started: Instant,
    mouse: Vec2,
}

impl Default for Gestures {
    fn default() -> Self {
        Self {
            touches: HashMap::new(),
            primary: None,
            mode: Mode::Idle,
            max_touches: 0,
            moved: false,
            started: Instant::now(),
            mouse: Vec2::ZERO,
        }
    }
}

impl Gestures {
    /// Vrai pendant un geste qui pilote la vue ou l'éditeur.
    pub fn is_active(&self) -> bool {
        !matches!(self.mode, Mode::Idle | Mode::Ui)
    }

    // ------------------------------------------------------------------
    // Tactile

    /// Début d'un contact ; `pencil` est vrai pour l'Apple Pencil.
    pub fn touch_start(&mut self, id: u64, pos: Vec2, pencil: bool, cx: InputContext) {
        if self.touches.is_empty() {
            self.max_touches = 0;
            self.moved = false;
            self.started = Instant::now();
            self.mode = Mode::Idle;
        }
        self.touches.insert(id, Touch { start: pos, pos });
        self.max_touches = self.max_touches.max(self.touches.len());

        match (self.touches.len(), self.mode) {
            (_, Mode::Ui) => {}
            (1, _) => {
                self.primary = Some(id);
                self.mode = self.begin_single(pos, pencil, cx);
            }
            (2, _) => {
                if self.mode == Mode::Editor {
                    cx.editor.pointer_cancel();
                }
                let (center, distance) = self.two_finger_state();
                self.mode = Mode::TwoFinger { center, distance };
            }
            _ => self.mode = Mode::Ignore,
        }
    }

    pub fn touch_move(&mut self, id: u64, pos: Vec2, cx: InputContext) {
        let Some(touch) = self.touches.get_mut(&id) else {
            return;
        };
        let delta = pos - touch.pos;
        touch.pos = pos;
        if touch.pos.distance(touch.start) > cx.tap_slop {
            self.moved = true;
        }
        match self.mode {
            Mode::Editor if self.primary == Some(id) => cx.editor.pointer_move(pos, &cx.viewport),
            Mode::Orbit if self.primary == Some(id) => cx.editor.camera.orbit(delta.x, delta.y),
            Mode::TwoFinger { center, distance } => {
                let (new_center, new_distance) = self.two_finger_state();
                let d = new_center - center;
                cx.editor.camera.pan(d.x, d.y, &cx.viewport);
                if distance > 1.0 && new_distance > 1.0 {
                    cx.editor.camera.zoom(new_distance / distance);
                }
                self.mode = Mode::TwoFinger {
                    center: new_center,
                    distance: new_distance,
                };
            }
            _ => {}
        }
    }

    /// Fin (ou annulation) d'un contact. Retourne une action globale éventuelle.
    pub fn touch_end(&mut self, id: u64, cancelled: bool, cx: InputContext) -> Option<GestureAction> {
        self.touches.remove(&id)?;
        if self.mode == Mode::Editor && self.primary == Some(id) {
            if cancelled {
                cx.editor.pointer_cancel();
            } else {
                cx.editor.pointer_up();
            }
            self.mode = Mode::Ignore;
        }
        if matches!(self.mode, Mode::TwoFinger { .. }) {
            self.mode = Mode::Ignore;
        }
        if !self.touches.is_empty() {
            return None;
        }

        // Fin de la séquence : détection des tapes.
        let tap = !cancelled && !self.moved && self.started.elapsed() <= TAP_MAX;
        let action = match (tap, self.max_touches) {
            (true, 2) => Some(GestureAction::Undo),
            (true, 3) => Some(GestureAction::Redo),
            _ => None,
        };
        if tap && self.max_touches == 1 && self.mode == Mode::Orbit {
            cx.editor.tap_empty();
        }
        self.mode = Mode::Idle;
        self.primary = None;
        action
    }

    fn begin_single(&mut self, pos: Vec2, pencil: bool, cx: InputContext) -> Mode {
        let may_edit = pencil || !cx.pencil_only;
        if (cx.over_ui)(pos) {
            Mode::Ui
        } else if may_edit && cx.editor.pointer_down(pos, &cx.viewport, cx.pick_radius) {
            Mode::Editor
        } else {
            Mode::Orbit
        }
    }

    fn two_finger_state(&self) -> (Vec2, f32) {
        let mut points = self.touches.values().map(|t| t.pos);
        match (points.next(), points.next()) {
            (Some(a), Some(b)) => ((a + b) * 0.5, a.distance(b)),
            (Some(a), None) => (a, 0.0),
            _ => (Vec2::ZERO, 0.0),
        }
    }

    // ------------------------------------------------------------------
    // Souris

    pub fn mouse_move(&mut self, pos: Vec2, cx: InputContext) {
        let delta = pos - self.mouse;
        self.mouse = pos;
        if let Some(t) = self.touches.get_mut(&u64::MAX) {
            t.pos = pos;
            if pos.distance(t.start) > cx.tap_slop {
                self.moved = true;
            }
        }
        match self.mode {
            Mode::Editor => cx.editor.pointer_move(pos, &cx.viewport),
            Mode::Orbit => cx.editor.camera.orbit(delta.x, delta.y),
            Mode::Pan => cx.editor.camera.pan(delta.x, delta.y, &cx.viewport),
            _ => {}
        }
    }

    /// Bouton principal (`pan` = faux) ou secondaire (`pan` = vrai) enfoncé.
    pub fn mouse_down(&mut self, pan: bool, cx: InputContext) {
        if self.mode != Mode::Idle {
            return;
        }
        let pos = self.mouse;
        self.moved = false;
        self.started = Instant::now();
        self.touches.insert(u64::MAX, Touch { start: pos, pos });
        self.mode = if (cx.over_ui)(pos) {
            Mode::Ui
        } else if pan {
            Mode::Pan
        } else {
            self.begin_single(pos, true, cx)
        };
    }

    pub fn mouse_up(&mut self, cx: InputContext) {
        self.touches.remove(&u64::MAX);
        match self.mode {
            Mode::Editor => cx.editor.pointer_up(),
            Mode::Orbit if !self.moved => cx.editor.tap_empty(),
            _ => {}
        }
        self.mode = Mode::Idle;
    }

    pub fn mouse_wheel(&mut self, lines: f32, cx: InputContext) {
        if !(cx.over_ui)(self.mouse) {
            cx.editor.camera.zoom(1.1f32.powf(lines));
        }
    }
}
