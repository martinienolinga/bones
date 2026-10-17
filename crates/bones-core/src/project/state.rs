//! Réglages de l'éditeur enregistrés avec le projet : caméra, affichage,
//! mode, outils, sélection, image courante, marqueurs d'auto-rig…
//!
//! Tous les champs ont une valeur par défaut (celle d'un éditeur neuf) : un
//! fichier plus ancien, auquel il manque des réglages, reste lisible.

use serde::{Deserialize, Serialize};

use crate::editor::{EditTool, Editor, Mode, PoseTool, ViewOptions};
use crate::{AutoRigOptions, BindMethod, BoneId, ManipulatorSettings, Markers, Mesh, OrbitCamera, Rig};

/// Longueur maximale de chaîne IK acceptée à la lecture.
const MAX_CHAIN: usize = 64;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct EditorState {
    /// `None` : la caméra cadre le modèle à l'ouverture.
    pub camera: Option<OrbitCamera>,
    pub mode: Mode,
    pub pose_tool: PoseTool,
    pub edit_tool: EditTool,
    pub selected: Option<BoneId>,
    pub chain_len: usize,
    pub bind_method: BindMethod,
    pub view: ViewOptions,
    pub frame: f32,
    pub markers: Option<Markers>,
    pub autorig_options: AutoRigOptions,
    pub mirror_edit: bool,
    pub manipulator: ManipulatorSettings,
}

impl Default for EditorState {
    /// Réglages d'un éditeur neuf : les valeurs par défaut restent ainsi
    /// toujours celles de [`Editor::new`].
    fn default() -> Self {
        let mut state = Self::capture(&Editor::new(Rig::new("", Mesh::default())));
        state.camera = None;
        state
    }
}

impl EditorState {
    pub fn capture(editor: &Editor) -> Self {
        Self {
            camera: Some(editor.camera),
            mode: editor.mode,
            pose_tool: editor.pose_tool,
            edit_tool: editor.edit_tool,
            selected: editor.selected,
            chain_len: editor.chain_len,
            bind_method: editor.bind_method,
            view: editor.view,
            frame: editor.frame,
            markers: editor.markers,
            autorig_options: editor.autorig_options,
            mirror_edit: editor.mirror_edit,
            manipulator: editor.manipulator,
        }
    }

    /// Applique les réglages à un éditeur qui vient d'ouvrir le document.
    pub fn apply(self, editor: &mut Editor) {
        if let Some(camera) = self.camera {
            editor.camera = camera;
        }
        editor.pose_tool = self.pose_tool;
        editor.edit_tool = self.edit_tool;
        editor.selected = self.selected.filter(|b| editor.rig.skeleton.contains(*b));
        editor.chain_len = self.chain_len;
        editor.bind_method = self.bind_method;
        editor.view = self.view;
        editor.frame = self.frame;
        editor.markers = self.markers;
        editor.autorig_options = self.autorig_options;
        editor.mirror_edit = self.mirror_edit;
        editor.manipulator = self.manipulator;
        editor.manipulator.invert_snap = false;
        // En dernier : le changement de mode recale les membres IK (Pose) ou
        // estime les marqueurs s'il n'y en a pas (Auto-rig).
        editor.set_mode(self.mode);
    }

    /// Ramène des réglages lus dans un fichier à des valeurs sûres.
    pub fn sanitize(&mut self, rig: &Rig) {
        let fresh = Self::default();
        if self.camera.is_some_and(|c| !camera_ok(&c)) {
            self.camera = None;
        }
        self.selected = self.selected.filter(|b| rig.skeleton.contains(*b));
        self.chain_len = self.chain_len.clamp(1, MAX_CHAIN);
        let last = rig.clip.frame_count.saturating_sub(1) as f32;
        self.frame = if self.frame.is_finite() {
            self.frame.clamp(0.0, last)
        } else {
            0.0
        };
        if self.markers.is_some_and(|m| !markers_ok(&m)) {
            self.markers = None;
        }
        let o = &mut self.autorig_options;
        o.spine_count = o.spine_count.clamp(1, 6);
        o.neck_count = o.neck_count.clamp(1, 3);
        let m = &self.manipulator;
        let positive = |v: f32| v.is_finite() && v > 0.0;
        if ![m.angle_step_deg, m.distance_step, m.scale_step, m.size]
            .into_iter()
            .all(positive)
        {
            self.manipulator = fresh.manipulator;
        }
    }
}

fn camera_ok(c: &OrbitCamera) -> bool {
    c.target.is_finite()
        && c.distance.is_finite()
        && c.distance > 0.0
        && c.yaw.is_finite()
        && c.pitch.is_finite()
        && c.fov_y.is_finite()
        && c.fov_y > 0.0
        && c.fov_y < std::f32::consts::PI
}

fn markers_ok(m: &Markers) -> bool {
    [m.head_top, m.chin, m.neck, m.shoulder, m.wrist, m.root, m.ankle]
        .iter()
        .all(|p| p.is_finite())
}
