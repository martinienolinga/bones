//! Formats de fichiers reconnus et leurs capacités.

use std::path::Path;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Format {
    /// Document natif `.bones` (JSON, textures incluses).
    Bones,
    /// glTF 2.0 binaire.
    Glb,
    /// glTF 2.0 texte (+ `.bin` et images à côté).
    Gltf,
    /// Autodesk FBX binaire 7.x.
    Fbx,
    /// Wavefront OBJ (+ MTL et textures).
    Obj,
    /// Stéréolithographie (impression 3D).
    Stl,
    /// Stanford PLY (scans 3D).
    Ply,
    /// Biovision Hierarchy (capture de mouvement).
    Bvh,
}

impl Format {
    pub const ALL: [Format; 8] = [
        Format::Bones,
        Format::Glb,
        Format::Gltf,
        Format::Fbx,
        Format::Obj,
        Format::Stl,
        Format::Ply,
        Format::Bvh,
    ];

    /// Extensions acceptées (en minuscules, sans point) ; la première est
    /// celle utilisée à l'export.
    pub fn extensions(self) -> &'static [&'static str] {
        match self {
            Format::Bones => &["bones"],
            Format::Glb => &["glb"],
            Format::Gltf => &["gltf"],
            Format::Fbx => &["fbx"],
            Format::Obj => &["obj"],
            Format::Stl => &["stl"],
            Format::Ply => &["ply"],
            Format::Bvh => &["bvh"],
        }
    }

    pub fn extension(self) -> &'static str {
        self.extensions()[0]
    }

    pub fn from_extension(ext: &str) -> Option<Format> {
        let ext = ext.trim_start_matches('.').to_ascii_lowercase();
        Format::ALL
            .into_iter()
            .find(|f| f.extensions().contains(&ext.as_str()))
    }

    pub fn from_path(path: &Path) -> Option<Format> {
        Format::from_extension(path.extension()?.to_str()?)
    }

    /// Nom affiché dans l'interface.
    pub fn label(self) -> &'static str {
        match self {
            Format::Bones => "Document PaintSharp Bones",
            Format::Glb => "glTF binaire (Blender, Unity, Unreal, Godot)",
            Format::Gltf => "glTF texte + fichiers séparés",
            Format::Fbx => "FBX (Maya, 3ds Max, Mixamo)",
            Format::Obj => "Wavefront OBJ + MTL",
            Format::Stl => "STL (impression 3D)",
            Format::Ply => "PLY (scans 3D)",
            Format::Bvh => "BVH (capture de mouvement)",
        }
    }

    pub fn can_import(self) -> bool {
        true
    }

    pub fn can_export(self) -> bool {
        !matches!(self, Format::Fbx)
    }

    /// Le format peut porter un squelette.
    pub fn exports_skeleton(self) -> bool {
        matches!(self, Format::Bones | Format::Glb | Format::Gltf | Format::Bvh)
    }

    /// Le format peut porter l'animation.
    pub fn exports_animation(self) -> bool {
        self.exports_skeleton()
    }

    /// Le format peut porter des textures.
    pub fn supports_textures(self) -> bool {
        matches!(
            self,
            Format::Bones | Format::Glb | Format::Gltf | Format::Fbx | Format::Obj
        )
    }

    pub fn importable() -> impl Iterator<Item = Format> {
        Format::ALL.into_iter().filter(|f| f.can_import())
    }

    pub fn exportable() -> impl Iterator<Item = Format> {
        Format::ALL.into_iter().filter(|f| f.can_export())
    }
}
