//! Banc d'essai sur un dossier de fichiers réels (non versionnés) :
//! `BONES_IO_CORPUS=/chemin cargo test -p bones-core corpus -- --nocapture`.
//! Chaque fichier est importé, résumé, puis réexporté dans tous les formats.

use crate::io::{Format, Imported, export, import};

#[test]
fn corpus() {
    let Some(dir) = std::env::var_os("BONES_IO_CORPUS") else {
        return;
    };
    let out = super::temp_dir("corpus");
    let mut entries: Vec<_> = std::fs::read_dir(dir)
        .unwrap()
        .filter_map(Result::ok)
        .map(|e| e.path())
        .collect();
    entries.sort();
    for path in entries {
        if Format::from_path(&path).is_none() {
            continue;
        }
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        let mut rig = match import(&path) {
            Ok(Imported::Rig(rig)) => rig,
            Ok(Imported::Mesh(mesh)) => crate::Rig::new("maillage", mesh),
            Err(e) => {
                println!("{name:28} ERREUR {e}");
                continue;
            }
        };
        let m = &rig.mesh;
        let textures = m
            .materials
            .iter()
            .filter(|x| x.base_color_texture.is_some())
            .count();
        let (lo, hi) = m.bounds().unwrap_or_default();
        println!(
            "{name:28} {:6} sommets {:6} tri {} mat ({textures} tex) uv={} | {} os {} clés @{} ips, liaison {} | {lo:.2} → {hi:.2}",
            m.vertex_count(),
            m.triangle_count(),
            m.materials.len(),
            m.has_uvs(),
            rig.skeleton.len(),
            rig.clip.keys().len(),
            rig.clip.fps,
            if rig.needs_bind() { "à calculer" } else { "lue" },
        );
        // Maillage déformé à mi-animation : doit rester cohérent.
        if !rig.clip.is_empty() {
            let mut pose = rig.pose.clone();
            rig.clip.sample_into(rig.clip.frame_count as f32 / 2.0, &mut pose);
            rig.pose = pose;
            if let Some((lo, hi)) = crate::io::rigging::posed_mesh(&rig).bounds() {
                println!("{:28} posé à mi-parcours : {lo:.2} → {hi:.2}", "");
            }
        }
        for format in Format::exportable() {
            let mesh_only = matches!(format, Format::Obj | Format::Stl | Format::Ply);
            if (format == Format::Bvh && rig.skeleton.is_empty())
                || (mesh_only && rig.mesh.indices.is_empty())
            {
                continue;
            }
            let target = out.join(format!(
                "{}.{}",
                path.file_stem().unwrap().to_string_lossy(),
                format.extension()
            ));
            if let Err(e) = export(&rig, format, &target).and_then(|_| import(&target)) {
                println!("    export {format:?} : {e}");
            }
        }
    }
    let _ = std::fs::remove_dir_all(&out);
}
