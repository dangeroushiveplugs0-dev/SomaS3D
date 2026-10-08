//! Platform-independent geometry primitives for SomaS3D.
//!
//! This crate owns editable mesh data and authoring-time material data.
//! It deliberately contains no Android, GPU, or UI dependencies.

mod material;
mod topology;
mod uv;

pub use material::{Material, MaterialSemantics, PbrMaterial};
pub use topology::{CornerId, EdgeId, FaceId, Mesh, MeshError, VertexId};
pub use uv::{Uv, UvLayer, UvError};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quad_uvs_are_stored_per_corner() {
        let mut mesh = Mesh::new();
        let v0 = mesh.add_vertex([0.0, 0.0, 0.0]);
        let v1 = mesh.add_vertex([1.0, 0.0, 0.0]);
        let v2 = mesh.add_vertex([1.0, 1.0, 0.0]);
        let v3 = mesh.add_vertex([0.0, 1.0, 0.0]);

        let face = mesh.add_face(&[v0, v1, v2, v3]).unwrap();
        let uv = mesh.uv_layer_mut("UVMap").unwrap();

        uv.set(face, 0, Uv::new(0.0, 0.0)).unwrap();
        uv.set(face, 1, Uv::new(1.0, 0.0)).unwrap();
        uv.set(face, 2, Uv::new(1.0, 1.0)).unwrap();
        uv.set(face, 3, Uv::new(0.0, 1.0)).unwrap();

        assert_eq!(uv.get(face, 0).unwrap(), Uv::new(0.0, 0.0));
        assert_eq!(uv.get(face, 2).unwrap(), Uv::new(1.0, 1.0));
    }

    #[test]
    fn semantic_wetness_is_not_a_pbr_channel() {
        let material = Material {
            name: "Skin".into(),
            pbr: PbrMaterial::default(),
            semantics: MaterialSemantics {
                wetness: 0.8,
                dryness: 0.0,
                organicness: 1.0,
            },
        };

        assert_eq!(material.semantics.wetness, 0.8);
        assert_eq!(material.pbr.roughness, 0.5);
    }
}
