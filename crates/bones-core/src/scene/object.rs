//! Objet de la scène, sa liaison au squelette et les propriétés du corps.

use std::sync::Arc;

use serde::{Deserialize, Serialize};

use super::bind::BindStamp;
use super::{ObjectId, Parent};
use crate::{BindMethod, Mesh, Skin, Transform, VertexWeights, next_revision};

/// Origine des poids d'un objet déformé.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SkinMethod {
    /// Copiés du corps au point le plus proche (« Data Transfer », face la
    /// plus proche interpolée, de Blender) : idéal pour les vêtements.
    #[default]
    Transfer,
    /// Diffusion de chaleur (Baran & Popović), comme le corps.
    Heat,
    /// Enveloppes par distance aux os.
    Envelope,
    /// Poids importés ou fournis : jamais recalculés automatiquement.
    Manual,
}

impl SkinMethod {
    /// Poids recalculés automatiquement quand le squelette change.
    pub fn is_automatic(self) -> bool {
        self != SkinMethod::Manual
    }

    /// Méthode de calcul directe (hors transfert et poids manuels).
    pub(crate) fn bind_method(self) -> Option<BindMethod> {
        match self {
            SkinMethod::Heat => Some(BindMethod::Heat),
            SkinMethod::Envelope => Some(BindMethod::Envelope),
            SkinMethod::Transfer | SkinMethod::Manual => None,
        }
    }
}

/// Poids d'un objet déformé par le squelette.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SkinBinding {
    /// Un poids par sommet du maillage de l'objet (os du squelette du rig).
    pub skin: Arc<Skin>,
    pub method: SkinMethod,
    /// État pour lequel les poids ont été calculés (non sérialisé).
    #[serde(skip)]
    pub(crate) bound_to: Option<BindStamp>,
}

/// Liaison d'un objet au squelette.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub enum Binding {
    /// Suit rigidement son parent (accessoire).
    #[default]
    Rigid,
    /// Déformé par le squelette avec ses propres poids (vêtement, partie
    /// du corps séparée).
    Skinned(SkinBinding),
}

impl Binding {
    pub fn is_skinned(&self) -> bool {
        matches!(self, Binding::Skinned(_))
    }

    pub fn skin(&self) -> Option<&SkinBinding> {
        match self {
            Binding::Skinned(s) => Some(s),
            Binding::Rigid => None,
        }
    }
}

/// Révisions d'un objet (non sérialisées), pour les caches GPU.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ObjectRevisions {
    pub mesh: u64,
    pub skin: u64,
}

impl Default for ObjectRevisions {
    fn default() -> Self {
        Self {
            mesh: next_revision(),
            skin: next_revision(),
        }
    }
}

/// Objet supplémentaire de la scène. Les champs structurels (maillage,
/// parent, liaison) ne changent que par les méthodes du `Rig`, qui tiennent
/// les révisions et l'absence de cycle à jour.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SceneObject {
    pub(crate) id: ObjectId,
    pub name: String,
    pub(crate) mesh: Arc<Mesh>,
    #[serde(default)]
    pub(crate) parent: Parent,
    /// Transformation relative au parent (à l'os posé pour un os).
    #[serde(default)]
    pub(crate) local: Transform,
    #[serde(default)]
    pub(crate) binding: Binding,
    #[serde(default = "visible_default")]
    pub visible: bool,
    /// Verrouillé : ni sélection dans la vue 3D, ni transformation (comme
    /// « Disable Selection » de Blender) ; reste sélectionnable dans la liste.
    #[serde(default)]
    pub locked: bool,
    #[serde(skip)]
    pub(crate) revisions: ObjectRevisions,
}

fn visible_default() -> bool {
    true
}

impl SceneObject {
    pub(crate) fn new(id: ObjectId, name: String, mesh: Arc<Mesh>, parent: Parent, local: Transform) -> Self {
        Self {
            id,
            name,
            mesh,
            parent,
            local,
            binding: Binding::Rigid,
            visible: true,
            locked: false,
            revisions: ObjectRevisions::default(),
        }
    }

    pub fn id(&self) -> ObjectId {
        self.id
    }

    pub fn mesh(&self) -> &Arc<Mesh> {
        &self.mesh
    }

    pub fn parent(&self) -> Parent {
        self.parent
    }

    pub fn local(&self) -> Transform {
        self.local
    }

    pub fn binding(&self) -> &Binding {
        &self.binding
    }

    pub fn revisions(&self) -> ObjectRevisions {
        self.revisions
    }

    /// Poids vides (un par sommet), en attente de calcul.
    pub(crate) fn empty_skin(&self) -> Arc<Skin> {
        Arc::new(Skin {
            weights: vec![VertexWeights::default(); self.mesh.vertex_count()],
        })
    }

    pub(crate) fn bump_skin(&mut self) {
        self.revisions.skin = next_revision();
    }
}

/// Propriétés du corps (maillage principal) dans la liste des objets.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct BodyProps {
    pub name: String,
    pub visible: bool,
    /// Verrouillé : ni sélection dans la vue 3D, ni transformation.
    pub locked: bool,
}

impl Default for BodyProps {
    fn default() -> Self {
        Self {
            name: "Body".into(),
            visible: true,
            locked: false,
        }
    }
}
