//! État de l'éditeur et interactions, indépendants de la plateforme : l'app
//! iPad ne fait que traduire les gestes en appels à cette API, ce qui rend
//! toute la logique testable sur poste de développement.

mod bone_info;
mod bone_ops;
mod drag;
mod edit_tools;
mod history;
mod manip_apply;
mod manipulate;
mod pointer;
mod pose_ops;
mod rig_ops;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_bones;
#[cfg(test)]
mod tests_manip;
mod timeline;
mod view;

pub use bone_info::BoneInfo;
pub use rig_ops::BindJob;

use glam::Vec3;

use crate::{
    AutoRigOptions, BindMethod, BoneId, ManipulatorSettings, MarkerKind, Markers, Mesh, OrbitCamera, Rig,
};

const UNDO_LIMIT: usize = 64;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    /// Pose et animation (Cascadeur).
    Pose,
    /// Édition du squelette de repos.
    Edit,
    /// Marqueurs du rigging assisté (Auto-Rig Pro « Smart »).
    AutoRig,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PoseTool {
    /// Glisser un os : IK (FABRIK), membre IK ou translation de la racine.
    Move,
    /// Rotation autour de l'axe de vue.
    Rotate,
    /// Épingler / désépingler la queue d'un os.
    Pin,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EditTool {
    /// Glisser un os déplace sa queue.
    Select,
    /// Glisser crée un nouvel os.
    AddBone,
    /// Glisser un os déplace sa tête (la queue reste).
    MoveHead,
    /// Glisser un os le déplace en entier.
    MoveBone,
    /// Glisser fait tourner l'os autour de son axe (roulis).
    Roll,
    /// Toucher un os, puis son nouveau parent (ou le vide : racine).
    Reparent,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ViewOptions {
    pub mesh: bool,
    /// Matériaux et textures du modèle (sinon rendu « argile »).
    pub textures: bool,
    pub weights: bool,
    pub bones: bool,
    pub grid: bool,
    pub trajectory: bool,
    pub ghosts: bool,
    pub center_of_mass: bool,
}

impl Default for ViewOptions {
    fn default() -> Self {
        Self {
            mesh: true,
            textures: true,
            weights: false,
            bones: true,
            grid: true,
            trajectory: true,
            ghosts: false,
            center_of_mass: false,
        }
    }
}

/// Poignée manipulable autre qu'un os.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Handle {
    IkTarget(usize),
    IkPole(usize),
    /// Marqueur auto-rig ; `true` pour sa copie symétrique (côté droit).
    Marker(MarkerKind, bool),
}

pub struct Editor {
    pub rig: Rig,
    pub camera: OrbitCamera,
    pub mode: Mode,
    pub pose_tool: PoseTool,
    pub edit_tool: EditTool,
    pub selected: Option<BoneId>,
    /// Longueur de chaîne de l'IK FABRIK et des épingles.
    pub chain_len: usize,
    pub bind_method: BindMethod,
    pub view: ViewOptions,
    pub frame: f32,
    pub playing: bool,
    pub markers: Option<Markers>,
    pub autorig_options: AutoRigOptions,
    /// Dernier message à afficher (toast).
    pub status: Option<String>,
    /// Édition symétrique (« X-Axis Mirror » de Blender) : toute retouche
    /// d'un os `.l` / `.r` est reportée sur son symétrique existant.
    pub mirror_edit: bool,
    /// Manipulateur 3D (translation, rotation, échelle).
    pub manipulator: ManipulatorSettings,
    /// Échelle de l'interface (pixels physiques par point), réglée par
    /// l'application : taille du manipulateur à l'écran.
    pub pixels_per_point: f32,
    /// Outil « Parent » : enfant choisi, en attente de son nouveau parent.
    reparent_child: Option<BoneId>,
    /// Dernière retouche numérique (os, propriété) : les réglages continus
    /// d'une même valeur ne créent qu'une étape d'annulation.
    coalesce: Option<(BoneId, u8)>,
    undo: Vec<Rig>,
    redo: Vec<Rig>,
    /// Pile de rétablissement vidée par le dernier point de reprise, gardée
    /// pour pouvoir l'abandonner (geste sans effet).
    redo_backup: Vec<Rig>,
    /// Le geste en cours a réellement bougé.
    drag_moved: bool,
    drag: Option<drag::Drag>,
    bind_requested: Option<(u64, u64)>,
    masses: Option<(u64, u64, Vec<f32>)>,
    trajectory: Option<(BoneId, u64, u64, Vec<Vec3>)>,
}

impl Editor {
    pub fn new(rig: Rig) -> Self {
        let mut editor = Self {
            rig,
            camera: OrbitCamera::default(),
            mode: Mode::Pose,
            pose_tool: PoseTool::Move,
            edit_tool: EditTool::Select,
            selected: None,
            chain_len: 2,
            bind_method: BindMethod::Heat,
            view: ViewOptions::default(),
            frame: 0.0,
            playing: false,
            markers: None,
            autorig_options: AutoRigOptions::default(),
            status: None,
            mirror_edit: false,
            manipulator: ManipulatorSettings::default(),
            pixels_per_point: 1.0,
            reparent_child: None,
            coalesce: None,
            undo: Vec::new(),
            redo: Vec::new(),
            redo_backup: Vec::new(),
            drag_moved: false,
            drag: None,
            bind_requested: None,
            masses: None,
            trajectory: None,
        };
        editor.frame_all();
        editor
    }

    /// Remplace le document (ouverture d'un fichier).
    pub fn open(&mut self, rig: Rig) {
        self.checkpoint();
        self.rig = rig;
        self.selected = None;
        self.markers = None;
        self.frame = 0.0;
        self.playing = false;
        self.drag = None;
        self.frame_all();
    }

    /// Remplace le maillage (import OBJ) en gardant le squelette.
    pub fn import_mesh(&mut self, mesh: Mesh) {
        self.checkpoint();
        self.rig.set_mesh(mesh);
        self.markers = None;
        self.frame_all();
    }

    pub fn frame_all(&mut self) {
        if let Some((lo, hi)) = self.rig.mesh.bounds() {
            self.camera.frame(lo, hi);
        }
    }
}

fn is_limb_marker(kind: MarkerKind) -> bool {
    matches!(kind, MarkerKind::Shoulder | MarkerKind::Wrist | MarkerKind::Ankle)
}

fn mirror_x(p: Vec3, center: f32) -> Vec3 {
    Vec3::new(2.0 * center - p.x, p.y, p.z)
}
