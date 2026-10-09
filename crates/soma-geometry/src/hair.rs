//! Editable procedural hair data and deterministic guide generation.
//!
//! Hair is represented independently from the source body's mesh. The generated
//! guide curves retain stable roots and can be regenerated from the same source
//! faces and settings; viewport rendering and scene-object integration belong to
//! higher layers.

use crate::{FaceId, Mesh, VertexId};
use std::f32::consts::TAU;

/// User-facing hair style presets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HairPreset {
    /// Longer strands intended for scalp hair and flowing hairstyles.
    FlowingHair,
    /// Short, dense strands intended for body hair and close-cropped styles.
    ShortHair,
}

/// Settings shared by procedural hair presets.
///
/// The UI should expose `amount` as Density / Amount for Short Hair. Flowing
/// Hair still uses it internally to control the number of guide strands.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HairSettings {
    pub length: f32,
    /// Normalized curl amount in the inclusive range 0..=1.
    pub curl: f32,
    /// RGBA hair color, independent of the source surface material.
    pub color: [f32; 4],
    /// Normalized gravity in the inclusive range 0..=1.
    pub gravity: f32,
    /// Number of guide strands used to describe the hairstyle.
    pub amount: usize,
    /// Fixed seed makes generation repeatable for the same source mesh/settings.
    pub seed: u64,
}

impl HairSettings {
    pub fn flowing_hair() -> Self {
        Self {
            length: 1.0,
            curl: 0.15,
            color: [0.12, 0.07, 0.035, 1.0],
            gravity: 0.5,
            amount: 512,
            seed: 1,
        }
    }

    pub fn short_hair() -> Self {
        Self {
            length: 0.08,
            curl: 0.1,
            color: [0.12, 0.07, 0.035, 1.0],
            gravity: 0.5,
            amount: 2048,
            seed: 1,
        }
    }

    fn validate(self) -> Result<(), HairError> {
        if !self.length.is_finite()
            || self.length <= 0.0
            || !self.curl.is_finite()
            || !(0.0..=1.0).contains(&self.curl)
            || !self.gravity.is_finite()
            || !(0.0..=1.0).contains(&self.gravity)
            || !self.color.iter().all(|channel| channel.is_finite() && (0.0..=1.0).contains(channel))
        {
            return Err(HairError::InvalidSettings);
        }
        if self.amount == 0 {
            return Err(HairError::NoStrands);
        }
        if self.amount > MAX_GUIDE_STRANDS {
            return Err(HairError::TooManyStrands);
        }
        Ok(())
    }
}

/// Safety ceiling for guide generation. Rendering can derive more strands from
/// guides later, subject to its own preview/final quality budgets.
pub const MAX_GUIDE_STRANDS: usize = 50_000;

/// Errors encountered while preparing a procedural hair object.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HairError {
    EmptyFaceSelection,
    FaceNotFound(FaceId),
    InvalidFaceVertex { face: FaceId, vertex: VertexId },
    DegenerateFace(FaceId),
    InvalidSettings,
    NoStrands,
    TooManyStrands,
}

/// One sampled strand guide. Points start at the attached root and end at the tip.
#[derive(Debug, Clone, PartialEq)]
pub struct HairGuide {
    pub root_position: [f32; 3],
    pub root_normal: [f32; 3],
    pub points: Vec<[f32; 3]>,
}

/// Editable hair generated from a set of source surface faces.
///
/// The source mesh is not modified. Store this object separately from body
/// topology so color, density, curl, and length can be changed later.
#[derive(Debug, Clone, PartialEq)]
pub struct HairObject {
    preset: HairPreset,
    source_faces: Vec<FaceId>,
    settings: HairSettings,
    guides: Vec<HairGuide>,
}

impl HairObject {
    /// Generates hair guides over selected faces without mutating the source mesh.
    pub fn generate(
        source: &Mesh,
        selected_faces: &[FaceId],
        preset: HairPreset,
        settings: HairSettings,
    ) -> Result<Self, HairError> {
        settings.validate()?;
        if selected_faces.is_empty() {
            return Err(HairError::EmptyFaceSelection);
        }

        let mut source_faces = selected_faces.to_vec();
        source_faces.sort_unstable();
        source_faces.dedup();

        let mut surfaces = Vec::with_capacity(source_faces.len());
        let mut total_area = 0.0f32;
        for &face_id in &source_faces {
            let face = source.face(face_id).ok_or(HairError::FaceNotFound(face_id))?;
            let mut positions = Vec::with_capacity(face.vertices.len());
            for &vertex in &face.vertices {
                let position = source.vertex_position(vertex).ok_or(
                    HairError::InvalidFaceVertex { face: face_id, vertex },
                )?;
                if !position.iter().all(|component| component.is_finite()) {
                    return Err(HairError::InvalidFaceVertex { face: face_id, vertex });
                }
                positions.push(position);
            }

            let normal = face_normal(&positions).ok_or(HairError::DegenerateFace(face_id))?;
            let mut triangles = Vec::new();
            let origin = positions[0];
            for index in 1..positions.len().saturating_sub(1) {
                let a = positions[index];
                let b = positions[index + 1];
                let area = triangle_area(origin, a, b);
                if area > f32::EPSILON && area.is_finite() {
                    triangles.push((origin, a, b, area));
                }
            }
            let area: f32 = triangles.iter().map(|triangle| triangle.3).sum();
            if !area.is_finite() || area <= f32::EPSILON {
                return Err(HairError::DegenerateFace(face_id));
            }
            total_area += area;
            surfaces.push((face_id, normal, triangles, area));
        }

        if !total_area.is_finite() || total_area <= f32::EPSILON {
            return Err(HairError::DegenerateFace(source_faces[0]));
        }

        let mut rng = DeterministicRng::new(settings.seed);
        let mut guides = Vec::with_capacity(settings.amount);
        for _ in 0..settings.amount {
            let mut choice = rng.next_f32() * total_area;
            let mut selected_surface = surfaces.last().expect("selection is non-empty");
            for surface in &surfaces {
                if choice < surface.3 {
                    selected_surface = surface;
                    break;
                }
                choice -= surface.3;
            }

            let mut triangle_choice = rng.next_f32() * selected_surface.3;
            let mut triangle = selected_surface.2.last().expect("surface has a triangle");
            for candidate in &selected_surface.2 {
                if triangle_choice < candidate.3 {
                    triangle = candidate;
                    break;
                }
                triangle_choice -= candidate.3;
            }

            let root = sample_triangle(
                triangle.0,
                triangle.1,
                triangle.2,
                rng.next_f32(),
                rng.next_f32(),
            );
            let phase = rng.next_f32() * TAU;
            let points = build_guide(root, selected_surface.1, settings, phase);
            guides.push(HairGuide {
                root_position: root,
                root_normal: selected_surface.1,
                points,
            });
        }

        Ok(Self {
            preset,
            source_faces,
            settings,
            guides,
        })
    }

    pub fn preset(&self) -> HairPreset {
        self.preset
    }

    pub fn source_faces(&self) -> &[FaceId] {
        &self.source_faces
    }

    pub fn settings(&self) -> HairSettings {
        self.settings
    }

    pub fn guides(&self) -> &[HairGuide] {
        &self.guides
    }

    /// Regenerates a candidate first, so invalid settings never replace valid guides.
    pub fn restyle(
        &mut self,
        source: &Mesh,
        settings: HairSettings,
    ) -> Result<(), HairError> {
        let candidate = Self::generate(source, &self.source_faces, self.preset, settings)?;
        *self = candidate;
        Ok(())
    }
}

fn build_guide(
    root: [f32; 3],
    normal: [f32; 3],
    settings: HairSettings,
    phase: f32,
) -> Vec<[f32; 3]> {
    const SEGMENTS: usize = 5;
    let side = perpendicular(normal);
    let mut points = Vec::with_capacity(SEGMENTS + 1);
    for index in 0..=SEGMENTS {
        let t = index as f32 / SEGMENTS as f32;
        let distance = settings.length * t;
        let curl = settings.curl * settings.length * 0.18 * (t * TAU * 1.5 + phase).sin() * t;
        let gravity = settings.gravity * settings.length * t * t * 0.75;
        points.push([
            root[0] + normal[0] * distance + side[0] * curl,
            root[1] + normal[1] * distance + side[1] * curl - gravity,
            root[2] + normal[2] * distance + side[2] * curl,
        ]);
    }
    points
}

fn face_normal(points: &[[f32; 3]]) -> Option<[f32; 3]> {
    if points.len() < 3 {
        return None;
    }
    let mut normal = [0.0; 3];
    for index in 0..points.len() {
        let current = points[index];
        let next = points[(index + 1) % points.len()];
        normal[0] += (current[1] - next[1]) * (current[2] + next[2]);
        normal[1] += (current[2] - next[2]) * (current[0] + next[0]);
        normal[2] += (current[0] - next[0]) * (current[1] + next[1]);
    }
    let length = dot(normal, normal).sqrt();
    if !length.is_finite() || length <= f32::EPSILON {
        return None;
    }
    Some([normal[0] / length, normal[1] / length, normal[2] / length])
}

fn triangle_area(a: [f32; 3], b: [f32; 3], c: [f32; 3]) -> f32 {
    let ab = subtract(b, a);
    let ac = subtract(c, a);
    let cross = [
        ab[1] * ac[2] - ab[2] * ac[1],
        ab[2] * ac[0] - ab[0] * ac[2],
        ab[0] * ac[1] - ab[1] * ac[0],
    ];
    dot(cross, cross).sqrt() * 0.5
}

fn sample_triangle(
    a: [f32; 3],
    b: [f32; 3],
    c: [f32; 3],
    random_a: f32,
    random_b: f32,
) -> [f32; 3] {
    let root_a = random_a.sqrt();
    let weight_a = 1.0 - root_a;
    let weight_b = root_a * (1.0 - random_b);
    let weight_c = root_a * random_b;
    [
        a[0] * weight_a + b[0] * weight_b + c[0] * weight_c,
        a[1] * weight_a + b[1] * weight_b + c[1] * weight_c,
        a[2] * weight_a + b[2] * weight_b + c[2] * weight_c,
    ]
}

fn perpendicular(normal: [f32; 3]) -> [f32; 3] {
    let reference = if normal[1].abs() < 0.9 {
        [0.0, 1.0, 0.0]
    } else {
        [1.0, 0.0, 0.0]
    };
    let cross = [
        normal[1] * reference[2] - normal[2] * reference[1],
        normal[2] * reference[0] - normal[0] * reference[2],
        normal[0] * reference[1] - normal[1] * reference[0],
    ];
    let length = dot(cross, cross).sqrt().max(f32::EPSILON);
    [cross[0] / length, cross[1] / length, cross[2] / length]
}

fn subtract(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

struct DeterministicRng {
    state: u64,
}

impl DeterministicRng {
    fn new(seed: u64) -> Self {
        Self {
            state: seed ^ 0x9E37_79B9_7F4A_7C15,
        }
    }

    fn next_u32(&mut self) -> u32 {
        // xorshift64*; a fixed seed yields a repeatable sequence.
        self.state ^= self.state >> 12;
        self.state ^= self.state << 25;
        self.state ^= self.state >> 27;
        (self.state.wrapping_mul(0x2545_F491_4F6C_DD1D) >> 32) as u32
    }

    fn next_f32(&mut self) -> f32 {
        self.next_u32() as f32 / u32::MAX as f32
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Uv;

    fn quad() -> (Mesh, FaceId) {
        let mut mesh = Mesh::new();
        let a = mesh.add_vertex([-1.0, 0.0, -1.0]);
        let b = mesh.add_vertex([1.0, 0.0, -1.0]);
        let c = mesh.add_vertex([1.0, 0.0, 1.0]);
        let d = mesh.add_vertex([-1.0, 0.0, 1.0]);
        let face = mesh.add_face(&[a, b, c, d]).unwrap();
        for (corner, uv) in [
            Uv::new(0.0, 0.0),
            Uv::new(1.0, 0.0),
            Uv::new(1.0, 1.0),
            Uv::new(0.0, 1.0),
        ]
        .into_iter()
        .enumerate()
        {
            mesh.set_uv("UVMap", face, corner, uv).unwrap();
        }
        (mesh, face)
    }

    #[test]
    fn generates_editable_guides_without_changing_source_mesh() {
        let (mesh, face) = quad();
        let before = mesh.vertex_count();
        let hair = HairObject::generate(
            &mesh,
            &[face],
            HairPreset::ShortHair,
            HairSettings::short_hair(),
        )
        .unwrap();

        assert_eq!(hair.guides().len(), 2048);
        assert_eq!(hair.source_faces(), &[face]);
        assert_eq!(mesh.vertex_count(), before);
        assert!(hair.guides().iter().all(|guide| {
            guide.points.len() == 6 && guide.points[0] == guide.root_position
        }));
        assert!(hair.guides().iter().all(|guide| {
            guide.root_position[0].abs() <= 1.0 && guide.root_position[2].abs() <= 1.0
        }));
    }

    #[test]
    fn generation_is_deterministic_for_the_same_seed() {
        let (mesh, face) = quad();
        let first = HairObject::generate(
            &mesh,
            &[face],
            HairPreset::FlowingHair,
            HairSettings::flowing_hair(),
        )
        .unwrap();
        let second = HairObject::generate(
            &mesh,
            &[face],
            HairPreset::FlowingHair,
            HairSettings::flowing_hair(),
        )
        .unwrap();
        assert_eq!(first, second);
    }

    #[test]
    fn invalid_restyle_keeps_previous_hair() {
        let (mesh, face) = quad();
        let mut hair = HairObject::generate(
            &mesh,
            &[face],
            HairPreset::ShortHair,
            HairSettings::short_hair(),
        )
        .unwrap();
        let before = hair.clone();
        let invalid = HairSettings {
            length: f32::NAN,
            ..HairSettings::short_hair()
        };
        assert_eq!(hair.restyle(&mesh, invalid), Err(HairError::InvalidSettings));
        assert_eq!(hair, before);
    }

    #[test]
    fn invalid_faces_and_excessive_density_are_rejected() {
        let (mesh, face) = quad();
        assert_eq!(
            HairObject::generate(
                &mesh,
                &[FaceId(999)],
                HairPreset::ShortHair,
                HairSettings::short_hair(),
            ),
            Err(HairError::FaceNotFound(FaceId(999)))
        );
        let excessive = HairSettings {
            amount: MAX_GUIDE_STRANDS + 1,
            ..HairSettings::short_hair()
        };
        assert_eq!(
            HairObject::generate(&mesh, &[face], HairPreset::ShortHair, excessive),
            Err(HairError::TooManyStrands)
        );
    }

    #[test]
    fn invalid_settings_reject_out_of_range_normalized_controls() {
        let (mesh, face) = quad();
        let invalid = HairSettings {
            gravity: 1.1,
            ..HairSettings::short_hair()
        };
        assert_eq!(
            HairObject::generate(&mesh, &[face], HairPreset::ShortHair, invalid),
            Err(HairError::InvalidSettings)
        );
    }
}
