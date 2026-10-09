//! Parameterized geometric primitive generation.
//!
//! Primitive generation is separate from interactive UI state so the same
//! validated builders can be used by the viewport, importers, and future presets.

use crate::{Mesh, MeshError, VertexId};
use std::f32::consts::PI;

/// Supported starter primitives. Dimensions are full extents, not half-extents.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PrimitiveKind {
    Cube { size: f32 },
    Plane { width: f32, depth: f32 },
    UvSphere { radius: f32, segments: u32, rings: u32 },
}

/// Invalid parameters for a generated primitive.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PrimitiveError {
    NonFiniteDimension,
    NonPositiveDimension,
    TooFewSegments,
    TooFewRings,
    ExcessiveResolution,
    Mesh(MeshError),
}

impl From<MeshError> for PrimitiveError {
    fn from(value: MeshError) -> Self {
        Self::Mesh(value)
    }
}

/// Generates a new mesh centered at the origin.
///
/// Resolution is capped to protect mobile devices from accidental runaway
/// allocations. UV coordinates are left for the UV authoring stage.
pub fn generate_primitive(kind: PrimitiveKind) -> Result<Mesh, PrimitiveError> {
    match kind {
        PrimitiveKind::Cube { size } => {
            validate_dimension(size)?;
            let h = size * 0.5;
            let mut mesh = Mesh::new();
            let vertices = [
                mesh.add_vertex([-h, -h, -h]),
                mesh.add_vertex([h, -h, -h]),
                mesh.add_vertex([h, h, -h]),
                mesh.add_vertex([-h, h, -h]),
                mesh.add_vertex([-h, -h, h]),
                mesh.add_vertex([h, -h, h]),
                mesh.add_vertex([h, h, h]),
                mesh.add_vertex([-h, h, h]),
            ];
            for face in [
                [vertices[0], vertices[3], vertices[2], vertices[1]],
                [vertices[4], vertices[5], vertices[6], vertices[7]],
                [vertices[0], vertices[1], vertices[5], vertices[4]],
                [vertices[3], vertices[7], vertices[6], vertices[2]],
                [vertices[0], vertices[4], vertices[7], vertices[3]],
                [vertices[1], vertices[2], vertices[6], vertices[5]],
            ] {
                mesh.add_face(&face)?;
            }
            Ok(mesh)
        }
        PrimitiveKind::Plane { width, depth } => {
            validate_dimension(width)?;
            validate_dimension(depth)?;
            let x = width * 0.5;
            let z = depth * 0.5;
            let mut mesh = Mesh::new();
            let a = mesh.add_vertex([-x, 0.0, -z]);
            let b = mesh.add_vertex([x, 0.0, -z]);
            let c = mesh.add_vertex([x, 0.0, z]);
            let d = mesh.add_vertex([-x, 0.0, z]);
            mesh.add_face(&[a, d, c, b])?;
            Ok(mesh)
        }
        PrimitiveKind::UvSphere { radius, segments, rings } => {
            validate_dimension(radius)?;
            if segments < 3 {
                return Err(PrimitiveError::TooFewSegments);
            }
            if rings < 2 {
                return Err(PrimitiveError::TooFewRings);
            }
            if segments > 256 || rings > 256 {
                return Err(PrimitiveError::ExcessiveResolution);
            }

            let mut mesh = Mesh::new();
            let north = mesh.add_vertex([0.0, radius, 0.0]);
            let mut latitude_rings: Vec<Vec<VertexId>> =
                Vec::with_capacity((rings - 1) as usize);
            for ring in 1..rings {
                let theta = PI * ring as f32 / rings as f32;
                let y = radius * theta.cos();
                let radial = radius * theta.sin();
                let mut ids = Vec::with_capacity(segments as usize);
                for segment in 0..segments {
                    let phi = 2.0 * PI * segment as f32 / segments as f32;
                    ids.push(mesh.add_vertex([radial * phi.cos(), y, radial * phi.sin()]));
                }
                latitude_rings.push(ids);
            }
            let south = mesh.add_vertex([0.0, -radius, 0.0]);

            let first = &latitude_rings[0];
            for segment in 0..segments as usize {
                let next = (segment + 1) % segments as usize;
                mesh.add_face(&[north, first[segment], first[next]])?;
            }

            for ring in 0..latitude_rings.len().saturating_sub(1) {
                let upper = &latitude_rings[ring];
                let lower = &latitude_rings[ring + 1];
                for segment in 0..segments as usize {
                    let next = (segment + 1) % segments as usize;
                    mesh.add_face(&[
                        upper[segment],
                        lower[segment],
                        lower[next],
                        upper[next],
                    ])?;
                }
            }

            let last = latitude_rings.last().expect("rings >= 2 creates a latitude ring");
            for segment in 0..segments as usize {
                let next = (segment + 1) % segments as usize;
                mesh.add_face(&[last[segment], south, last[next]])?;
            }
            Ok(mesh)
        }
    }
}

fn validate_dimension(value: f32) -> Result<(), PrimitiveError> {
    if !value.is_finite() {
        return Err(PrimitiveError::NonFiniteDimension);
    }
    if value <= 0.0 {
        return Err(PrimitiveError::NonPositiveDimension);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cube_has_shared_manifold_topology() {
        let mesh = generate_primitive(PrimitiveKind::Cube { size: 2.0 }).unwrap();
        assert_eq!(mesh.vertex_count(), 8);
        assert_eq!(mesh.edge_count(), 12);
        assert_eq!(mesh.face_count(), 6);
        assert!(mesh.validate_topology().is_empty());
    }

    #[test]
    fn plane_is_centered_and_uses_one_quad() {
        let mesh = generate_primitive(PrimitiveKind::Plane { width: 4.0, depth: 2.0 }).unwrap();
        assert_eq!(mesh.vertex_count(), 4);
        assert_eq!(mesh.edge_count(), 4);
        assert_eq!(mesh.face_count(), 1);
        assert_eq!(mesh.vertex_position(VertexId(0)), Some([-2.0, 0.0, -1.0]));
        assert!(mesh.validate_topology().is_empty());
    }

    #[test]
    fn uv_sphere_has_closed_manifold_topology() {
        let mesh = generate_primitive(PrimitiveKind::UvSphere {
            radius: 1.0,
            segments: 12,
            rings: 6,
        })
        .unwrap();
        assert_eq!(mesh.vertex_count(), 2 + 5 * 12);
        assert_eq!(mesh.face_count(), 12 * 6);
        assert!(mesh.validate_topology().is_empty());
        assert_eq!(mesh.edge_count(), 12 * 6 + 12);
    }

    #[test]
    fn invalid_dimensions_and_resolution_are_rejected() {
        assert_eq!(
            generate_primitive(PrimitiveKind::Cube { size: 0.0 }).unwrap_err(),
            PrimitiveError::NonPositiveDimension
        );
        assert_eq!(
            generate_primitive(PrimitiveKind::Plane { width: f32::NAN, depth: 1.0 }).unwrap_err(),
            PrimitiveError::NonFiniteDimension
        );
        assert_eq!(
            generate_primitive(PrimitiveKind::UvSphere { radius: 1.0, segments: 2, rings: 4 })
                .unwrap_err(),
            PrimitiveError::TooFewSegments
        );
        assert_eq!(
            generate_primitive(PrimitiveKind::UvSphere { radius: 1.0, segments: 512, rings: 4 })
                .unwrap_err(),
            PrimitiveError::ExcessiveResolution
        );
    }
}
