//! Import minimal du format Wavefront OBJ (positions et faces). Les faces
//! polygonales sont triangulées en éventail ; les normales sont recalculées.

use glam::Vec3;

use crate::Mesh;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObjError {
    pub line: usize,
    pub message: String,
}

impl std::fmt::Display for ObjError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "OBJ ligne {} : {}", self.line, self.message)
    }
}

impl std::error::Error for ObjError {}

pub fn parse(source: &str) -> Result<Mesh, ObjError> {
    let mut mesh = Mesh::default();
    let mut face: Vec<u32> = Vec::with_capacity(8);

    for (n, line) in source.lines().enumerate() {
        let err = |message: String| ObjError { line: n + 1, message };
        let mut tokens = line.split_whitespace();
        match tokens.next() {
            Some("v") => {
                let mut c = [0.0f32; 3];
                for v in &mut c {
                    *v = tokens
                        .next()
                        .ok_or_else(|| err("sommet incomplet".into()))?
                        .parse()
                        .map_err(|e| err(format!("coordonnée invalide ({e})")))?;
                }
                mesh.positions.push(Vec3::from_array(c));
            }
            Some("f") => {
                face.clear();
                for token in tokens {
                    let raw = token.split('/').next().unwrap_or_default();
                    let idx: i64 = raw
                        .parse()
                        .map_err(|e| err(format!("indice invalide « {token} » ({e})")))?;
                    let count = mesh.positions.len() as i64;
                    let resolved = if idx < 0 { count + idx } else { idx - 1 };
                    if !(0..count).contains(&resolved) {
                        return Err(err(format!("indice hors limites : {idx}")));
                    }
                    face.push(resolved as u32);
                }
                if face.len() < 3 {
                    return Err(err("face de moins de trois sommets".into()));
                }
                for i in 1..face.len() - 1 {
                    mesh.indices.extend_from_slice(&[face[0], face[i], face[i + 1]]);
                }
            }
            _ => {}
        }
    }

    if mesh.indices.is_empty() {
        return Err(ObjError {
            line: 0,
            message: "aucune face".into(),
        });
    }
    mesh.compute_normals();
    mesh.shrink_to_fit();
    Ok(mesh)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_quad_with_slashes_and_negative_indices() {
        let src = "# carré\nv 0 0 0\nv 1 0 0\nv 1 1 0\nv 0 1 0\nvn 0 0 1\nf 1//1 2//1 -2//1 -1//1\n";
        let mesh = parse(src).unwrap();
        assert_eq!(mesh.vertex_count(), 4);
        assert_eq!(mesh.indices, vec![0, 1, 2, 0, 2, 3]);
        assert!(mesh.normals[0].abs_diff_eq(Vec3::Z, 1e-6));
    }

    #[test]
    fn reports_bad_index() {
        let e = parse("v 0 0 0\nf 1 2 3\n").unwrap_err();
        assert_eq!(e.line, 2);
    }
}
