//! Formats de fichiers reconnus et leurs capacités.

use std::path::Path;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Format {
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
    pub const ALL: [Format; 7] = [
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
            Format::Glb => "Binary glTF (Blender, Unity, Unreal, Godot)",
            Format::Gltf => "glTF text + separate files",
            Format::Fbx => "FBX (Maya, 3ds Max, Mixamo)",
            Format::Obj => "Wavefront OBJ + MTL",
            Format::Stl => "STL (3D printing)",
            Format::Ply => "PLY (3D scans)",
            Format::Bvh => "BVH (motion capture)",
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
        matches!(self, Format::Glb | Format::Gltf | Format::Bvh)
    }

    /// Le format peut porter l'animation.
    pub fn exports_animation(self) -> bool {
        self.exports_skeleton()
    }

    /// Le format peut porter des textures.
    pub fn supports_textures(self) -> bool {
        matches!(self, Format::Glb | Format::Gltf | Format::Fbx | Format::Obj)
    }

    pub fn importable() -> impl Iterator<Item = Format> {
        Format::ALL.into_iter().filter(|f| f.can_import())
    }

    pub fn exportable() -> impl Iterator<Item = Format> {
        Format::ALL.into_iter().filter(|f| f.can_export())
    }
}
