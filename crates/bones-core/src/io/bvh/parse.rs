//! Lecture du texte BVH : hiérarchie puis mouvement.

use glam::{EulerRot, Mat4, Quat, Vec3};

use crate::io::{Format, IoError};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Channel {
    PosX,
    PosY,
    PosZ,
    RotX,
    RotY,
    RotZ,
}

impl Channel {
    fn parse(s: &str) -> Option<Channel> {
        Some(match s.to_ascii_lowercase().as_str() {
            "xposition" => Channel::PosX,
            "yposition" => Channel::PosY,
            "zposition" => Channel::PosZ,
            "xrotation" => Channel::RotX,
            "yrotation" => Channel::RotY,
            "zrotation" => Channel::RotZ,
            _ => return None,
        })
    }
}

pub(super) struct BvhJoint {
    pub name: String,
    pub parent: Option<usize>,
    pub offset: Vec3,
    pub channels: Vec<Channel>,
    /// Décalage du bout de chaîne (« End Site »).
    pub end_site: Option<Vec3>,
    /// Indice du premier canal de l'articulation dans une image.
    pub first_channel: usize,
}

pub(super) struct Bvh {
    pub joints: Vec<BvhJoint>,
    pub frame_time: f32,
    pub channel_count: usize,
    /// Valeurs de toutes les images, à la suite.
    pub values: Vec<f32>,
}

fn err(message: impl std::fmt::Display) -> IoError {
    IoError::parse(Format::Bvh, message)
}

impl Bvh {
    pub fn frame_count(&self) -> usize {
        self.values.len() / self.channel_count.max(1)
    }

    /// Transformation locale d'une articulation à une image (`None` : repos).
    pub fn local(&self, joint: usize, frame: Option<usize>, scale: f32) -> Mat4 {
        let j = &self.joints[joint];
        let Some(f) = frame else {
            return Mat4::from_translation(j.offset * scale);
        };
        let base = f * self.channel_count + j.first_channel;
        let mut position = None::<Vec3>;
        let mut rotation = Quat::IDENTITY;
        for (k, c) in j.channels.iter().enumerate() {
            let v = self.values[base + k];
            let p = position.get_or_insert(Vec3::ZERO);
            match c {
                Channel::PosX => p.x = v,
                Channel::PosY => p.y = v,
                Channel::PosZ => p.z = v,
                Channel::RotX => rotation *= Quat::from_rotation_x(v.to_radians()),
                Channel::RotY => rotation *= Quat::from_rotation_y(v.to_radians()),
                Channel::RotZ => rotation *= Quat::from_rotation_z(v.to_radians()),
            }
        }
        let has_position = j
            .channels
            .iter()
            .any(|c| matches!(c, Channel::PosX | Channel::PosY | Channel::PosZ));
        let translation = if has_position {
            position.unwrap_or(Vec3::ZERO)
        } else {
            j.offset
        };
        Mat4::from_rotation_translation(rotation, translation * scale)
    }
}

/// Angles d'Euler ZXY (degrés) d'une rotation, dans l'ordre d'écriture.
pub(super) fn euler_zxy_degrees(q: Quat) -> [f32; 3] {
    let (z, x, y) = q.to_euler(EulerRot::ZXY);
    [z.to_degrees(), x.to_degrees(), y.to_degrees()]
}

pub(super) fn parse(text: &str) -> Result<Bvh, IoError> {
    let mut tokens = text.split_whitespace();
    let mut next = || tokens.next().ok_or_else(|| err("truncated file"));
    let number = |t: &str| t.parse::<f32>().map_err(|_| err(format!("invalid number: {t}")));

    if !next()?.eq_ignore_ascii_case("HIERARCHY") {
        return Err(err("HIERARCHY keyword expected"));
    }
    let mut joints: Vec<BvhJoint> = Vec::new();
    let mut stack: Vec<Option<usize>> = Vec::new();
    let mut channel_count = 0;
    let mut pending: Option<(String, Option<usize>)> = None;
    loop {
        let t = next()?;
        match t.to_ascii_uppercase().as_str() {
            "ROOT" | "JOINT" => pending = Some((next()?.to_owned(), stack.last().copied().flatten())),
            "END" => {
                next()?; // « Site »
                pending = None;
                stack.push(None);
                if next()? != "{" {
                    return Err(err("\"{\" expected after End Site"));
                }
                if !next()?.eq_ignore_ascii_case("OFFSET") {
                    return Err(err("OFFSET expected in End Site"));
                }
                let o = Vec3::new(number(next()?)?, number(next()?)?, number(next()?)?);
                if let Some(Some(j)) = stack.iter().rev().nth(1) {
                    joints[*j].end_site = Some(o);
                }
            }
            "{" => {
                if let Some((name, parent)) = pending.take() {
                    joints.push(BvhJoint {
                        name,
                        parent,
                        offset: Vec3::ZERO,
                        channels: Vec::new(),
                        end_site: None,
                        first_channel: channel_count,
                    });
                    stack.push(Some(joints.len() - 1));
                }
            }
            "}" => {
                stack.pop();
            }
            "OFFSET" => {
                let o = Vec3::new(number(next()?)?, number(next()?)?, number(next()?)?);
                if let Some(j) = joints.last_mut() {
                    j.offset = o;
                }
            }
            "CHANNELS" => {
                let n: usize = next()?.parse().map_err(|_| err("invalid channel count"))?;
                let j = joints.last_mut().ok_or_else(|| err("CHANNELS outside a joint"))?;
                for _ in 0..n {
                    let c = next()?;
                    j.channels
                        .push(Channel::parse(c).ok_or_else(|| err(format!("unknown channel: {c}")))?);
                }
                channel_count += n;
            }
            "MOTION" => break,
            other => return Err(err(format!("unexpected word: {other}"))),
        }
    }

    // Mouvement : « Frames: N » puis « Frame Time: t ».
    let mut frames = 0usize;
    let frame_time = loop {
        let t = next()?;
        if t.eq_ignore_ascii_case("Frames:") {
            frames = next()?.parse().map_err(|_| err("invalid frame count"))?;
        } else if t.eq_ignore_ascii_case("Time:") {
            break number(next()?)?;
        }
    };
    let total = frames * channel_count;
    let mut values = Vec::with_capacity(total);
    for _ in 0..total {
        values.push(number(next()?)?);
    }
    if joints.is_empty() {
        return Err(err("no joint"));
    }
    Ok(Bvh {
        joints,
        frame_time: if frame_time > 0.0 { frame_time } else { 1.0 / 30.0 },
        channel_count,
        values,
    })
}
