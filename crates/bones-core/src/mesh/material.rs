//! Matériaux et textures des modèles importés.
//!
//! Les textures sont gardées sous leur forme encodée (PNG, JPEG…) telle
//! qu'elle a été lue : c'est bien plus compact en mémoire qu'une image
//! décodée, et cela permet de les réécrire à l'identique à l'export. Le
//! décodage en RGBA n'a lieu qu'au moment de l'envoi au GPU.

use std::path::Path;
use std::sync::Arc;

use serde::{Deserialize, Serialize};

/// Matériau PBR simplifié (modèle « metallic-roughness » de glTF).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Material {
    pub name: String,
    /// Couleur de base linéaire RGBA.
    pub base_color: [f32; 4],
    #[serde(default)]
    pub base_color_texture: Option<Texture>,
    pub metallic: f32,
    pub roughness: f32,
}

impl Default for Material {
    fn default() -> Self {
        Self {
            name: "Matériau".into(),
            base_color: [0.8, 0.8, 0.8, 1.0],
            base_color_texture: None,
            metallic: 0.0,
            roughness: 0.9,
        }
    }
}

/// Image encodée (PNG, JPEG, TGA, BMP), partagée sans copie.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Texture {
    pub name: String,
    /// Type MIME (`image/png`, `image/jpeg`…).
    pub mime: String,
    #[serde(with = "base64_bytes")]
    pub bytes: Arc<[u8]>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextureError(pub String);

impl std::fmt::Display for TextureError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "texture illisible : {}", self.0)
    }
}

impl std::error::Error for TextureError {}

impl Texture {
    pub fn new(name: impl Into<String>, bytes: impl Into<Arc<[u8]>>) -> Self {
        let bytes = bytes.into();
        Self {
            name: name.into(),
            mime: sniff_mime(&bytes).to_owned(),
            bytes,
        }
    }

    /// Lit une image depuis un fichier.
    pub fn from_file(path: &Path) -> std::io::Result<Self> {
        let bytes = std::fs::read(path)?;
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        Ok(Self::new(name, bytes))
    }

    /// Extension de fichier correspondant au type MIME.
    pub fn extension(&self) -> &'static str {
        match self.mime.as_str() {
            "image/jpeg" => "jpg",
            "image/x-tga" => "tga",
            "image/bmp" => "bmp",
            _ => "png",
        }
    }

    /// Décode l'image en RGBA 8 bits : (largeur, hauteur, pixels).
    pub fn decode_rgba8(&self) -> Result<(u32, u32, Vec<u8>), TextureError> {
        let format = match self.mime.as_str() {
            "image/png" => Some(image::ImageFormat::Png),
            "image/jpeg" => Some(image::ImageFormat::Jpeg),
            "image/x-tga" => Some(image::ImageFormat::Tga),
            "image/bmp" => Some(image::ImageFormat::Bmp),
            _ => None,
        };
        let decoded = match format {
            Some(f) => image::load_from_memory_with_format(&self.bytes, f),
            None => image::load_from_memory(&self.bytes),
        }
        .map_err(|e| TextureError(e.to_string()))?;
        let rgba = decoded.into_rgba8();
        Ok((rgba.width(), rgba.height(), rgba.into_raw()))
    }
}

/// Type MIME deviné d'après les premiers octets (ou l'extension à défaut).
pub fn sniff_mime(bytes: &[u8]) -> &'static str {
    match bytes {
        [0x89, b'P', b'N', b'G', ..] => "image/png",
        [0xFF, 0xD8, 0xFF, ..] => "image/jpeg",
        [b'B', b'M', ..] => "image/bmp",
        _ => "image/x-tga",
    }
}

/// Sérialisation des octets en base64 : les documents `.bones` restent
/// autonomes (textures incluses) tout en restant du JSON.
mod base64_bytes {
    use std::sync::Arc;

    use base64::Engine;
    use base64::engine::general_purpose::STANDARD;
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(bytes: &Arc<[u8]>, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&STANDARD.encode(bytes))
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Arc<[u8]>, D::Error> {
        let text = String::deserialize(d)?;
        STANDARD
            .decode(text.as_bytes())
            .map(Arc::from)
            .map_err(serde::de::Error::custom)
    }
}
