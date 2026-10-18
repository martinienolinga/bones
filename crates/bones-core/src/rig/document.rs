//! Document de travail : maillage, squelette, poids, pose, animation,
//! membres IK/FK et points épinglés.
//!
//! Les données lourdes (maillage, poids, animation) sont partagées par `Arc` :
//! cloner un `Rig` (historique d'annulation) ne copie que ce qui est léger,
//! et une donnée lourde n'est dupliquée que lorsqu'elle est modifiée.

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use glam::{Mat4, Vec3};
use serde::{Deserialize, Serialize};

use crate::scene::{BodyProps, SceneObject};
use crate::{BindMethod, BoneId, Clip, Limb, Mesh, Pin, Pose, Skeleton, Skin, Transform};

static NEXT_REVISION: AtomicU64 = AtomicU64::new(1);

/// Identifiant unique de version d'une donnée (jamais réutilisé).
pub fn next_revision() -> u64 {
    NEXT_REVISION.fetch_add(1, Ordering::Relaxed)
}

/// Numéros de version des données, pour les caches (GPU, trajectoires) et
/// la liaison en tâche de fond. Non sérialisés.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Revisions {
    pub skeleton: u64,
    pub mesh: u64,
    pub skin: u64,
    pub clip: u64,
    /// (squelette, maillage) pour lesquels les poids ont été calculés.
    pub bound_to: (u64, u64),
}

impl Default for Revisions {
    fn default() -> Self {
        let (skeleton, mesh) = (next_revision(), next_revision());
        Self {
            skeleton,
            mesh,
            skin: next_revision(),
            clip: next_revision(),
            bound_to: (skeleton, mesh),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Rig {
    pub name: String,
    pub skeleton: Skeleton,
    pub mesh: Arc<Mesh>,
    pub skin: Arc<Skin>,
    pub pose: Pose,
    pub clip: Arc<Clip>,
    #[serde(default)]
    pub limbs: Vec<Limb>,
    #[serde(default)]
    pub pins: Vec<Pin>,
    /// Les poids ont été peints (ou retouchés par un outil « Weights ») : ils
    /// ne sont plus recalculés automatiquement quand le squelette change,
    /// jusqu'à ce que l'utilisateur relance les poids automatiques.
    #[serde(default)]
    pub(crate) painted: bool,
    /// Nom, visibilité et verrou du corps dans la liste des objets.
    #[serde(default)]
    pub body: BodyProps,
    /// Objets supplémentaires (voir [`crate::scene`]), dans l'ordre de la liste.
    #[serde(default)]
    pub(crate) objects: Vec<SceneObject>,
    #[serde(skip)]
    pub(crate) revisions: Revisions,
}

impl Rig {
    pub fn new(name: impl Into<String>, mesh: Mesh) -> Self {
        let skin = Skin {
            weights: vec![Default::default(); mesh.vertex_count()],
        };
        Self {
            name: name.into(),
            skeleton: Skeleton::new(),
            mesh: Arc::new(mesh),
            skin: Arc::new(skin),
            pose: Pose::default(),
            clip: Arc::new(Clip::default()),
            limbs: Vec::new(),
            pins: Vec::new(),
            painted: false,
            body: BodyProps::default(),
            objects: Vec::new(),
            revisions: Revisions::default(),
        }
    }

    pub fn revisions(&self) -> Revisions {
        self.revisions
    }

    /// Vrai si les poids ont été peints à la main (voir [`Rig::painted`]).
    pub fn weights_painted(&self) -> bool {
        self.painted
    }

    /// Marque les poids comme peints (plus de recalcul automatique) ou non.
    pub(crate) fn set_painted(&mut self, painted: bool) {
        self.painted = painted;
    }

    /// Incrémente la révision des poids pour que le rendu les renvoie (après
    /// une mutation de `skin` par un emprunt disjoint, pendant la peinture).
    pub(crate) fn bump_skin(&mut self) {
        self.revisions.skin = next_revision();
    }

    /// Vrai si les poids ne correspondent plus au squelette ou au maillage.
    pub fn needs_bind(&self) -> bool {
        self.revisions.bound_to != (self.revisions.skeleton, self.revisions.mesh)
    }

    /// Calcule les poids immédiatement (bloquant).
    pub fn bind(&mut self, method: BindMethod) {
        let skin = method.compute(&self.mesh, &self.skeleton);
        let key = (self.revisions.skeleton, self.revisions.mesh);
        self.apply_skin(skin, key);
    }

    /// Installe des poids calculés en tâche de fond pour `bound_to`. Ignoré
    /// si le squelette ou le maillage ont changé entre-temps.
    pub fn apply_skin(&mut self, skin: Skin, bound_to: (u64, u64)) -> bool {
        if bound_to != (self.revisions.skeleton, self.revisions.mesh)
            || skin.weights.len() != self.mesh.vertex_count()
        {
            return false;
        }
        self.skin = Arc::new(skin);
        self.revisions.skin = next_revision();
        self.revisions.bound_to = bound_to;
        true
    }

    /// Signale une modification du squelette de repos (mode édition).
    pub fn touch_skeleton(&mut self) {
        self.revisions.skeleton = next_revision();
    }

    pub fn touch_clip(&mut self) {
        self.revisions.clip = next_revision();
    }

    pub fn clip_mut(&mut self) -> &mut Clip {
        self.touch_clip();
        Arc::make_mut(&mut self.clip)
    }

    /// Remplace tout le squelette (auto-rig) : pose, animation et épingles
    /// sont réinitialisées car elles ne correspondent plus aux os.
    pub fn set_skeleton(&mut self, skeleton: Skeleton, limbs: Vec<Limb>) {
        // Objets attachés aux os : retrouvés par le nom de l'os.
        let anchors = self.object_rest_anchors();
        let old_names: Vec<String> = self.skeleton.bones().iter().map(|b| b.name.clone()).collect();
        self.pose = Pose::rest(&skeleton);
        self.skeleton = skeleton;
        self.limbs = limbs;
        self.pins.clear();
        // Squelette remplacé : les anciens poids peints ne correspondent plus.
        self.painted = false;
        self.clip = Arc::new(Clip::new(self.clip.fps, self.clip.frame_count));
        self.touch_clip();
        self.touch_skeleton();
        self.retarget_objects(&old_names, &anchors);
    }

    /// Remplace le maillage (import) en gardant le squelette.
    pub fn set_mesh(&mut self, mesh: Mesh) {
        self.mesh = Arc::new(mesh);
        self.skin = Arc::new(Skin {
            weights: vec![Default::default(); self.mesh.vertex_count()],
        });
        self.revisions.mesh = next_revision();
        self.revisions.skin = next_revision();
        // Nouveau maillage : poids remis à zéro, donc plus rien de peint.
        self.painted = false;
    }

    pub fn add_bone_world(&mut self, name: &str, parent: Option<BoneId>, head: Vec3, tail: Vec3) -> BoneId {
        let name = self.skeleton.unique_name(name);
        let id = self.skeleton.add_bone_world(name, parent, head, tail);
        self.pose.bones.push(Transform::IDENTITY);
        if !self.clip.is_empty() {
            self.clip_mut().push_bone();
        }
        self.touch_skeleton();
        id
    }

    /// Supprime un os ; ses enfants passent à son parent.
    pub fn remove_bone(&mut self, id: BoneId) {
        let anchors = self.object_anchors();
        let remap = self.skeleton.remove_bone(id);
        self.apply_remap_keeping(&remap, true, &anchors);
    }

    pub fn world(&self) -> Vec<Transform> {
        self.pose.world(&self.skeleton)
    }

    pub fn skinning_matrices(&self) -> Vec<Mat4> {
        self.pose.skinning_matrices(&self.skeleton)
    }
}
