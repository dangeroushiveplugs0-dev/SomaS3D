//! Platform-independent geometry primitives for SomaS3D.

mod history;
mod material;
mod material_eval;
mod selection;
mod topology;
mod uv;

pub use history::EditHistory;
pub use material::{Material, MaterialSemantics, PbrMaterial};
pub use material_eval::{evaluate, EvaluatedPbr};
pub use selection::{Selection, SelectionMode};
pub use topology::{
    CornerId, Edge, EdgeId, ExtrusionResult, Face, FaceId, Mesh, MeshError, TopologyIssue,
    TopologyRemap, Transform3D, Vertex, VertexId,
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
    fn transform_vertices_scales_and_rotates_around_pivot_atomically() {
        let mut mesh = Mesh::new();
        let a = mesh.add_vertex([2.0, 0.0, 0.0]);
        let untouched = mesh.add_vertex([9.0, 9.0, 9.0]);
        let transform = Transform3D {
            rotation: [0.0, 0.0, std::f32::consts::FRAC_PI_2],
            scale: [2.0, 1.0, 1.0],
            pivot: [1.0, 0.0, 0.0],
            ..Transform3D::default()
        };

        mesh.transform_vertices(&[a], transform).unwrap();
        let result = mesh.vertex_position(a).unwrap();
        assert!((result[0] - 1.0).abs() < 1e-5);
        assert!((result[1] - 2.0).abs() < 1e-5);
        assert_eq!(mesh.vertex_position(untouched), Some([9.0, 9.0, 9.0]));
    }

    #[test]
    fn transform_vertices_rejects_invalid_selection_without_partial_changes() {
        let mut mesh = Mesh::new();
        let a = mesh.add_vertex([1.0, 0.0, 0.0]);
        let before = mesh.vertex_position(a);
        assert_eq!(
            mesh.transform_vertices(&[a, VertexId(99)], Transform3D::default()),
            Err(MeshError::InvalidVertex(VertexId(99)))
        );
        assert_eq!(mesh.vertex_position(a), before);
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

        let result =
            mesh.set_vertex_positions(&[(a, [5.0, 5.0, 5.0]), (b, [f32::INFINITY, 0.0, 0.0])]);

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

    #[test]
    fn extruding_an_isolated_quad_creates_cap_sides_and_valid_topology() {
        let mut mesh = Mesh::new();
        let a = mesh.add_vertex([0.0, 0.0, 0.0]);
        let b = mesh.add_vertex([1.0, 0.0, 0.0]);
        let c = mesh.add_vertex([1.0, 1.0, 0.0]);
        let d = mesh.add_vertex([0.0, 1.0, 0.0]);
        let source = mesh.add_face(&[a, b, c, d]).unwrap();
        for (corner, uv) in [
            Uv::new(0.0, 0.0),
            Uv::new(1.0, 0.0),
            Uv::new(1.0, 1.0),
            Uv::new(0.0, 1.0),
        ]
        .into_iter()
        .enumerate()
        {
            mesh.set_uv("UVMap", source, corner, uv).unwrap();
        }

        let result = mesh.extrude_face(source, [0.0, 0.0, 2.0]).unwrap();
        assert_eq!(mesh.vertex_count(), 8);
        assert_eq!(mesh.face_count(), 5);
        assert_eq!(mesh.edge_count(), 12);
        assert_eq!(result.vertices.len(), 4);
        assert_eq!(result.side_faces.len(), 4);
        assert_eq!(
            mesh.vertex_position(result.vertices[0]),
            Some([0.0, 0.0, 2.0])
        );
        assert_eq!(
            mesh.uv_layer("UVMap").unwrap().get(result.top_face, 2),
            Some(Uv::new(1.0, 1.0))
        );
        assert_eq!(
            mesh.uv_layer("UVMap").unwrap().get(result.side_faces[0], 2),
            Some(Uv::new(1.0, 2.0))
        );
        assert!(mesh.validate_topology().is_empty());
        mesh.validate_uv_layer("UVMap").unwrap();
    }

    #[test]
    fn extruding_a_face_shared_with_a_neighbor_keeps_manifold_adjacency() {
        let (mut mesh, left, _right, _, _, _) = quad_pair();
        let result = mesh.extrude_face(left, [0.0, 0.0, 1.0]).unwrap();
        let shared_original_edge = mesh.edge_between(VertexId(1), VertexId(2)).unwrap();
        assert_eq!(mesh.edge(shared_original_edge).unwrap().faces.len(), 2);
        assert_eq!(mesh.face_count(), 6);
        assert_eq!(result.side_faces.len(), 4);
        assert_eq!(
            result.topology_remap.faces.get(&FaceId(1)),
            Some(&FaceId(0))
        );
        assert!(mesh.validate_topology().is_empty());
    }

    #[test]
    fn invalid_face_extrusion_does_not_mutate_mesh() {
        let mut mesh = Mesh::new();
        let a = mesh.add_vertex([0.0, 0.0, 0.0]);
        let b = mesh.add_vertex([1.0, 0.0, 0.0]);
        let c = mesh.add_vertex([0.0, 1.0, 0.0]);
        let face = mesh.add_face(&[a, b, c]).unwrap();
        let before_vertices = mesh.vertex_count();
        let before_edges = mesh.edge_count();
        let before_faces = mesh.face_count();

        assert_eq!(
            mesh.extrude_face(face, [f32::INFINITY, 0.0, 0.0]),
            Err(MeshError::NonFinitePosition)
        );
        assert_eq!(
            mesh.extrude_face(FaceId(99), [0.0, 0.0, 1.0]),
            Err(MeshError::FaceNotFound(FaceId(99)))
        );
        assert_eq!(mesh.vertex_count(), before_vertices);
        assert_eq!(mesh.edge_count(), before_edges);
        assert_eq!(mesh.face_count(), before_faces);
        assert!(mesh.validate_topology().is_empty());
    }

    #[test]
    fn deleting_a_face_rebuilds_adjacency_and_remaps_uvs() {
        let mut mesh = Mesh::new();
        let a = mesh.add_vertex([0.0, 0.0, 0.0]);
        let b = mesh.add_vertex([1.0, 0.0, 0.0]);
        let c = mesh.add_vertex([1.0, 1.0, 0.0]);
        let d = mesh.add_vertex([0.0, 1.0, 0.0]);
        let e = mesh.add_vertex([2.0, 0.0, 0.0]);
        let f = mesh.add_vertex([2.0, 1.0, 0.0]);
        let first = mesh.add_face(&[a, b, c, d]).unwrap();
        let second = mesh.add_face(&[b, e, f, c]).unwrap();

        for corner in 0..4 {
            mesh.set_uv("UVMap", first, corner, Uv::new(corner as f32, 0.0))
                .unwrap();
            mesh.set_uv("UVMap", second, corner, Uv::new(10.0 + corner as f32, 1.0))
                .unwrap();
        }
        let shared = mesh.edge_between(b, c).unwrap();
        mesh.set_edge_seam(shared, true).unwrap();

        let remap = mesh.remove_face(first).unwrap();

        assert_eq!(mesh.face_count(), 1);
        assert_eq!(mesh.edge_count(), 4);
        assert_eq!(remap.faces.get(&second), Some(&FaceId(0)));
        assert_eq!(
            mesh.uv_layer("UVMap").unwrap().get(FaceId(0), 2),
            Some(Uv::new(12.0, 1.0))
        );
        assert_eq!(
            mesh.edge(mesh.edge_between(b, c).unwrap()).unwrap().seam,
            true
        );
        assert!(mesh.validate_topology().is_empty());
        mesh.validate_uv_layer("UVMap").unwrap();
    }

    #[test]
    fn deleting_a_missing_face_does_not_mutate_mesh() {
        let mut mesh = Mesh::new();
        let a = mesh.add_vertex([0.0, 0.0, 0.0]);
        let b = mesh.add_vertex([1.0, 0.0, 0.0]);
        let c = mesh.add_vertex([0.0, 1.0, 0.0]);
        mesh.add_face(&[a, b, c]).unwrap();

        assert_eq!(
            mesh.remove_face(FaceId(99)),
            Err(MeshError::FaceNotFound(FaceId(99)))
        );
        assert_eq!(mesh.face_count(), 1);
        assert_eq!(mesh.edge_count(), 3);
        assert!(mesh.validate_topology().is_empty());
    }
}
