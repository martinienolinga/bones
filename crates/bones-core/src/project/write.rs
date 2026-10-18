//! Enregistrement : compression parallèle des entrées, puis écriture
//! atomique du conteneur ZIP.

use std::borrow::Cow;
use std::path::Path;

use rayon::prelude::*;

use super::codec::encode_indices;
use super::container::{
    MAX_JSON, MAX_TEXTURE, MAX_THUMBNAIL, deflate, finish, store, write_atomically, writer,
};
use super::document::*;
use super::objects::entry as object_entry;
use super::split::split;
use super::{ProjectError, ProjectInfo, ProjectSnapshot, unix_now};

// Les tableaux binaires sont écrits tels qu'en mémoire : petit-boutiste,
// comme sur toutes les plateformes visées (ARM64, x86-64).
const _: () = assert!(
    cfg!(target_endian = "little"),
    "format .kbrig : petit-boutiste uniquement"
);

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
    // Mêmes limites qu'à la lecture : un projet enregistré reste lisible.
    document
        .counts
        .check(snapshot.rig.clip.keys().len(), snapshot.rig.skeleton.len())?;
    super::objects::check_objects(&document.objects)?;
    let sizes = heavy
        .textures
        .iter()
        .map(|(n, t)| (n.as_str(), t.bytes.len(), MAX_TEXTURE));
    let thumb = thumbnail.map(|t| (THUMBNAIL, t.len(), MAX_THUMBNAIL));
    if let Some((name, len, _)) = sizes.chain(thumb).find(|(_, len, max)| *len as u64 > *max) {
        return Err(ProjectError::TooLarge(format!("entry {name} ({len} bytes)")));
    }
    let rig = &snapshot.rig;
    let mesh = &*rig.mesh;

    // Une tâche par entrée : encodage et compression en parallèle (les
    // tableaux déjà au bon format sont compressés sans copie). Les entrées
    // du corps d'abord, puis la géométrie et les poids de chaque objet.
    type Encode<'a> = Box<dyn Fn() -> Result<Cow<'a, [u8]>, ProjectError> + Sync + 'a>;
    let mut entries: Vec<(String, Encode)> = vec![
        (DOCUMENT.into(), Box::new(|| json(&document))),
        (
            POSITIONS.into(),
            Box::new(|| raw(bytemuck::cast_slice(&mesh.positions))),
        ),
        (
            NORMALS.into(),
            Box::new(|| raw(bytemuck::cast_slice(&mesh.normals))),
        ),
        (UVS.into(), Box::new(|| raw(bytemuck::cast_slice(&mesh.uvs)))),
        (
            INDICES.into(),
            Box::new(|| Ok(Cow::Owned(encode_indices(&mesh.indices)))),
        ),
        (
            WEIGHTS.into(),
            Box::new(|| raw(bytemuck::cast_slice(&rig.skin.weights))),
        ),
        (POSES.into(), Box::new(|| raw(bytemuck::cast_slice(&heavy.poses)))),
    ];
    for (i, o) in heavy.objects.heavy.iter().enumerate() {
        let m = &*o.mesh;
        entries.push((
            object_entry(i, "positions"),
            Box::new(|| raw(bytemuck::cast_slice(&m.positions))),
        ));
        entries.push((
            object_entry(i, "normals"),
            Box::new(|| raw(bytemuck::cast_slice(&m.normals))),
        ));
        entries.push((
            object_entry(i, "uvs"),
            Box::new(|| raw(bytemuck::cast_slice(&m.uvs))),
        ));
        entries.push((
            object_entry(i, "indices"),
            Box::new(|| Ok(Cow::Owned(encode_indices(&m.indices)))),
        ));
        if let Some(skin) = &o.skin {
            entries.push((
                object_entry(i, "weights"),
                Box::new(|| raw(bytemuck::cast_slice(&skin.weights))),
            ));
        }
    }
    let prepared = entries
        .par_iter()
        .map(|(name, encode)| {
            let data = encode()?;
            (!data.is_empty()).then(|| deflate(name, &data)).transpose()
        })
        .collect::<Result<Vec<_>, _>>()?;

    write_atomically(path, |file| {
        let mut zip = writer(file);
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
        finish(zip)
    })?;
    Ok(info)
}

fn json(document: &Document) -> Result<Cow<'static, [u8]>, ProjectError> {
    let bytes = serde_json::to_vec(document)?;
    if bytes.len() as u64 > MAX_JSON {
        return Err(ProjectError::TooLarge(format!(
            "{DOCUMENT} ({} bytes)",
            bytes.len()
        )));
    }
    Ok(Cow::Owned(bytes))
}

fn raw(bytes: &[u8]) -> Result<Cow<'_, [u8]>, ProjectError> {
    Ok(Cow::Borrowed(bytes))
}
