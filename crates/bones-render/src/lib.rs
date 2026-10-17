//! Rendu 3D de PaintSharp Bones avec wgpu (Metal sur iPad).
//!
//! Choix orientés performance et mémoire :
//! - skinning sur le GPU : le maillage et les poids sont envoyés une seule
//!   fois (à chaque changement de révision) ; par image, seules les matrices
//!   des os transitent (64 octets par os) ;
//! - positions, normales, coordonnées de texture et poids sont envoyés
//!   directement depuis les tableaux du cœur, sans copie intermédiaire ;
//! - textures décodées et envoyées en parallèle (une par tâche rayon), avec
//!   mipmaps calculés sur le CPU ; les pixels sont libérés dès l'envoi ;
//! - os, poignées et lignes utilisent des tampons réutilisés, agrandis par
//!   puissances de deux ;
//! - une seule passe de rendu, MSAA 4× et profondeur en attachements
//!   transitoires (mémoire de tuile sur iPad, aucune mémoire vidéo).
//!
//! Les os, poignées et trajectoires sont dessinés « en rayons X », par-dessus
//! le maillage, comme dans les logiciels de rigging.
//!
//! Organisation :
//! - [`scene`] : données d'une image fournies par l'application ;
//! - [`renderer`] : état GPU et préparation ; `pass` : dessin ;
//! - `materials`, `textures` : matériaux et textures du maillage ;
//! - `pipelines`, `bindings`, `layouts`, `buffers`, `targets`, `geometry` :
//!   détails internes ;
//! - `shaders/` : sources WGSL.

mod bindings;
mod buffers;
mod geometry;
mod layouts;
mod materials;
mod pass;
mod pipelines;
pub mod renderer;
pub mod scene;
mod targets;
mod textures;

pub use geometry::{bone_matrix, handle_matrix};
pub use renderer::Renderer;
pub use scene::{BoneInstance, LineVertex, Scene};
