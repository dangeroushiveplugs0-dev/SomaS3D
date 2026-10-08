//! Platform-independent geometry primitives for SomaS3D.

mod material;
mod topology;
mod uv;

pub use material::{Material, MaterialSemantics, PbrMaterial};
pub use topology::{CornerId, EdgeId, FaceId, Mesh, MeshError, VertexId};
pub use uv::{Uv, UvCorner, UvError, UvIsland, UvLayer};

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

        mesh.set_uv("UVMap", face, 0, Uv::new(0.0, 0.0)).unwrap();
        mesh.set_uv("UVMap", face, 1, Uv::new(1.0, 0.0)).unwrap();
        mesh.set_uv("UVMap", face, 2, Uv::new(1.0, 1.0)).unwrap();
        mesh.set_uv("UVMap", face, 3, Uv::new(0.0, 1.0)).unwrap();

        assert_eq!(mesh.uv_layer("UVMap").unwrap().get(face, 2), Some(Uv::new(1.0, 1.0)));
        assert!(mesh.validate_uv_layer("UVMap").is_ok());
    }

    #[test]
    fn uv_coordinates_can_be_outside_unit_square() {
        let mut mesh = Mesh::new();
        let a = mesh.add_vertex([0.0, 0.0, 0.0]);
        let b = mesh.add_vertex([1.0, 0.0, 0.0]);
        let c = mesh.add_vertex([0.0, 1.0, 0.0]);
        let face = mesh.add_face(&[a, b, c]).unwrap();

        mesh.set_uv("UVMap", face, 0, Uv::new(-1.0, 2.0)).unwrap();
        mesh.set_uv("UVMap", face, 1, Uv::new(0.0, 0.0)).unwrap();
        mesh.set_uv("UVMap", face, 2, Uv::new(1.0, 0.0)).unwrap();
        assert!(mesh.validate_uv_layer("UVMap").is_ok());
    }

    #[test]
    fn semantic_material_controls_remain_separate_from_pbr() {
        let material = Material {
            name: "Skin".into(),
            pbr: PbrMaterial::default(),
            semantics: MaterialSemantics { wetness: 0.8, dryness: 0.0, organicness: 1.0 },
        };
        assert_eq!(material.semantics.wetness, 0.8);
        assert_eq!(material.pbr.roughness, 0.5);
    }
}
