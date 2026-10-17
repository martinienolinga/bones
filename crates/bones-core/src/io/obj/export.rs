//! Export OBJ + MTL du maillage dans la pose courante, textures copiées à
//! côté du fichier.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use super::mtl::shininess;
use crate::Rig;
use crate::io::rigging::posed_mesh;
use crate::io::{IoError, safe_file_name};

pub fn export(rig: &Rig, path: &Path) -> Result<Vec<PathBuf>, IoError> {
    let mesh = posed_mesh(rig);
    let dir = path.parent().unwrap_or(Path::new("."));
    let stem = path
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "modele".into());
    let mut written = vec![path.to_path_buf()];
    let names: Vec<String> = mesh
        .materials
        .iter()
        .enumerate()
        .map(|(i, m)| safe_file_name(&m.name, &format!("materiau_{i}")))
        .collect();

    let mut obj = String::with_capacity(mesh.vertex_count() * 80);
    let _ = writeln!(obj, "# PaintSharp Bones");
    if !mesh.materials.is_empty() {
        let mtl_name = format!("{stem}.mtl");
        let _ = writeln!(obj, "mtllib {mtl_name}");
        let mtl_path = dir.join(&mtl_name);
        std::fs::write(&mtl_path, write_mtl(&mesh, &names, &stem, dir, &mut written)?)?;
        written.insert(1, mtl_path);
    }
    let _ = writeln!(obj, "o {}", safe_file_name(&rig.name, "modele"));
    for p in &mesh.positions {
        let _ = writeln!(obj, "v {} {} {}", p.x, p.y, p.z);
    }
    let uvs = mesh.has_uvs();
    if uvs {
        for t in &mesh.uvs {
            let _ = writeln!(obj, "vt {} {}", t.x, 1.0 - t.y);
        }
    }
    for n in &mesh.normals {
        let _ = writeln!(obj, "vn {} {} {}", n.x, n.y, n.z);
    }
    for part in mesh.parts() {
        if let Some(m) = part.material {
            let _ = writeln!(obj, "usemtl {}", names[m]);
        }
        let range = part.indices.start as usize..part.indices.end as usize;
        for tri in mesh.indices[range].as_chunks::<3>().0 {
            obj.push('f');
            for i in tri {
                let i = i + 1;
                let _ = if uvs {
                    write!(obj, " {i}/{i}/{i}")
                } else {
                    write!(obj, " {i}//{i}")
                };
            }
            obj.push('\n');
        }
    }
    std::fs::write(path, obj)?;
    Ok(written)
}

fn write_mtl(
    mesh: &crate::Mesh,
    names: &[String],
    stem: &str,
    dir: &Path,
    written: &mut Vec<PathBuf>,
) -> Result<String, IoError> {
    let mut mtl = String::from("# PaintSharp Bones\n");
    for (m, name) in mesh.materials.iter().zip(names) {
        let [r, g, b, a] = m.base_color;
        let _ = writeln!(mtl, "\nnewmtl {name}\nKd {r} {g} {b}\nd {a}");
        let _ = writeln!(mtl, "Ns {}\nillum 2", shininess(m.roughness));
        if let Some(tex) = &m.base_color_texture {
            let file = format!("{stem}_{name}.{}", tex.extension());
            let tex_path = dir.join(&file);
            std::fs::write(&tex_path, &tex.bytes)?;
            written.push(tex_path);
            let _ = writeln!(mtl, "map_Kd {file}");
        }
    }
    Ok(mtl)
}
