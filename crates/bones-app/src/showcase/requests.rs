//! Échange entre l'interface et le rendu des vignettes, par les données
//! temporaires du contexte egui (comme `bones_ui::glass`) : avant
//! l'interface, les textures disponibles par emplacement ; pendant, les
//! vignettes demandées par les cartes visibles, dans l'ordre.

use std::sync::Arc;

use bones_core::Rig;
use glam::{Mat4, Vec3};

/// Vignette demandée par une carte.
#[derive(Clone)]
pub struct TileRequest {
    /// Identifie le contenu (préréglage, image, taille) : une vignette déjà
    /// rendue avec la même clé n'est pas redessinée.
    pub key: u64,
    /// Taille en pixels physiques.
    pub size: [u32; 2],
    /// Matrices de skinning du modèle.
    pub joints: Arc<[Mat4]>,
    pub view_proj: Mat4,
    pub eye: Vec3,
}

/// Demandes d'une image : le modèle (maillage et poids) et les vignettes.
#[derive(Clone, Default)]
pub(super) struct Batch {
    pub model: Option<Arc<Rig>>,
    pub requests: Vec<TileRequest>,
}

/// Textures egui des emplacements, publiées avant l'interface.
#[derive(Clone, Default)]
struct Published {
    textures: Vec<Option<egui::TextureId>>,
}

fn batch_id() -> egui::Id {
    egui::Id::unique("bones-showcase-batch")
}

fn published_id() -> egui::Id {
    egui::Id::unique("bones-showcase-published")
}

/// Avant l'interface : textures disponibles par emplacement.
pub(super) fn publish(ctx: &egui::Context, textures: Vec<Option<egui::TextureId>>) {
    ctx.data_mut(|d| d.insert_temp(published_id(), Published { textures }));
}

/// Demande une vignette de `model` ; retourne la texture où elle sera
/// rendue avant l'affichage de cette image, ou `None` si son emplacement
/// n'existe pas encore (il sera prêt à l'image suivante).
pub fn request(ctx: &egui::Context, model: &Arc<Rig>, tile: TileRequest) -> Option<egui::TextureId> {
    ctx.data_mut(|d| {
        let batch = d.get_temp_mut_or_default::<Batch>(batch_id());
        if !batch.model.as_ref().is_some_and(|m| Arc::ptr_eq(m, model)) {
            batch.model = Some(model.clone());
        }
        let slot = batch.requests.len();
        batch.requests.push(tile);
        let published = d.get_temp::<Published>(published_id()).unwrap_or_default();
        published.textures.get(slot).copied().flatten()
    })
}

/// Après l'interface : demandes de l'image (vidées).
pub(super) fn take(ctx: &egui::Context) -> Batch {
    ctx.data_mut(|d| d.remove_temp::<Batch>(batch_id()).unwrap_or_default())
}
