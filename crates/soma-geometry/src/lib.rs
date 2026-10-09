//! Platform-independent geometry primitives for SomaS3D.

mod material;
mod material_eval;
mod selection;
mod topology;
mod uv;

pub use material::{Material, MaterialSemantics, PbrMaterial};
pub use material_eval::{evaluate, EvaluatedPbr};
pub use selection::{Selection, SelectionMode};
pub use topology::{
    CornerId, Edge, EdgeId, Face, FaceId, Mesh, MeshError, TopologyIssue, Vertex, VertexId,
};
pub use uv::{Uv, UvCorner, UvError, UvIsland, UvLayer, UvTransform};

#[cfg(test)]
mod tests {
    use super::*;

    fn quad_pair() -> (Mesh, FaceId, FaceId, [VertexId; 4], VertexId, VertexId) {
        let mut mesh = Mesh::new();
        let a = mesh.add_vertex([0.0, 0.0, 0.0]);
        let b = mesh.add_vertex([1.0, 0.0, 0.0]);
        let c = mesh.add_vertex([1.0, 1.0, 0.0]);
        let d = mesh.add_vertex([0.0, 1.0, 0.0]);
        let e = mesh.add_vertex([2.0, 0.0, 0.0]);
        let f = mesh.add_vertex([2.0, 1.0, 0.0]);
        let left = mesh.add_face(&[a, b, c, d]).unwrap();
        let right = mesh.add_face(&[b, e, f, c]).unwrap();
        (mesh, left, right, [a, b, c, d], e, f)
    }

    fn set_continuous_pair_uvs(mesh: &mut Mesh, left: FaceId, right: FaceId) {
        let left_uvs = [
            Uv::new(0.0, 0.0),
            Uv::new(1.0, 0.0),
            Uv::new(1.0, 1.0),
            Uv::new(0.0, 1.0),
        ];
        let right_uvs = [
            Uv::new(1.0, 0.0),
            Uv::new(2.0, 0.0),
            Uv::new(2.0, 1.0),
            Uv::new(1.0, 1.0),
        ];
        for (corner, uv) in left_uvs.into_iter().enumerate() {
            mesh.set_uv("UVMap", left, corner, uv).unwrap();
        }
        for (corner, uv) in right_uvs.into_iter().enumerate() {
            mesh.set_uv("UVMap", right, corner, uv).unwrap();
        }
    }

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

        assert_eq!(
            mesh.uv_layer("UVMap").unwrap().get(face, 2),
            Some(Uv::new(1.0, 1.0))
        );
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
    fn continuous_uvs_form_one_island_across_adjacent_faces() {
        let (mut mesh, left, right, _, _, _) = quad_pair();
        set_continuous_pair_uvs(&mut mesh, left, right);

        let islands = mesh.uv_islands("UVMap").unwrap();
        assert_eq!(islands.len(), 1);
        assert_eq!(islands[0].faces.len(), 2);
        assert_eq!(islands[0].corners.len(), 8);
    }

    #[test]
    fn explicit_seam_splits_uv_islands_even_when_coordinates_match() {
        let (mut mesh, left, right, _, _, _) = quad_pair();
        set_continuous_pair_uvs(&mut mesh, left, right);
        let shared = mesh.edge_between(VertexId(1), VertexId(2)).unwrap();
        mesh.set_edge_seam(shared, true).unwrap();

        assert_eq!(mesh.uv_islands("UVMap").unwrap().len(), 2);
    }

    #[test]
    fn uv_discontinuity_splits_islands_without_explicit_seam() {
        let (mut mesh, left, right, _, _, _) = quad_pair();
        set_continuous_pair_uvs(&mut mesh, left, right);
        mesh.set_uv("UVMap", right, 0, Uv::new(1.25, 0.0)).unwrap();

        assert_eq!(mesh.uv_islands("UVMap").unwrap().len(), 2);
    }

    #[test]
    fn non_finite_uvs_are_rejected() {
        let mut mesh = Mesh::new();
        let a = mesh.add_vertex([0.0, 0.0, 0.0]);
        let b = mesh.add_vertex([1.0, 0.0, 0.0]);
        let c = mesh.add_vertex([0.0, 1.0, 0.0]);
        let face = mesh.add_face(&[a, b, c]).unwrap();

        assert_eq!(
            mesh.set_uv("UVMap", face, 0, Uv::new(f32::NAN, 0.0)),
            Err(MeshError::Uv(UvError::NonFinite))
        );
    }

    #[test]
    fn semantic_material_controls_remain_separate_from_pbr() {
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
    #[test]
    fn selected_uv_corners_can_be_translated_independently() {
        let mut mesh = Mesh::new();
        let a = mesh.add_vertex([0.0, 0.0, 0.0]);
        let b = mesh.add_vertex([1.0, 0.0, 0.0]);
        let c = mesh.add_vertex([0.0, 1.0, 0.0]);
        let face = mesh.add_face(&[a, b, c]).unwrap();
        mesh.set_uv("UVMap", face, 0, Uv::new(0.0, 0.0)).unwrap();
        mesh.set_uv("UVMap", face, 1, Uv::new(1.0, 0.0)).unwrap();
        mesh.set_uv("UVMap", face, 2, Uv::new(0.0, 1.0)).unwrap();

        mesh.transform_uv_corners(
            "UVMap",
            &[UvCorner { face, corner: 1 }],
            UvTransform::Translate {
                delta: [0.25, -0.5],
            },
        )
        .unwrap();

        let layer = mesh.uv_layer("UVMap").unwrap();
        assert_eq!(layer.get(face, 0), Some(Uv::new(0.0, 0.0)));
        assert_eq!(layer.get(face, 1), Some(Uv::new(1.25, -0.5)));
        assert_eq!(layer.get(face, 2), Some(Uv::new(0.0, 1.0)));
    }

    #[test]
    fn invalid_uv_transform_is_atomic() {
        let mut mesh = Mesh::new();
        let a = mesh.add_vertex([0.0, 0.0, 0.0]);
        let b = mesh.add_vertex([1.0, 0.0, 0.0]);
        let c = mesh.add_vertex([0.0, 1.0, 0.0]);
        let face = mesh.add_face(&[a, b, c]).unwrap();
        for (corner, uv) in [Uv::new(0.0, 0.0), Uv::new(1.0, 0.0), Uv::new(0.0, 1.0)]
            .into_iter()
            .enumerate()
        {
            mesh.set_uv("UVMap", face, corner, uv).unwrap();
        }
        let before = mesh.uv_layer("UVMap").unwrap().get(face, 0);
        let result = mesh.transform_uv_corners(
            "UVMap",
            &[UvCorner { face, corner: 0 }, UvCorner { face, corner: 99 }],
            UvTransform::Translate { delta: [1.0, 1.0] },
        );
        assert_eq!(
            result,
            Err(MeshError::Uv(UvError::CornerOutOfRange {
                face,
                corner: 99
            }))
        );
        assert_eq!(mesh.uv_layer("UVMap").unwrap().get(face, 0), before);
    }

    #[test]
    fn topology_validation_accepts_a_consistent_quad() {
        let mut mesh = Mesh::new();
        let a = mesh.add_vertex([0.0, 0.0, 0.0]);
        let b = mesh.add_vertex([1.0, 0.0, 0.0]);
        let c = mesh.add_vertex([1.0, 1.0, 0.0]);
        let d = mesh.add_vertex([0.0, 1.0, 0.0]);
        mesh.add_face(&[a, b, c, d]).unwrap();

        assert!(mesh.validate_topology().is_empty());
    }

    #[test]
    fn topology_validation_reports_non_finite_vertex_positions() {
        let mut mesh = Mesh::new();
        mesh.add_vertex([f32::NAN, 0.0, 0.0]);

        assert_eq!(
            mesh.validate_topology(),
            vec![TopologyIssue::NonFiniteVertexPosition(VertexId(0))]
        );
    }

    #[test]
    fn topology_validation_reports_non_manifold_edges_without_hiding_them() {
        let mut mesh = Mesh::new();
        let a = mesh.add_vertex([0.0, 0.0, 0.0]);
        let b = mesh.add_vertex([1.0, 0.0, 0.0]);
        let c = mesh.add_vertex([0.0, 1.0, 0.0]);
        let d = mesh.add_vertex([0.0, -1.0, 0.0]);
        let e = mesh.add_vertex([0.0, 0.0, 1.0]);
        mesh.add_face(&[a, b, c]).unwrap();
        mesh.add_face(&[b, a, d]).unwrap();
        mesh.add_face(&[a, b, e]).unwrap();

        let shared = mesh.edge_between(a, b).unwrap();
        assert!(mesh
            .validate_topology()
            .contains(&TopologyIssue::NonManifoldEdge {
                edge: shared,
                incident_faces: 3,
            }));
    }
    #[test]
    fn vertex_translation_updates_only_requested_vertices() {
        let mut mesh = Mesh::new();
        let a = mesh.add_vertex([0.0, 0.0, 0.0]);
        let b = mesh.add_vertex([1.0, 0.0, 0.0]);
        mesh.translate_vertices(&[b], [0.0, 2.0, -1.0]).unwrap();

        assert_eq!(mesh.vertex_position(a), Some([0.0, 0.0, 0.0]));
        assert_eq!(mesh.vertex_position(b), Some([1.0, 2.0, -1.0]));
    }

    #[test]
    fn invalid_vertex_update_is_atomic() {
        let mut mesh = Mesh::new();
        let a = mesh.add_vertex([0.0, 0.0, 0.0]);
        let b = mesh.add_vertex([1.0, 0.0, 0.0]);

        let result = mesh.set_vertex_positions(&[
            (a, [5.0, 5.0, 5.0]),
            (b, [f32::INFINITY, 0.0, 0.0]),
        ]);

        assert_eq!(result, Err(MeshError::NonFinitePosition));
        assert_eq!(mesh.vertex_position(a), Some([0.0, 0.0, 0.0]));
        assert_eq!(mesh.vertex_position(b), Some([1.0, 0.0, 0.0]));
    }

    #[test]
    fn duplicate_vertex_updates_are_rejected() {
        let mut mesh = Mesh::new();
        let a = mesh.add_vertex([0.0, 0.0, 0.0]);

        assert_eq!(
            mesh.set_vertex_positions(&[(a, [1.0, 0.0, 0.0]), (a, [2.0, 0.0, 0.0])]),
            Err(MeshError::DuplicateVertexUpdate(a))
        );
        assert_eq!(mesh.vertex_position(a), Some([0.0, 0.0, 0.0]));
    }

}
