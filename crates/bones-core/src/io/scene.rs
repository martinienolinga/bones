//! Import « en objets » : chaque maillage d'un fichier devient un objet
//! séparé (hiérarchie des nœuds conservée, maillages rigides attachés à une
//! articulation rattachés à l'os, maillages skinnés gardant leurs poids),
//! au lieu d'être fusionné dans le corps comme par [`super::import`].
//!
//! - [`import_scene`] lit le fichier ; glTF/GLB donne un objet par nœud
//!   maillé, les autres formats un seul objet ;
//! - [`SceneImport::into_rig`] en fait un nouveau projet (le plus grand
//!   maillage skinné, sinon le plus grand maillage, devient le corps) ;
//! - `Editor::add_imported_objects` les ajoute au projet ouvert (os
//!   retrouvés par leur nom).

use std::path::Path;

use super::{Format, Imported, IoError, gltf, import_bytes, unsupported};
use crate::{BoneId, Clip, Mesh, Pose, Rig, Skeleton, Transform, VertexWeights};

/// Parent d'un objet importé.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ImportedParent {
    World,
    /// Objet importé précédent (indice dans [`SceneImport::objects`]).
    Object(usize),
    /// Os du squelette importé.
    Joint(BoneId),
}

/// Objet lu dans un fichier d'échange.
#[derive(Clone, Debug)]
pub struct ImportedObject {
    pub name: String,
    /// Maillage dans l'espace de l'objet.
    pub mesh: Mesh,
    pub parent: ImportedParent,
    /// Transformation relative au parent (à l'os posé pour une articulation).
    pub local: Transform,
    /// Transformation monde dans la pose du fichier.
    pub world: Transform,
    /// Poids par sommet (os du squelette importé) d'un maillage skinné.
    pub weights: Option<Vec<VertexWeights>>,
}

/// Contenu d'un fichier lu en objets séparés.
#[derive(Clone, Debug, Default)]
pub struct SceneImport {
    pub name: String,
    pub skeleton: Option<Skeleton>,
    /// Pose du fichier (celle où les objets ont été lus).
    pub pose: Pose,
    pub clip: Option<Clip>,
    /// Parents toujours avant leurs enfants.
    pub objects: Vec<ImportedObject>,
}

/// Lit un fichier en objets séparés (voir le module).
pub fn import_scene(path: &Path) -> Result<SceneImport, IoError> {
    let format = Format::from_path(path).ok_or_else(|| unsupported(path))?;
    let bytes = std::fs::read(path)?;
    let base = path.parent().unwrap_or(Path::new("."));
    let name = path
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "Import".into());
    import_scene_bytes(format, &bytes, &name, Some(base))
}

/// Comme [`import_scene`], depuis la mémoire.
pub fn import_scene_bytes(
    format: Format,
    bytes: &[u8],
    name: &str,
    base_dir: Option<&Path>,
) -> Result<SceneImport, IoError> {
    match format {
        Format::Glb | Format::Gltf => gltf::import_scene(bytes, name, base_dir),
        _ => Ok(match import_bytes(format, bytes, name, base_dir)? {
            Imported::Rig(rig) => SceneImport::from_rig(rig),
            Imported::Mesh(mesh) => SceneImport::from_mesh(name, mesh),
        }),
    }
}

impl SceneImport {
    /// Un seul objet, sans squelette.
    pub fn from_mesh(name: &str, mesh: Mesh) -> Self {
        Self {
            name: name.to_owned(),
            objects: vec![ImportedObject {
                name: name.to_owned(),
                mesh,
                parent: ImportedParent::World,
                local: Transform::IDENTITY,
                world: Transform::IDENTITY,
                weights: None,
            }],
            ..Self::default()
        }
    }

    /// Un document complet : son corps devient un objet (avec ses poids).
    pub fn from_rig(rig: Rig) -> Self {
        let skinned = !rig.skeleton.is_empty() && rig.skin.weights.len() == rig.mesh.vertex_count();
        let weights = skinned.then(|| rig.skin.weights.clone());
        let mut out = Self::from_mesh(&rig.name, (*rig.mesh).clone());
        out.objects[0].weights = weights;
        out.objects.retain(|o| o.mesh.triangle_count() > 0);
        out.pose = rig.pose.clone();
        out.clip = (!rig.clip.is_empty()).then(|| (*rig.clip).clone());
        out.skeleton = (!rig.skeleton.is_empty()).then_some(rig.skeleton);
        out
    }

    /// Nouveau projet : le plus grand maillage skinné (sinon le plus grand
    /// maillage) devient le corps, les autres des objets.
    pub fn into_rig(mut self) -> Rig {
        let size = |o: &ImportedObject| (o.weights.is_some(), o.mesh.triangle_count());
        let body = (0..self.objects.len()).max_by_key(|i| size(&self.objects[*i]));
        let mut rig = Rig::new(&self.name, Mesh::default());
        if let Some(skeleton) = self.skeleton.take() {
            rig.set_skeleton(skeleton, Vec::new());
        }
        if let Some(b) = body {
            let o = &self.objects[b];
            let mut mesh = o.mesh.clone();
            mesh.transform_by(&o.world);
            rig.set_mesh(mesh);
            if let Some(w) = o.weights.clone().filter(|w| w.len() == rig.mesh.vertex_count()) {
                let revs = rig.revisions();
                rig.apply_skin(crate::Skin { weights: w }, (revs.skeleton, revs.mesh));
            }
        }
        rig.pose = std::mem::take(&mut self.pose);
        rig.pose.sync_len(&rig.skeleton);
        if let Some(clip) = self.clip.take().filter(|c| !c.is_empty()) {
            rig.clip = std::sync::Arc::new(clip);
            rig.touch_clip();
        }
        let world = rig.world();
        rig.add_imported(self.objects, body, Some, true, &world);
        rig
    }
}
