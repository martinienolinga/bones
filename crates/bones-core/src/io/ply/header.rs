//! En-tête PLY et lecture des valeurs (ASCII ou binaire, deux boutismes).

use crate::io::{Format, IoError};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Encoding {
    Ascii,
    LittleEndian,
    BigEndian,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Scalar {
    I8,
    U8,
    I16,
    U16,
    I32,
    U32,
    F32,
    F64,
}

impl Scalar {
    fn parse(name: &str) -> Option<Scalar> {
        Some(match name {
            "char" | "int8" => Scalar::I8,
            "uchar" | "uint8" => Scalar::U8,
            "short" | "int16" => Scalar::I16,
            "ushort" | "uint16" => Scalar::U16,
            "int" | "int32" => Scalar::I32,
            "uint" | "uint32" => Scalar::U32,
            "float" | "float32" => Scalar::F32,
            "double" | "float64" => Scalar::F64,
            _ => return None,
        })
    }

    fn size(self) -> usize {
        match self {
            Scalar::I8 | Scalar::U8 => 1,
            Scalar::I16 | Scalar::U16 => 2,
            Scalar::I32 | Scalar::U32 | Scalar::F32 => 4,
            Scalar::F64 => 8,
        }
    }
}

#[derive(Clone, Debug)]
pub(super) enum Property {
    Scalar(String, Scalar),
    List(String, Scalar, Scalar),
}

#[derive(Clone, Debug)]
pub(super) struct Element {
    pub name: String,
    pub count: usize,
    pub properties: Vec<Property>,
}

pub(super) struct Header {
    pub encoding: Encoding,
    pub elements: Vec<Element>,
    /// Position du premier octet des données.
    pub data_start: usize,
}

pub(super) fn err(message: impl std::fmt::Display) -> IoError {
    IoError::parse(Format::Ply, message)
}

pub(super) fn parse_header(bytes: &[u8]) -> Result<Header, IoError> {
    let end = bytes
        .windows(10)
        .position(|w| w == b"end_header")
        .ok_or_else(|| err("incomplete header"))?;
    let mut data_start = end + 10;
    while data_start < bytes.len() && bytes[data_start] != b'\n' {
        data_start += 1;
    }
    data_start += 1;
    let text = std::str::from_utf8(&bytes[..end]).map_err(|_| err("non-text header"))?;
    let mut lines = text.lines();
    if lines.next().map(str::trim) != Some("ply") {
        return Err(err("missing \"ply\" signature"));
    }
    let mut encoding = None;
    let mut elements: Vec<Element> = Vec::new();
    for line in lines {
        let t: Vec<&str> = line.split_whitespace().collect();
        match t.as_slice() {
            ["format", "ascii", ..] => encoding = Some(Encoding::Ascii),
            ["format", "binary_little_endian", ..] => encoding = Some(Encoding::LittleEndian),
            ["format", "binary_big_endian", ..] => encoding = Some(Encoding::BigEndian),
            ["element", name, count] => elements.push(Element {
                name: (*name).to_owned(),
                count: count.parse().map_err(|_| err("invalid element count"))?,
                properties: Vec::new(),
            }),
            ["property", "list", count, item, name] => {
                let (Some(c), Some(i)) = (Scalar::parse(count), Scalar::parse(item)) else {
                    return Err(err(format!("unknown type: {line}")));
                };
                let e = elements
                    .last_mut()
                    .ok_or_else(|| err("property outside an element"))?;
                e.properties.push(Property::List((*name).to_owned(), c, i));
            }
            ["property", ty, name] => {
                let ty = Scalar::parse(ty).ok_or_else(|| err(format!("unknown type: {line}")))?;
                let e = elements
                    .last_mut()
                    .ok_or_else(|| err("property outside an element"))?;
                e.properties.push(Property::Scalar((*name).to_owned(), ty));
            }
            _ => {}
        }
    }
    Ok(Header {
        encoding: encoding.ok_or_else(|| err("format not specified"))?,
        elements,
        data_start,
    })
}

/// Lecteur séquentiel des valeurs.
pub(super) struct Values<'a> {
    encoding: Encoding,
    data: &'a [u8],
    pos: usize,
    tokens: std::str::SplitAsciiWhitespace<'a>,
}

impl<'a> Values<'a> {
    pub fn new(header: &Header, bytes: &'a [u8]) -> Self {
        let data = &bytes[header.data_start.min(bytes.len())..];
        let text = if header.encoding == Encoding::Ascii {
            std::str::from_utf8(data).unwrap_or("")
        } else {
            ""
        };
        Self {
            encoding: header.encoding,
            data,
            pos: 0,
            tokens: text.split_ascii_whitespace(),
        }
    }

    pub fn read(&mut self, ty: Scalar) -> Result<f64, IoError> {
        if self.encoding == Encoding::Ascii {
            let token = self.tokens.next().ok_or_else(|| err("truncated data"))?;
            return token
                .parse::<f64>()
                .map_err(|_| err(format!("invalid value: {token}")));
        }
        let size = ty.size();
        let raw = self
            .data
            .get(self.pos..self.pos + size)
            .ok_or_else(|| err("truncated data"))?;
        self.pos += size;
        let mut buf = [0u8; 8];
        buf[..size].copy_from_slice(raw);
        if self.encoding == Encoding::BigEndian {
            buf[..size].reverse();
        }
        Ok(match ty {
            Scalar::I8 => buf[0] as i8 as f64,
            Scalar::U8 => buf[0] as f64,
            Scalar::I16 => i16::from_le_bytes([buf[0], buf[1]]) as f64,
            Scalar::U16 => u16::from_le_bytes([buf[0], buf[1]]) as f64,
            Scalar::I32 => i32::from_le_bytes(buf[..4].try_into().unwrap()) as f64,
            Scalar::U32 => u32::from_le_bytes(buf[..4].try_into().unwrap()) as f64,
            Scalar::F32 => f32::from_le_bytes(buf[..4].try_into().unwrap()) as f64,
            Scalar::F64 => f64::from_le_bytes(buf),
        })
    }
}
