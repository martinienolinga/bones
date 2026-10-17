//! Enregistrement : compression parallèle des entrées, puis écriture
//! atomique du conteneur ZIP.

use std::borrow::Cow;
use std::io::{BufWriter, Seek, Write};
use std::path::Path;

use rayon::prelude::*;
use zip::write::{PreparedZipFile, SimpleFileOptions, ZipFileBuilder};
use zip::{CompressionMethod, ZipWriter};

use super::atomic::write_atomically;
use super::codec::encode_indices;
use super::document::*;
use super::split::split;
use super::{ProjectError, ProjectInfo, ProjectSnapshot, unix_now};

// Les tableaux binaires sont écrits tels qu'en mémoire : petit-boutiste,
// comme sur toutes les plateformes visées (ARM64, x86-64).
const _: () = assert!(
    cfg!(target_endian = "little"),
    "format .kbrig : petit-boutiste uniquement"
);

/// Niveau de compression deflate (zlib-rs) : bon compromis vitesse / taille.
const LEVEL: i64 = 6;

pub(super) fn save(
    snapshot: &ProjectSnapshot,
    path: &Path,
    name: &str,
    created: u64,
    thumbnail: Option<&[u8]>,
) -> Result<ProjectInfo, ProjectError> {
    let info = snapshot.info(name, created, unix_now());
    let manifest = deflate(MANIFEST, &serde_json::to_vec_pretty(&Manifest::new(&info))?)?;
    let (document, heavy) = split(snapshot);
    let rig = &snapshot.rig;
    let mesh = &*rig.mesh;

    // Une tâche par entrée : encodage et compression en parallèle (les
    // tableaux déjà au bon format sont compressés sans copie).
    type Encode<'a> = &'a (dyn Fn() -> Result<Cow<'a, [u8]>, ProjectError> + Sync);
    let entries: [(&str, Encode); 7] = [
        (DOCUMENT, &|| Ok(Cow::Owned(serde_json::to_vec(&document)?))),
        (POSITIONS, &|| raw(bytemuck::cast_slice(&mesh.positions))),
        (NORMALS, &|| raw(bytemuck::cast_slice(&mesh.normals))),
        (UVS, &|| raw(bytemuck::cast_slice(&mesh.uvs))),
        (INDICES, &|| Ok(Cow::Owned(encode_indices(&mesh.indices)))),
        (WEIGHTS, &|| raw(bytemuck::cast_slice(&rig.skin.weights))),
        (POSES, &|| raw(bytemuck::cast_slice(&heavy.poses))),
    ];
    let prepared = entries
        .par_iter()
        .map(|(name, encode)| {
            let data = encode()?;
            (!data.is_empty()).then(|| deflate(name, &data)).transpose()
        })
        .collect::<Result<Vec<_>, _>>()?;

    write_atomically(path, |file| {
        let mut zip = ZipWriter::new(BufWriter::with_capacity(1 << 16, file));
        // Manifeste puis vignette en tête : la galerie les lit sans rien
        // décompresser d'autre.
        zip.add_prepared_file(manifest)?;
        if let Some(jpeg) = thumbnail {
            store(&mut zip, THUMBNAIL, jpeg)?;
        }
        for entry in prepared.into_iter().flatten() {
            zip.add_prepared_file(entry)?;
        }
        for (name, texture) in &heavy.textures {
            store(&mut zip, name, &texture.bytes)?;
        }
        let file = zip.finish()?.into_inner().map_err(|e| e.into_error())?;
        file.sync_all()?;
        Ok(())
    })?;
    Ok(info)
}

fn raw(bytes: &[u8]) -> Result<Cow<'_, [u8]>, ProjectError> {
    Ok(Cow::Borrowed(bytes))
}

/// Entrée compressée (deflate), prête à être ajoutée au conteneur.
pub(super) fn deflate(name: &str, data: &[u8]) -> Result<PreparedZipFile, ProjectError> {
    let options = SimpleFileOptions::default()
        .compression_method(CompressionMethod::Deflated)
        .compression_level(Some(LEVEL));
    let mut builder = ZipFileBuilder::new(name, options)?;
    builder.write_all(data)?;
    Ok(builder.finish()?)
}

/// Entrée écrite sans recompression (JPEG, PNG : déjà compressés).
fn store<W: Write + Seek>(zip: &mut ZipWriter<W>, name: &str, data: &[u8]) -> Result<(), ProjectError> {
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);
    zip.start_file(name, options)?;
    zip.write_all(data)?;
    Ok(())
}
