//! Conteneur ZIP partagé par les formats de fichiers de l'application
//! (projets `.kbrig`, préréglages…) :
//! - [`Archive`] : lecture bornée et vérifiée (fichiers reçus d'ailleurs) ;
//! - [`write_atomically`] : écriture atomique (temporaire puis renommage) ;
//! - [`writer`], [`deflate`], [`store`], [`finish`] : écriture des entrées.
//!
//! Exemple d'écriture :
//!
//! ```ignore
//! write_atomically(path, |file| {
//!     let mut zip = writer(file);
//!     zip.add_prepared_file(deflate("data.json", &json)?)?;
//!     store(&mut zip, "image.png", &png)?;
//!     finish(zip)
//! })?;
//! ```

mod archive;
mod atomic;
mod entries;
mod shared_file;

pub(crate) use archive::{Archive, MAX_JSON};
pub(crate) use atomic::write_atomically;
pub(crate) use entries::{deflate, finish, store, writer};

pub(super) use archive::{MAX_MANIFEST, MAX_TEXTURE, MAX_THUMBNAIL};
