use std::collections::{HashMap, VecDeque};

use crate::uv::{Uv, UvCorner, UvError, UvIsland, UvLayer, UvTransform};

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
    /// A seam prevents UV island traversal across this edge.
    pub seam: bool,
    /// Faces incident to this edge. More than two indicates non-manifold use.
    pub faces: Vec<FaceId>,
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
    EdgeNotFound(EdgeId),
    UvLayerNotFound(String),
    Uv(UvError),
}

#[derive(Debug, Default)]
pub struct Mesh {
    vertices: Vec<Vertex>,
    edges: Vec<Edge>,
    faces: Vec<Face>,
    edge_lookup: HashMap<(u32, u32), EdgeId>,
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

        let face_id = FaceId(self.faces.len() as u32);
        self.faces.push(Face {
            vertices: vertices.to_vec(),
        });

        for index in 0..vertices.len() {
            let a = vertices[index];
            let b = vertices[(index + 1) % vertices.len()];
            let key = edge_key(a, b);

            if let Some(&edge_id) = self.edge_lookup.get(&key) {
                self.edges[edge_id.0 as usize].faces.push(face_id);
            } else {
                let edge_id = EdgeId(self.edges.len() as u32);
                self.edges.push(Edge {
                    vertices: [a, b],
                    seam: false,
                    faces: vec![face_id],
                });
                self.edge_lookup.insert(key, edge_id);
            }
        }

        Ok(face_id)
    }

    pub fn vertex_position(&self, id: VertexId) -> Option<[f32; 3]> {
        self.vertices
            .get(id.0 as usize)
            .map(|vertex| vertex.position)
    }

    pub fn face(&self, id: FaceId) -> Option<&Face> {
        self.faces.get(id.0 as usize)
    }

    pub fn edge(&self, id: EdgeId) -> Option<&Edge> {
        self.edges.get(id.0 as usize)
    }

    pub fn edge_between(&self, a: VertexId, b: VertexId) -> Option<EdgeId> {
        self.edge_lookup.get(&edge_key(a, b)).copied()
    }

    pub fn set_edge_seam(&mut self, edge: EdgeId, seam: bool) -> Result<(), MeshError> {
        let edge = self
            .edges
            .get_mut(edge.0 as usize)
            .ok_or(MeshError::EdgeNotFound(edge))?;
        edge.seam = seam;
        Ok(())
    }

    pub fn face_count_corners(&self, id: FaceId) -> Result<usize, MeshError> {
        self.face(id)
            .map(|face| face.vertices.len())
            .ok_or(MeshError::FaceNotFound(id))
    }

    pub fn uv_layer_mut(&mut self, name: &str) -> Option<&mut UvLayer> {
        if !self.uv_layers.contains_key(name) {
            self.uv_layers.insert(name.to_owned(), UvLayer::new(name));
        }
        self.uv_layers.get_mut(name)
    }

    pub fn set_uv(
        &mut self,
        layer_name: &str,
        face: FaceId,
        corner: usize,
        uv: Uv,
    ) -> Result<(), MeshError> {
        let corner_count = self.face_count_corners(face)?;
        if corner >= corner_count {
            return Err(MeshError::Uv(UvError::CornerOutOfRange { face, corner }));
        }

        let layer = self
            .uv_layers
            .entry(layer_name.to_owned())
            .or_insert_with(|| UvLayer::new(layer_name));
        layer.set(face, corner, uv).map_err(MeshError::Uv)
    }

    /// Applies a UV transform to selected face corners atomically.
    ///
    /// All IDs, existing UV values, and transformed coordinates are validated
    /// before any coordinates are changed. Duplicate corner IDs are applied once.
    pub fn transform_uv_corners(
        &mut self,
        layer_name: &str,
        corners: &[UvCorner],
        transform: UvTransform,
    ) -> Result<(), MeshError> {
        let layer = self
            .uv_layers
            .get(layer_name)
            .ok_or_else(|| MeshError::UvLayerNotFound(layer_name.to_owned()))?;

        let mut updates = Vec::with_capacity(corners.len());
        for &corner in corners {
            if updates.iter().any(|(existing, _)| *existing == corner) {
                continue;
            }
            let face_data = self
                .faces
                .get(corner.face.0 as usize)
                .ok_or(MeshError::FaceNotFound(corner.face))?;
            if corner.corner >= face_data.vertices.len() {
                return Err(MeshError::Uv(UvError::CornerOutOfRange {
                    face: corner.face,
                    corner: corner.corner,
                }));
            }
            let current = layer
                .get(corner.face, corner.corner)
                .ok_or(MeshError::Uv(UvError::MissingCoordinate(corner)))?;
            updates.push((corner, transform.apply(current).map_err(MeshError::Uv)?));
        }

        let layer = self
            .uv_layers
            .get_mut(layer_name)
            .expect("layer was checked before validation");
        for (corner, uv) in updates {
            layer
                .set(corner.face, corner.corner, uv)
                .map_err(MeshError::Uv)?;
        }
        Ok(())
    }

    pub fn validate_uv_layer(&self, name: &str) -> Result<(), MeshError> {
        let Some(layer) = self.uv_layers.get(name) else {
            return Ok(());
        };

        for (face_index, face_data) in self.faces.iter().enumerate() {
            let face = FaceId(face_index as u32);
            for corner in 0..face_data.vertices.len() {
                if layer.get(face, corner).is_none() {
                    return Err(MeshError::Uv(UvError::MissingCoordinate(UvCorner {
                        face,
                        corner,
                    })));
                }
            }
        }
        Ok(())
    }

    pub fn uv_layer(&self, name: &str) -> Option<&UvLayer> {
        self.uv_layers.get(name)
    }

    /// Returns UV islands using real face adjacency, explicit seams, and UV
    /// discontinuities. Non-manifold edges are treated as island boundaries.
    pub fn uv_islands(&self, layer_name: &str) -> Result<Vec<UvIsland>, MeshError> {
        let layer = self
            .uv_layers
            .get(layer_name)
            .ok_or_else(|| MeshError::UvLayerNotFound(layer_name.to_owned()))?;
        self.validate_uv_layer(layer_name)?;

        let mut visited = vec![false; self.faces.len()];
        let mut islands = Vec::new();

        for start_index in 0..self.faces.len() {
            if visited[start_index] {
                continue;
            }

            visited[start_index] = true;
            let mut queue = VecDeque::from([FaceId(start_index as u32)]);
            let mut island_faces = Vec::new();

            while let Some(face_id) = queue.pop_front() {
                island_faces.push(face_id);
                let face = &self.faces[face_id.0 as usize];

                for corner in 0..face.vertices.len() {
                    let a = face.vertices[corner];
                    let b = face.vertices[(corner + 1) % face.vertices.len()];
                    let Some(edge_id) = self.edge_between(a, b) else {
                        continue;
                    };
                    let edge = &self.edges[edge_id.0 as usize];

                    if edge.seam || edge.faces.len() != 2 {
                        continue;
                    }

                    let neighbor = if edge.faces[0] == face_id {
                        edge.faces[1]
                    } else {
                        edge.faces[0]
                    };
                    if visited[neighbor.0 as usize] {
                        continue;
                    }

                    if self.uvs_match_across_edge(layer, face_id, corner, neighbor) {
                        visited[neighbor.0 as usize] = true;
                        queue.push_back(neighbor);
                    }
                }
            }

            let corners = island_faces
                .iter()
                .flat_map(|&face_id| {
                    (0..self.faces[face_id.0 as usize].vertices.len()).map(move |corner| UvCorner {
                        face: face_id,
                        corner,
                    })
                })
                .collect();

            islands.push(UvIsland {
                faces: island_faces,
                corners,
            });
        }

        Ok(islands)
    }

    fn uvs_match_across_edge(
        &self,
        layer: &UvLayer,
        face_a_id: FaceId,
        corner_a: usize,
        face_b_id: FaceId,
    ) -> bool {
        let face_a = &self.faces[face_a_id.0 as usize];
        let face_b = &self.faces[face_b_id.0 as usize];
        let a0 = face_a.vertices[corner_a];
        let a1 = face_a.vertices[(corner_a + 1) % face_a.vertices.len()];

        let Some(corner_b) = (0..face_b.vertices.len()).find(|&index| {
            let b0 = face_b.vertices[index];
            let b1 = face_b.vertices[(index + 1) % face_b.vertices.len()];
            (a0 == b0 && a1 == b1) || (a0 == b1 && a1 == b0)
        }) else {
            return false;
        };

        let a_uv0 = layer.get(face_a_id, corner_a).expect("UV layer validated");
        let a_uv1 = layer
            .get(face_a_id, (corner_a + 1) % face_a.vertices.len())
            .expect("UV layer validated");
        let b_uv0 = layer.get(face_b_id, corner_b).expect("UV layer validated");
        let b_uv1 = layer
            .get(face_b_id, (corner_b + 1) % face_b.vertices.len())
            .expect("UV layer validated");

        let b_uv_for_a0 = if face_b.vertices[corner_b] == a0 {
            b_uv0
        } else {
            b_uv1
        };
        let b_uv_for_a1 = if face_b.vertices[corner_b] == a1 {
            b_uv0
        } else {
            b_uv1
        };

        uv_nearly_equal(a_uv0, b_uv_for_a0) && uv_nearly_equal(a_uv1, b_uv_for_a1)
    }
}

fn edge_key(a: VertexId, b: VertexId) -> (u32, u32) {
    if a.0 <= b.0 {
        (a.0, b.0)
    } else {
        (b.0, a.0)
    }
}

fn uv_nearly_equal(a: Uv, b: Uv) -> bool {
    const EPSILON: f32 = 1.0e-5;
    (a.u - b.u).abs() <= EPSILON && (a.v - b.v).abs() <= EPSILON
}
