//! Tests des projets `.kbrig`.

mod codec;
mod edit_count;
mod hostile;
mod objects;
mod roundtrip;
mod store;

use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

use glam::{Quat, Vec2, Vec3};

use super::ProjectSnapshot;
use crate::autorig::face::FaceMarkers;
use crate::editor::{EditTool, Mode, PoseTool, ViewOptions};
use crate::manipulator::{ManipulatorMode, TransformSpace};
use crate::mesh::Primitive;
use crate::scene::{BodyProps, Parent};
use crate::{
    AutoRigOptions, BindMethod, BoneId, Editor, Interpolation, LimbMode, ManipulatorSettings, Markers,
    Material, OrbitCamera, Pin, Rig, Submesh, Texture, Transform,
};

/// Dossier temporaire propre à un test, supprimé à la fin.
pub(super) struct TempDir(PathBuf);

impl TempDir {
    pub fn new(name: &str) -> Self {
        static NEXT: AtomicU32 = AtomicU32::new(0);
        let n = NEXT.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("bones-kbrig-{name}-{}-{n}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        Self(dir)
    }

    pub fn join(&self, name: &str) -> PathBuf {
        self.0.join(name)
    }

    pub fn path(&self) -> &Path {
        &self.0
    }

    /// Noms des fichiers du dossier, triés.
    pub fn files(&self) -> Vec<String> {
        let mut names: Vec<String> = std::fs::read_dir(&self.0)
            .unwrap()
            .flatten()
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        names
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// Petite image PNG valide.
pub(super) fn tiny_png(rgb: [u8; 3]) -> Vec<u8> {
    let pixels: Vec<u8> = (0..4).flat_map(|_| [rgb[0], rgb[1], rgb[2], 255]).collect();
    let img = image::RgbaImage::from_raw(2, 2, pixels).unwrap();
    let mut out = Vec::new();
    img.write_to(&mut std::io::Cursor::new(&mut out), image::ImageFormat::Png)
        .unwrap();
    out
}

/// Éditeur dont tout l'état enregistrable diffère des valeurs par défaut :
/// maillage texturé en plusieurs parties, modèle transformé, pose, clés,
/// membres, épingles, réglages de l'éditeur.
pub(super) fn full_editor() -> Editor {
    let mut rig = Rig::demo();
    rig.name = "Hero".into();
    rig.transform_model(&Transform {
        translation: Vec3::new(0.1, 0.0, -0.2),
        rotation: Quat::from_rotation_y(0.3),
        scale: Vec3::splat(1.1),
    });
    {
        let mesh = Arc::make_mut(&mut rig.mesh);
        mesh.uvs = mesh.positions.iter().map(|p| Vec2::new(p.x, p.y)).collect();
        let skin = Texture::new("skin.png", tiny_png([200, 150, 120]));
        mesh.materials = vec![
            Material {
                name: "Skin".into(),
                base_color_texture: Some(skin.clone()),
                ..Material::default()
            },
            Material {
                name: "Cloth".into(),
                base_color: [0.2, 0.3, 0.4, 1.0],
                base_color_texture: Some(skin),
                metallic: 0.5,
                roughness: 0.4,
            },
            Material {
                name: "Eyes".into(),
                base_color_texture: Some(Texture::new("eyes.png", tiny_png([10, 20, 200]))),
                ..Material::default()
            },
        ];
        let n = mesh.indices.len() as u32;
        let (a, b) = ((n / 9) * 3, (n / 6) * 3);
        mesh.submeshes = vec![
            Submesh {
                indices: 0..a,
                material: Some(0),
            },
            Submesh {
                indices: a..b,
                material: Some(2),
            },
            Submesh {
                indices: b..n,
                material: Some(1),
            },
        ];
    }
    rig.pose.bones[0].translation = Vec3::new(0.0, -0.05, 0.02);
    rig.pose.bones[1].rotation = Quat::from_rotation_z(0.4);
    let (first, second) = (rig.pose.clone(), {
        let mut p = rig.pose.clone();
        p.bones[2].rotation = Quat::from_rotation_x(-0.3);
        p
    });
    let clip = rig.clip_mut();
    clip.fps = 24.0;
    clip.set_key(0, first);
    clip.set_key(12, second);
    clip.key_at_mut(12).unwrap().interpolation = Interpolation::Step;
    clip.frame_count = 48;
    rig.limbs[0].mode = LimbMode::Fk;
    rig.limbs[1].target += Vec3::new(0.01, 0.02, 0.03);
    rig.pins.push(Pin {
        bone: BoneId(5),
        target: Vec3::new(0.3, 1.2, 0.1),
    });
    // Poids marqués comme peints : couvre le champ dans le test « tout champ
    // diffère du défaut » et son aller-retour.
    rig.set_painted(true);
    // Corps et objets supplémentaires : un accessoire rigide (caché,
    // verrouillé) attaché à un os, et un vêtement déformé à poids manuels.
    rig.body = BodyProps {
        name: "Hero body".into(),
        visible: true,
        locked: true,
    };
    let bone = rig.skeleton.find("head.x").unwrap_or(BoneId(0));
    let mut hat = Primitive::Cube.mesh();
    hat.uvs = hat.positions.iter().map(|p| Vec2::new(p.x, p.y)).collect();
    hat.materials = vec![Material {
        name: "Felt".into(),
        base_color: [0.1, 0.1, 0.1, 1.0],
        ..Material::default()
    }];
    let a = rig
        .add_object(
            "Hat",
            Arc::new(hat),
            Parent::Bone(bone),
            Transform::from_translation(Vec3::new(0.0, 0.2, 0.0)),
        )
        .unwrap();
    rig.object_mut(a).unwrap().visible = false;
    rig.object_mut(a).unwrap().locked = true;
    let cloth_mesh = Primitive::Sphere.mesh();
    let n = cloth_mesh.vertex_count();
    let b = rig
        .add_object(
            "Cloak",
            Arc::new(cloth_mesh),
            Parent::Object(a),
            Transform::from_translation(Vec3::new(0.1, 0.0, 0.0)),
        )
        .unwrap();
    rig.set_object_weights(
        b,
        crate::Skin {
            weights: (0..n)
                .map(|i| crate::VertexWeights::single(BoneId((i % 2) as u32)))
                .collect(),
        },
    )
    .unwrap();

    let mut e = Editor::new(rig);
    e.camera = OrbitCamera {
        target: Vec3::new(0.1, 0.9, 0.0),
        distance: 6.5,
        yaw: 1.2,
        pitch: -0.3,
        fov_y: 0.6,
        locked: true,
        orthographic: true,
    };
    e.mode = Mode::Edit;
    e.pose_tool = PoseTool::Rotate;
    e.edit_tool = EditTool::Roll;
    e.selected = Some(BoneId(3));
    e.chain_len = 4;
    e.bind_method = BindMethod::Envelope;
    e.view = ViewOptions {
        mesh: false,
        textures: false,
        weights: true,
        bones: false,
        grid: false,
        trajectory: false,
        ghosts: true,
        center_of_mass: true,
    };
    e.frame = 12.0;
    e.markers = Markers::guess(&e.rig.mesh);
    // Marqueurs du visage ajustés à la main (toutes les pièces facultatives
    // présentes, pour couvrir l'aller-retour de chaque champ).
    e.face_markers = Some(FaceMarkers {
        eye: Vec3::new(0.06, 1.62, 0.09),
        eye_radius: 0.018,
        brow: Vec3::new(0.055, 1.66, 0.1),
        nose_tip: Vec3::new(0.0, 1.58, 0.12),
        lip_top: Vec3::new(0.0, 1.54, 0.11),
        lip_bottom: Vec3::new(0.0, 1.52, 0.11),
        mouth_corner: Vec3::new(0.03, 1.53, 0.1),
        chin: Vec3::new(0.0, 1.49, 0.1),
        jaw: Vec3::new(0.0, 1.57, 0.02),
        ear: Some(Vec3::new(0.09, 1.6, -0.02)),
        teeth_top: Some(Vec3::new(0.0, 1.535, 0.1)),
        teeth_bottom: Some(Vec3::new(0.0, 1.525, 0.1)),
        tongue: Some([Vec3::new(0.0, 1.53, 0.05), Vec3::new(0.0, 1.53, 0.09)]),
    });
    e.autorig_options = AutoRigOptions {
        spine_count: 4,
        neck_count: 2,
        fingers: false,
        toes: false,
        fit_to_mesh: false,
        face: false,
    };
    e.mirror_edit = true;
    e.manipulator = ManipulatorSettings {
        mode: ManipulatorMode::Rotate,
        space: TransformSpace::Local,
        snap: true,
        angle_step_deg: 5.0,
        distance_step: 0.1,
        scale_step: 0.25,
        size: 120.0,
        visible: false,
        invert_snap: false,
    };
    e
}

/// Enregistre l'éditeur dans `dir/<name>.kbrig`.
pub(super) fn save(editor: &Editor, dir: &TempDir, name: &str) -> PathBuf {
    let path = dir.join(&format!("{name}.{}", super::EXTENSION));
    ProjectSnapshot::capture(editor)
        .save(&path, name, 1_700_000_000, Some(&thumbnail()))
        .unwrap();
    path
}

pub(super) fn thumbnail() -> Vec<u8> {
    let rgba: Vec<u8> = (0..32 * 24)
        .flat_map(|i| [(i % 256) as u8, 80, 160, 255])
        .collect();
    super::encode_thumbnail(32, 24, &rgba).unwrap()
}

/// Réécrit un projet en transformant une entrée (`None` : la supprimer).
pub(super) fn rewrite(path: &Path, name: &str, edit: impl FnOnce(Option<Vec<u8>>) -> Option<Vec<u8>>) {
    let mut source = zip::ZipArchive::new(std::fs::File::open(path).unwrap()).unwrap();
    let mut entries: Vec<(String, Vec<u8>)> = (0..source.len())
        .map(|i| {
            let mut entry = source.by_index(i).unwrap();
            let mut data = Vec::new();
            entry.read_to_end(&mut data).unwrap();
            (entry.name().unwrap().to_string(), data)
        })
        .collect();
    let index = entries.iter().position(|(n, _)| n == name);
    let old = index.map(|i| entries.remove(i).1);
    if let Some(data) = edit(old) {
        entries.insert(index.unwrap_or(entries.len()), (name.to_owned(), data));
    }
    let mut zip = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    for (name, data) in entries {
        zip.start_file(name, zip::write::SimpleFileOptions::default())
            .unwrap();
        zip.write_all(&data).unwrap();
    }
    std::fs::write(path, zip.finish().unwrap().into_inner()).unwrap();
}

/// Modifie le document JSON d'un projet.
pub(super) fn edit_document(path: &Path, edit: impl FnOnce(&mut serde_json::Value)) {
    rewrite(path, super::document::DOCUMENT, |data| {
        let mut doc: serde_json::Value = serde_json::from_slice(&data.unwrap()).unwrap();
        edit(&mut doc);
        Some(serde_json::to_vec(&doc).unwrap())
    });
}
