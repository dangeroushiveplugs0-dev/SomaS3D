use std::collections::BTreeSet;

use crate::{EdgeId, FaceId, VertexId};

/// The active component domain for the modeling viewport.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SelectionMode {
    Vertex,
    Edge,
    #[default]
    Face,
}

/// Stable-ID selection sets shared by viewport tools and modeling operations.
///
/// IDs are kept in ordered sets for deterministic iteration and predictable tests.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Selection {
    mode: SelectionMode,
    vertices: BTreeSet<VertexId>,
    edges: BTreeSet<EdgeId>,
    faces: BTreeSet<FaceId>,
}

impl Selection {
    pub fn new(mode: SelectionMode) -> Self {
        Self {
            mode,
            ..Self::default()
        }
    }

    pub fn mode(&self) -> SelectionMode {
        self.mode
    }

    pub fn set_mode(&mut self, mode: SelectionMode) {
        self.mode = mode;
    }

    pub fn clear(&mut self) {
        self.vertices.clear();
        self.edges.clear();
        self.faces.clear();
    }

    pub fn is_empty(&self) -> bool {
        self.vertices.is_empty() && self.edges.is_empty() && self.faces.is_empty()
    }

    pub fn vertices(&self) -> impl Iterator<Item = VertexId> + '_ {
        self.vertices.iter().copied()
    }

    pub fn edges(&self) -> impl Iterator<Item = EdgeId> + '_ {
        self.edges.iter().copied()
    }

    pub fn faces(&self) -> impl Iterator<Item = FaceId> + '_ {
        self.faces.iter().copied()
    }

    pub fn contains_vertex(&self, id: VertexId) -> bool {
        self.vertices.contains(&id)
    }

    pub fn contains_edge(&self, id: EdgeId) -> bool {
        self.edges.contains(&id)
    }

    pub fn contains_face(&self, id: FaceId) -> bool {
        self.faces.contains(&id)
    }

    pub fn select_vertex(&mut self, id: VertexId) {
        self.vertices.insert(id);
    }

    pub fn select_edge(&mut self, id: EdgeId) {
        self.edges.insert(id);
    }

    pub fn select_face(&mut self, id: FaceId) {
        self.faces.insert(id);
    }

    pub fn deselect_vertex(&mut self, id: VertexId) {
        self.vertices.remove(&id);
    }

    pub fn deselect_edge(&mut self, id: EdgeId) {
        self.edges.remove(&id);
    }

    pub fn deselect_face(&mut self, id: FaceId) {
        self.faces.remove(&id);
    }

    pub fn toggle_vertex(&mut self, id: VertexId) {
        toggle(&mut self.vertices, id);
    }

    pub fn toggle_edge(&mut self, id: EdgeId) {
        toggle(&mut self.edges, id);
    }

    pub fn toggle_face(&mut self, id: FaceId) {
        toggle(&mut self.faces, id);
    }

    /// Removes stale IDs after topology changes without changing the active mode.
    pub fn retain_valid(&mut self, vertex_count: usize, edge_count: usize, face_count: usize) {
        self.vertices.retain(|id| (id.0 as usize) < vertex_count);
        self.edges.retain(|id| (id.0 as usize) < edge_count);
        self.faces.retain(|id| (id.0 as usize) < face_count);
    }
}

fn toggle<T: Ord + Copy>(set: &mut BTreeSet<T>, id: T) {
    if !set.insert(id) {
        set.remove(&id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selection_toggle_is_reversible_and_deterministic() {
        let mut selection = Selection::new(SelectionMode::Vertex);
        selection.toggle_vertex(VertexId(2));
        selection.toggle_vertex(VertexId(1));
        selection.toggle_vertex(VertexId(2));
        assert_eq!(selection.vertices().collect::<Vec<_>>(), vec![VertexId(1)]);
    }

    #[test]
    fn stale_selection_ids_can_be_removed_after_topology_changes() {
        let mut selection = Selection::default();
        selection.select_vertex(VertexId(0));
        selection.select_vertex(VertexId(4));
        selection.select_edge(EdgeId(1));
        selection.select_face(FaceId(3));
        selection.retain_valid(2, 1, 1);
        assert_eq!(selection.vertices().collect::<Vec<_>>(), vec![VertexId(0)]);
        assert!(selection.edges().next().is_none());
        assert!(selection.faces().next().is_none());
    }
}
