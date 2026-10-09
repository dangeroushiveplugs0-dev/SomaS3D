use std::collections::{HashSet, VecDeque};

use crate::{
    EdgeId, EditHistory, FaceId, Mesh, MeshError, Selection, SelectionMode, Transform3D, VertexId,
};

/// Mesh and component selection form one undoable editor state.
#[derive(Debug, Clone)]
pub struct EditorState {
    mesh: Mesh,
    selection: Selection,
}

impl EditorState {
    pub fn new(mesh: Mesh) -> Self {
        Self {
            mesh,
            selection: Selection::default(),
        }
    }

    pub fn mesh(&self) -> &Mesh {
        &self.mesh
    }

    pub fn selection(&self) -> &Selection {
        &self.selection
    }
}

/// Modeling commands coordinated with bounded undo/redo history.
#[derive(Debug, Clone)]
pub struct ModelingEditor {
    history: EditHistory<EditorState>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EditorError {
    Mesh(MeshError),
    EmptyVertexSelection,
    EmptyFaceSelection,
    ExtrusionRequiresSingleFace,
    EdgeNotFound(EdgeId),
}

impl From<MeshError> for EditorError {
    fn from(error: MeshError) -> Self {
        Self::Mesh(error)
    }
}

impl ModelingEditor {
    pub fn new(mesh: Mesh, undo_limit: usize) -> Self {
        Self {
            history: EditHistory::new(EditorState::new(mesh), undo_limit),
        }
    }

    pub fn state(&self) -> &EditorState {
        self.history.current()
    }

    pub fn can_undo(&self) -> bool {
        self.history.can_undo()
    }

    pub fn can_redo(&self) -> bool {
        self.history.can_redo()
    }

    pub fn undo(&mut self) -> bool {
        self.history.undo()
    }

    pub fn redo(&mut self) -> bool {
        self.history.redo()
    }

    /// Selection changes are tracked so undo can restore the complete editor state.
    pub fn set_selection_mode(&mut self, mode: SelectionMode) {
        let _ = self.history.apply(|state| {
            state.selection.set_mode(mode);
            Ok::<_, EditorError>(())
        });
    }

    pub fn select_vertex(&mut self, vertex: VertexId) {
        let _ = self.history.apply(|state| {
            state.selection.select_vertex(vertex);
            Ok::<_, EditorError>(())
        });
    }

    pub fn select_face(&mut self, face: FaceId) -> Result<(), EditorError> {
        if self.state().mesh.face(face).is_none() {
            return Err(MeshError::FaceNotFound(face).into());
        }
        self.history.apply(|state| {
            state.selection.select_face(face);
            Ok::<_, EditorError>(())
        })
    }

    pub fn clear_selection(&mut self) {
        let _ = self.history.apply(|state| {
            state.selection.clear();
            Ok::<_, EditorError>(())
        });
    }

    /// Selects the manifold-connected face component containing `seed`.
    ///
    /// Traversal stops at boundaries and non-manifold edges, avoiding ambiguous
    /// jumps between more than two incident faces.
    pub fn select_connected_faces(&mut self, seed: FaceId) -> Result<usize, EditorError> {
        if self.state().mesh.face(seed).is_none() {
            return Err(MeshError::FaceNotFound(seed).into());
        }
        let mut selected_count = 0;
        self.history.apply(|state| {
            let mut visited = HashSet::new();
            let mut queue = VecDeque::from([seed]);
            while let Some(face_id) = queue.pop_front() {
                if !visited.insert(face_id) {
                    continue;
                }
                let face = state.mesh.face(face_id).expect("queued face was validated");
                for index in 0..face.vertices.len() {
                    let a = face.vertices[index];
                    let b = face.vertices[(index + 1) % face.vertices.len()];
                    let Some(edge_id) = state.mesh.edge_between(a, b) else {
                        continue;
                    };
                    let Some(edge) = state.mesh.edge(edge_id) else {
                        continue;
                    };
                    if edge.faces.len() == 2 {
                        for &neighbor in &edge.faces {
                            if !visited.contains(&neighbor) {
                                queue.push_back(neighbor);
                            }
                        }
                    }
                }
            }
            for face in &visited {
                state.selection.select_face(*face);
            }
            selected_count = visited.len();
            Ok::<(), EditorError>(())
        })?;
        Ok(selected_count)
    }

    /// Selects the edge component connected to `seed` through shared vertices.
    pub fn select_connected_edges(&mut self, seed: EdgeId) -> Result<usize, EditorError> {
        if self.state().mesh.edge(seed).is_none() {
            return Err(EditorError::EdgeNotFound(seed));
        }
        let mut selected_count = 0;
        self.history.apply(|state| {
            let mut incident_edges = vec![Vec::new(); state.mesh.vertex_count()];
            for index in 0..state.mesh.edge_count() {
                let edge_id = EdgeId(index as u32);
                let Some(edge) = state.mesh.edge(edge_id) else {
                    continue;
                };
                for vertex in edge.vertices {
                    if let Some(edges) = incident_edges.get_mut(vertex.0 as usize) {
                        edges.push(edge_id);
                    }
                }
            }

            let mut visited = HashSet::new();
            let mut queue = VecDeque::from([seed]);
            while let Some(edge_id) = queue.pop_front() {
                if !visited.insert(edge_id) {
                    continue;
                }
                let edge = state.mesh.edge(edge_id).expect("queued edge was validated");
                for vertex in edge.vertices {
                    let Some(adjacent_edges) = incident_edges.get(vertex.0 as usize) else {
                        continue;
                    };
                    for &candidate in adjacent_edges {
                        if !visited.contains(&candidate) {
                            queue.push_back(candidate);
                        }
                    }
                }
            }
            for edge in &visited {
                state.selection.select_edge(*edge);
            }
            selected_count = visited.len();
            Ok::<(), EditorError>(())
        })?;
        Ok(selected_count)
    }

    /// Applies move/rotate/scale to the selected vertices as one undoable edit.
    pub fn transform_selected_vertices(
        &mut self,
        transform: Transform3D,
    ) -> Result<(), EditorError> {
        self.history.apply(|state| {
            let vertices: Vec<_> = state.selection.vertices().collect();
            if vertices.is_empty() {
                return Err(EditorError::EmptyVertexSelection);
            }
            state.mesh.transform_vertices(&vertices, transform)?;
            Ok(())
        })
    }

    /// Deletes selected faces while remapping edge/face selections after each removal.
    pub fn delete_selected_faces(&mut self) -> Result<(), EditorError> {
        self.history.apply(|state| {
            let mut faces: Vec<_> = state.selection.faces().collect();
            if faces.is_empty() {
                return Err(EditorError::EmptyFaceSelection);
            }
            faces.sort_by_key(|face| std::cmp::Reverse(face.0));

            for face in faces {
                let remap = state.mesh.remove_face(face)?;
                state.selection.apply_topology_remap(
                    &remap,
                    state.mesh.edge_count(),
                    state.mesh.face_count(),
                );
                state.selection.retain_valid(
                    state.mesh.vertex_count(),
                    state.mesh.edge_count(),
                    state.mesh.face_count(),
                );
            }
            Ok(())
        })
    }

    /// Extrudes exactly one selected face and selects the new cap.
    pub fn extrude_selected_face(&mut self, offset: [f32; 3]) -> Result<(), EditorError> {
        self.history.apply(|state| {
            let faces: Vec<_> = state.selection.faces().collect();
            if faces.is_empty() {
                return Err(EditorError::EmptyFaceSelection);
            }
            if faces.len() != 1 {
                return Err(EditorError::ExtrusionRequiresSingleFace);
            }

            let result = state.mesh.extrude_face(faces[0], offset)?;
            state.selection.apply_topology_remap(
                &result.topology_remap,
                state.mesh.edge_count(),
                state.mesh.face_count(),
            );
            state.selection.clear();
            state.selection.select_face(result.top_face);
            Ok(())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Uv;

    fn quad() -> (Mesh, FaceId, [VertexId; 4]) {
        let mut mesh = Mesh::new();
        let a = mesh.add_vertex([0.0, 0.0, 0.0]);
        let b = mesh.add_vertex([1.0, 0.0, 0.0]);
        let c = mesh.add_vertex([1.0, 1.0, 0.0]);
        let d = mesh.add_vertex([0.0, 1.0, 0.0]);
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
        (mesh, face, [a, b, c, d])
    }

    #[test]
    fn connected_face_selection_stops_at_non_manifold_edges_and_is_undoable() {
        let mut mesh = Mesh::new();
        let a = mesh.add_vertex([0.0, 0.0, 0.0]);
        let b = mesh.add_vertex([1.0, 0.0, 0.0]);
        let c = mesh.add_vertex([1.0, 1.0, 0.0]);
        let d = mesh.add_vertex([0.0, 1.0, 0.0]);
        let e = mesh.add_vertex([2.0, 0.0, 0.0]);
        let f = mesh.add_vertex([2.0, 1.0, 0.0]);
        let left = mesh.add_face(&[a, b, c, d]).unwrap();
        let right = mesh.add_face(&[b, e, f, c]).unwrap();
        let mut editor = ModelingEditor::new(mesh, 10);

        assert_eq!(editor.select_connected_faces(left).unwrap(), 2);
        assert_eq!(
            editor.state().selection().faces().collect::<Vec<_>>(),
            vec![left, right]
        );
        assert!(editor.undo());
        assert!(editor.state().selection().is_empty());
        assert!(editor.redo());
        assert_eq!(editor.state().selection().faces().count(), 2);
    }

    #[test]
    fn connected_edge_selection_follows_shared_vertices() {
        let (mesh, _, _) = quad();
        let mut editor = ModelingEditor::new(mesh, 10);
        let seed = editor
            .state()
            .mesh
            .edge_between(VertexId(0), VertexId(1))
            .unwrap();
        assert_eq!(editor.select_connected_edges(seed).unwrap(), 4);
        assert_eq!(editor.state().selection().edges().count(), 4);
    }

    #[test]
    fn transform_is_undoable_and_redoable() {
        let (mesh, _, vertices) = quad();
        let mut editor = ModelingEditor::new(mesh, 10);
        for vertex in vertices {
            editor.select_vertex(vertex);
        }
        let before = editor.state().mesh.vertex_position(vertices[0]).unwrap();
        editor
            .transform_selected_vertices(Transform3D {
                translation: [0.0, 0.0, 2.0],
                ..Transform3D::default()
            })
            .unwrap();
        assert_eq!(
            editor.state().mesh.vertex_position(vertices[0]),
            Some([before[0], before[1], 2.0])
        );
        assert!(editor.undo());
        assert_eq!(
            editor.state().mesh.vertex_position(vertices[0]),
            Some(before)
        );
        assert!(editor.redo());
        assert_eq!(
            editor.state().mesh.vertex_position(vertices[0]),
            Some([before[0], before[1], 2.0])
        );
    }

    #[test]
    fn extrusion_undo_restores_mesh_and_selection_together() {
        let (mesh, face, _) = quad();
        let mut editor = ModelingEditor::new(mesh, 10);
        editor.select_face(face).unwrap();
        let original_face_count = editor.state().mesh.face_count();
        editor.extrude_selected_face([0.0, 0.0, 1.0]).unwrap();
        assert_eq!(editor.state().mesh.face_count(), original_face_count + 4);
        assert_eq!(editor.state().selection().faces().count(), 1);
        assert!(editor.undo());
        assert_eq!(editor.state().mesh.face_count(), original_face_count);
        assert!(editor.state().selection().contains_face(face));
        assert!(editor.redo());
        assert_eq!(editor.state().mesh.face_count(), original_face_count + 4);
        assert_eq!(editor.state().selection().faces().count(), 1);
    }

    #[test]
    fn failed_edit_does_not_change_mesh_or_history() {
        let (mesh, _, _) = quad();
        let mut editor = ModelingEditor::new(mesh, 10);
        let face_count = editor.state().mesh.face_count();
        assert_eq!(
            editor.extrude_selected_face([0.0, 0.0, 1.0]),
            Err(EditorError::EmptyFaceSelection)
        );
        assert_eq!(editor.state().mesh.face_count(), face_count);
        assert!(!editor.can_undo());
    }

    #[test]
    fn delete_selected_face_can_be_undone_with_selection_restored() {
        let (mesh, face, _) = quad();
        let mut editor = ModelingEditor::new(mesh, 10);
        editor.select_face(face).unwrap();
        editor.delete_selected_faces().unwrap();
        assert_eq!(editor.state().mesh.face_count(), 0);
        assert!(editor.undo());
        assert_eq!(editor.state().mesh.face_count(), 1);
        assert!(editor.state().selection().contains_face(face));
    }

    #[test]
    fn deleting_multiple_faces_preserves_survivor_uvs_seams_and_valid_topology() {
        let mut mesh = Mesh::new();
        let mut quads = Vec::new();
        let mut rightmost_vertices = [VertexId(0); 4];

        for x in 0..3 {
            let a = mesh.add_vertex([x as f32, 0.0, 0.0]);
            let b = mesh.add_vertex([x as f32 + 1.0, 0.0, 0.0]);
            let c = mesh.add_vertex([x as f32 + 1.0, 1.0, 0.0]);
            let d = mesh.add_vertex([x as f32, 1.0, 0.0]);
            let face = mesh.add_face(&[a, b, c, d]).unwrap();
            for corner in 0..4 {
                mesh.set_uv(
                    "UVMap",
                    face,
                    corner,
                    Uv::new(x as f32 * 10.0 + corner as f32, x as f32 + corner as f32),
                )
                .unwrap();
            }
            quads.push(face);
            if x == 2 {
                rightmost_vertices = [a, b, c, d];
            }
        }

        let preserved_uvs: Vec<_> = (0..4)
            .map(|corner| {
                mesh.uv_layer("UVMap")
                    .unwrap()
                    .get(quads[2], corner)
                    .unwrap()
            })
            .collect();
        let preserved_edge = mesh
            .edge_between(rightmost_vertices[0], rightmost_vertices[1])
            .unwrap();
        mesh.set_edge_seam(preserved_edge, true).unwrap();

        let mut editor = ModelingEditor::new(mesh, 20);
        editor.select_face(quads[0]).unwrap();
        editor.select_face(quads[1]).unwrap();
        editor.delete_selected_faces().unwrap();

        let surviving_face = FaceId(0);
        assert_eq!(editor.state().mesh.face_count(), 1);
        assert_eq!(
            editor.state().mesh.face(surviving_face).unwrap().vertices,
            rightmost_vertices
        );
        for (corner, expected) in preserved_uvs.iter().enumerate() {
            assert_eq!(
                editor
                    .state()
                    .mesh
                    .uv_layer("UVMap")
                    .unwrap()
                    .get(surviving_face, corner),
                Some(*expected)
            );
        }
        let surviving_edge = editor
            .state()
            .mesh
            .edge_between(rightmost_vertices[0], rightmost_vertices[1])
            .unwrap();
        assert!(editor.state().mesh.edge(surviving_edge).unwrap().seam);
        assert!(editor.state().mesh.validate_topology().is_empty());

        assert!(editor.undo());
        assert_eq!(editor.state().mesh.face_count(), 3);
        assert!(editor.state().mesh.validate_topology().is_empty());
        assert!(editor.redo());
        assert_eq!(editor.state().mesh.face_count(), 1);
        assert!(editor.state().mesh.validate_topology().is_empty());
    }

    #[test]
    fn connected_face_selection_does_not_cross_a_three_face_non_manifold_edge() {
        let mut mesh = Mesh::new();
        let a = mesh.add_vertex([0.0, 0.0, 0.0]);
        let b = mesh.add_vertex([1.0, 0.0, 0.0]);
        let upper = mesh.add_vertex([0.5, 1.0, 0.0]);
        let lower_left = mesh.add_vertex([0.5, -1.0, 0.0]);
        let lower_right = mesh.add_vertex([0.5, 0.0, 1.0]);
        let first = mesh.add_face(&[a, b, upper]).unwrap();
        mesh.add_face(&[b, a, lower_left]).unwrap();
        mesh.add_face(&[a, b, lower_right]).unwrap();

        let mut editor = ModelingEditor::new(mesh, 10);
        assert_eq!(editor.select_connected_faces(first).unwrap(), 1);
        assert_eq!(
            editor.state().selection().faces().collect::<Vec<_>>(),
            vec![first]
        );
        assert_eq!(editor.state().mesh.validate_topology().len(), 1);
    }

    #[test]
    fn repeated_extrusion_and_undo_redo_keep_topology_valid() {
        let (mesh, initial_face, _) = quad();
        let mut editor = ModelingEditor::new(mesh, 32);
        editor.select_face(initial_face).unwrap();

        for step in 0..8 {
            editor
                .extrude_selected_face([0.0, 0.0, 0.25 + step as f32 * 0.01])
                .unwrap();
            assert!(editor.state().mesh.validate_topology().is_empty());
            assert_eq!(editor.state().selection().faces().count(), 1);
        }

        for _ in 0..8 {
            assert!(editor.undo());
            assert!(editor.state().mesh.validate_topology().is_empty());
        }
        assert_eq!(editor.state().mesh.face_count(), 1);
        for _ in 0..8 {
            assert!(editor.redo());
            assert!(editor.state().mesh.validate_topology().is_empty());
        }
        assert_eq!(editor.state().mesh.face_count(), 33);
    }

    #[test]
    fn connected_edge_selection_scales_to_dense_grid_meshes_and_is_undoable() {
        let side = 40usize;
        let mut mesh = Mesh::new();
        let mut vertices = vec![vec![VertexId(0); side + 1]; side + 1];

        for (y, row) in vertices.iter_mut().enumerate() {
            for (x, vertex) in row.iter_mut().enumerate() {
                *vertex = mesh.add_vertex([x as f32, y as f32, 0.0]);
            }
        }

        for y in 0..side {
            for x in 0..side {
                mesh.add_face(&[
                    vertices[y][x],
                    vertices[y][x + 1],
                    vertices[y + 1][x + 1],
                    vertices[y + 1][x],
                ])
                .unwrap();
            }
        }

        let expected_edges = 2 * side * (side + 1);
        assert_eq!(mesh.edge_count(), expected_edges);
        assert!(mesh.validate_topology().is_empty());

        let mut editor = ModelingEditor::new(mesh, 4);
        assert_eq!(
            editor.select_connected_edges(EdgeId(0)).unwrap(),
            expected_edges
        );
        assert_eq!(editor.state().selection().edges().count(), expected_edges);

        assert!(editor.undo());
        assert!(editor.state().selection().is_empty());
        assert!(editor.redo());
        assert_eq!(editor.state().selection().edges().count(), expected_edges);
    }
}
