//! Écriture des entrées : compressées (deflate, préparables en parallèle)
//! ou stockées telles quelles (données déjà compressées).

use std::fs::File;
use std::io::{BufWriter, Seek, Write};

use zip::write::{PreparedZipFile, SimpleFileOptions, ZipFileBuilder};
use zip::{CompressionMethod, ZipWriter};

use crate::project::ProjectError;

/// Niveau de compression deflate (zlib-rs) : bon compromis vitesse / taille.
const LEVEL: i64 = 6;

/// Conteneur écrit dans un fichier (tamponné).
pub(crate) fn writer(file: File) -> ZipWriter<BufWriter<File>> {
    ZipWriter::new(BufWriter::with_capacity(1 << 16, file))
}

/// Termine le conteneur et synchronise le fichier sur le disque (à appeler
/// en fin de [`super::write_atomically`]).
pub(crate) fn finish(zip: ZipWriter<BufWriter<File>>) -> Result<(), ProjectError> {
    let file = zip.finish()?.into_inner().map_err(|e| e.into_error())?;
    file.sync_all()?;
    Ok(())
}

/// Entrée compressée (deflate), prête à être ajoutée au conteneur avec
/// `ZipWriter::add_prepared_file` ; peut être préparée sur un autre fil.
pub(crate) fn deflate(name: &str, data: &[u8]) -> Result<PreparedZipFile, ProjectError> {
    let options = SimpleFileOptions::default()
        .compression_method(CompressionMethod::Deflated)
        .compression_level(Some(LEVEL));
    let mut builder = ZipFileBuilder::new(name, options)?;
    builder.write_all(data)?;
    Ok(builder.finish()?)
}

/// Entrée écrite sans recompression (JPEG, PNG : déjà compressés).
pub(crate) fn store<W: Write + Seek>(
    zip: &mut ZipWriter<W>,
    name: &str,
    data: &[u8],
) -> Result<(), ProjectError> {
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);
    zip.start_file(name, options)?;
    zip.write_all(data)?;
    Ok(())
}
