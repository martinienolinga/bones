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
            revisions: Revisions::default(),
        }
    }

    pub fn revisions(&self) -> Revisions {
        self.revisions
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
        self.pose = Pose::rest(&skeleton);
        self.skeleton = skeleton;
        self.limbs = limbs;
        self.pins.clear();
        self.clip = Arc::new(Clip::new(self.clip.fps, self.clip.frame_count));
        self.touch_clip();
        self.touch_skeleton();
    }

    /// Remplace le maillage (import) en gardant le squelette.
    pub fn set_mesh(&mut self, mesh: Mesh) {
        self.mesh = Arc::new(mesh);
        self.skin = Arc::new(Skin {
            weights: vec![Default::default(); self.mesh.vertex_count()],
        });
        self.revisions.mesh = next_revision();
        self.revisions.skin = next_revision();
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

    pub fn remove_bone(&mut self, id: BoneId) {
        let remap = self.skeleton.remove_bone(id);
        self.pose.remap(&remap);
        if !self.clip.is_empty() {
            self.clip_mut().remap(&remap);
        }
        let mut skin = (*self.skin).clone();
        skin.remap(&remap);
        self.skin = Arc::new(skin);
        self.revisions.skin = next_revision();
        self.limbs = self.limbs.iter().filter_map(|l| l.remap(&remap)).collect();
        self.pins = self
            .pins
            .iter()
            .filter_map(|p| {
                Some(Pin {
                    bone: remap.get(p.bone)?,
                    ..*p
                })
            })
            .collect();
        // Les poids ont été remappés : ils restent valables pour le nouveau
        // squelette, inutile de relancer la liaison pour cette seule raison.
        let was_bound = !self.needs_bind();
        self.touch_skeleton();
        if was_bound {
            self.revisions.bound_to = (self.revisions.skeleton, self.revisions.mesh);
        }
    }

    pub fn world(&self) -> Vec<Transform> {
        self.pose.world(&self.skeleton)
    }

    pub fn skinning_matrices(&self) -> Vec<Mat4> {
        self.pose.skinning_matrices(&self.skeleton)
    }
}
