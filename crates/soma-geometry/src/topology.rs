use std::collections::HashMap;

use crate::uv::{Uv, UvError, UvLayer};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct VertexId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EdgeId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FaceId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CornerId {
    pub face: FaceId,
    pub index: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Vertex {
    pub position: [f32; 3],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Edge {
    pub vertices: [VertexId; 2],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Face {
    pub vertices: Vec<VertexId>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MeshError {
    EmptyFace,
    FaceTooSmall,
    InvalidVertex(VertexId),
    DuplicateVertexInFace(VertexId),
    FaceNotFound(FaceId),
    Uv(UvError),
}

#[derive(Debug, Default)]
pub struct Mesh {
    vertices: Vec<Vertex>,
    edges: Vec<Edge>,
    faces: Vec<Face>,
    uv_layers: HashMap<String, UvLayer>,
}

impl Mesh {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn vertex_count(&self) -> usize {
        self.vertices.len()
    }

    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }

    pub fn face_count(&self) -> usize {
        self.faces.len()
    }

    pub fn add_vertex(&mut self, position: [f32; 3]) -> VertexId {
        let id = VertexId(self.vertices.len() as u32);
        self.vertices.push(Vertex { position });
        id
    }

    pub fn add_face(&mut self, vertices: &[VertexId]) -> Result<FaceId, MeshError> {
        if vertices.is_empty() {
            return Err(MeshError::EmptyFace);
        }
        if vertices.len() < 3 {
            return Err(MeshError::FaceTooSmall);
        }

        for (index, &vertex) in vertices.iter().enumerate() {
            if vertex.0 as usize >= self.vertices.len() {
                return Err(MeshError::InvalidVertex(vertex));
            }
            if vertices[..index].contains(&vertex) {
                return Err(MeshError::DuplicateVertexInFace(vertex));
            }
        }

        let id = FaceId(self.faces.len() as u32);
        self.faces.push(Face {
            vertices: vertices.to_vec(),
        });

        for pair in vertices.windows(2).chain(std::iter::once(&vertices[vertices.len() - 1..])) {
            let [a, b] = [pair[0], pair[1]];
            if !self.has_edge(a, b) {
                self.edges.push(Edge { vertices: [a, b] });
            }
        }

        Ok(id)
    }

    fn has_edge(&self, a: VertexId, b: VertexId) -> bool {
        self.edges.iter().any(|edge| {
            (edge.vertices[0] == a && edge.vertices[1] == b)
                || (edge.vertices[0] == b && edge.vertices[1] == a)
        })
    }

    pub fn face(&self, id: FaceId) -> Option<&Face> {
        self.faces.get(id.0 as usize)
    }

    pub fn uv_layer_mut(&mut self, name: &str) -> Option<&mut UvLayer> {
        if !self.uv_layers.contains_key(name) {
            self.uv_layers
                .insert(name.to_owned(), UvLayer::new(name));
        }
        self.uv_layers.get_mut(name)
    }

    pub fn set_uv(
        &mut self,
        layer: &str,
        face: FaceId,
        corner: usize,
        uv: Uv,
    ) -> Result<(), MeshError> {
        let face_data = self.face(face).ok_or(MeshError::FaceNotFound(face))?;
        if corner >= face_data.vertices.len() {
            return Err(MeshError::Uv(UvError::CornerOutOfRange { face, corner }));
        }

        let layer = self
            .uv_layers
            .entry(layer.to_owned())
            .or_insert_with(|| UvLayer::new(layer));
        layer.set(face, corner, uv).map_err(MeshError::Uv)
    }
}
