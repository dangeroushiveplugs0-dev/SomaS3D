use std::collections::{HashMap, VecDeque};

use crate::uv::{Uv, UvCorner, UvError, UvIsland, UvLayer, UvTransform};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct VertexId(pub u32);
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EdgeId(pub u32);
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FaceId(pub u32);
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CornerId {
    pub face: FaceId,
    pub index: u32,
}

/// Translation, Euler rotation (radians), and scale around a shared pivot.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Transform3D {
    pub translation: [f32; 3],
    pub rotation: [f32; 3],
    pub scale: [f32; 3],
    pub pivot: [f32; 3],
}

impl Default for Transform3D {
    fn default() -> Self {
        Self {
            translation: [0.0; 3],
            rotation: [0.0; 3],
            scale: [1.0; 3],
            pivot: [0.0; 3],
        }
    }
}

impl Transform3D {
    pub fn apply(self, position: [f32; 3]) -> Result<[f32; 3], MeshError> {
        if !position
            .iter()
            .chain(self.translation.iter())
            .chain(self.rotation.iter())
            .chain(self.scale.iter())
            .chain(self.pivot.iter())
            .all(|value| value.is_finite())
        {
            return Err(MeshError::NonFinitePosition);
        }

        let mut p = [
            (position[0] - self.pivot[0]) * self.scale[0],
            (position[1] - self.pivot[1]) * self.scale[1],
            (position[2] - self.pivot[2]) * self.scale[2],
        ];

        // Apply rotations in X, then Y, then Z order.
        let (sx, cx) = self.rotation[0].sin_cos();
        let (sy, cy) = self.rotation[1].sin_cos();
        let (sz, cz) = self.rotation[2].sin_cos();

        p = [p[0], p[1] * cx - p[2] * sx, p[1] * sx + p[2] * cx];
        p = [p[0] * cy + p[2] * sy, p[1], -p[0] * sy + p[2] * cy];
        p = [p[0] * cz - p[1] * sz, p[0] * sz + p[1] * cz, p[2]];

        let result = [
            p[0] + self.pivot[0] + self.translation[0],
            p[1] + self.pivot[1] + self.translation[1],
            p[2] + self.pivot[2] + self.translation[2],
        ];
        if result.iter().all(|value| value.is_finite()) {
            Ok(result)
        } else {
            Err(MeshError::NonFinitePosition)
        }
    }
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
    NonFinitePosition,
    DuplicateVertexUpdate(VertexId),
    DuplicateVertexInFace(VertexId),
    FaceNotFound(FaceId),
    EdgeNotFound(EdgeId),
    UvLayerNotFound(String),
    Uv(UvError),
}

/// A diagnostic found while validating editable mesh topology.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TopologyIssue {
    NonFiniteVertexPosition(VertexId),
    FaceTooSmall(FaceId),
    InvalidVertexReference {
        face: FaceId,
        vertex: VertexId,
    },
    DuplicateVertexInFace {
        face: FaceId,
        vertex: VertexId,
    },
    MissingEdgeForFace {
        face: FaceId,
        a: VertexId,
        b: VertexId,
    },
    InvalidEdgeEndpoint {
        edge: EdgeId,
        vertex: VertexId,
    },
    DuplicateEdge {
        edge: EdgeId,
        other: EdgeId,
    },
    EdgeLookupMismatch {
        edge: EdgeId,
    },
    InvalidEdgeFaceReference {
        edge: EdgeId,
        face: FaceId,
    },
    EdgeFaceMismatch {
        edge: EdgeId,
        face: FaceId,
    },
    NonManifoldEdge {
        edge: EdgeId,
        incident_faces: usize,
    },
}

/// ID remapping produced by a topology operation that compacts face and edge arrays.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TopologyRemap {
    pub faces: HashMap<FaceId, FaceId>,
    pub edges: HashMap<EdgeId, EdgeId>,
}

/// IDs created by a successful single-face extrusion.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtrusionResult {
    /// Duplicated vertices forming the extruded face, in source-corner order.
    pub vertices: Vec<VertexId>,
    /// The new face at the end of the extrusion.
    pub top_face: FaceId,
    /// Side faces, in source-edge order.
    pub side_faces: Vec<FaceId>,
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

    /// Checks mesh connectivity and reports every detected issue.
    ///
    /// Non-manifold edges are reported as diagnostics rather than silently
    /// repaired; callers can decide whether a particular operation supports them.
    pub fn validate_topology(&self) -> Vec<TopologyIssue> {
        let mut issues = Vec::new();

        for (index, vertex) in self.vertices.iter().enumerate() {
            if !vertex
                .position
                .iter()
                .all(|component| component.is_finite())
            {
                issues.push(TopologyIssue::NonFiniteVertexPosition(VertexId(
                    index as u32,
                )));
            }
        }

        for (index, face) in self.faces.iter().enumerate() {
            let face_id = FaceId(index as u32);
            if face.vertices.len() < 3 {
                issues.push(TopologyIssue::FaceTooSmall(face_id));
            }

            for (corner, &vertex) in face.vertices.iter().enumerate() {
                if vertex.0 as usize >= self.vertices.len() {
                    issues.push(TopologyIssue::InvalidVertexReference {
                        face: face_id,
                        vertex,
                    });
                }
                if face.vertices[..corner].contains(&vertex) {
                    issues.push(TopologyIssue::DuplicateVertexInFace {
                        face: face_id,
                        vertex,
                    });
                }

                if face.vertices.len() >= 2 {
                    let next = face.vertices[(corner + 1) % face.vertices.len()];
                    if (vertex.0 as usize) < self.vertices.len()
                        && (next.0 as usize) < self.vertices.len()
                        && !self.edge_lookup.contains_key(&edge_key(vertex, next))
                    {
                        issues.push(TopologyIssue::MissingEdgeForFace {
                            face: face_id,
                            a: vertex,
                            b: next,
                        });
                    }
                }
            }
        }

        let mut seen_edges: HashMap<(u32, u32), EdgeId> = HashMap::new();
        for (index, edge) in self.edges.iter().enumerate() {
            let edge_id = EdgeId(index as u32);
            let [a, b] = edge.vertices;
            if a == b || a.0 as usize >= self.vertices.len() {
                issues.push(TopologyIssue::InvalidEdgeEndpoint {
                    edge: edge_id,
                    vertex: a,
                });
            }
            if a == b || b.0 as usize >= self.vertices.len() {
                issues.push(TopologyIssue::InvalidEdgeEndpoint {
                    edge: edge_id,
                    vertex: b,
                });
            }

            let key = edge_key(a, b);
            if let Some(other) = seen_edges.insert(key, edge_id) {
                issues.push(TopologyIssue::DuplicateEdge {
                    edge: edge_id,
                    other,
                });
            }
            if self.edge_lookup.get(&key) != Some(&edge_id) {
                issues.push(TopologyIssue::EdgeLookupMismatch { edge: edge_id });
            }

            if edge.faces.len() > 2 {
                issues.push(TopologyIssue::NonManifoldEdge {
                    edge: edge_id,
                    incident_faces: edge.faces.len(),
                });
            }

            for &face_id in &edge.faces {
                let Some(face) = self.faces.get(face_id.0 as usize) else {
                    issues.push(TopologyIssue::InvalidEdgeFaceReference {
                        edge: edge_id,
                        face: face_id,
                    });
                    continue;
                };
                let uses_edge = face.vertices.iter().enumerate().any(|(corner, &v0)| {
                    let v1 = face.vertices[(corner + 1) % face.vertices.len()];
                    edge_key(v0, v1) == key
                });
                if !uses_edge {
                    issues.push(TopologyIssue::EdgeFaceMismatch {
                        edge: edge_id,
                        face: face_id,
                    });
                }
            }
        }

        for (&key, &edge_id) in &self.edge_lookup {
            let Some(edge) = self.edges.get(edge_id.0 as usize) else {
                issues.push(TopologyIssue::EdgeLookupMismatch { edge: edge_id });
                continue;
            };
            if edge_key(edge.vertices[0], edge.vertices[1]) != key {
                issues.push(TopologyIssue::EdgeLookupMismatch { edge: edge_id });
            }
        }

        for (face_index, face) in self.faces.iter().enumerate() {
            let face_id = FaceId(face_index as u32);
            for corner in 0..face.vertices.len() {
                let a = face.vertices[corner];
                let b = face.vertices[(corner + 1) % face.vertices.len()];
                let Some(&edge_id) = self.edge_lookup.get(&edge_key(a, b)) else {
                    continue;
                };
                let Some(edge) = self.edges.get(edge_id.0 as usize) else {
                    issues.push(TopologyIssue::EdgeLookupMismatch { edge: edge_id });
                    continue;
                };
                if !edge.faces.contains(&face_id) {
                    issues.push(TopologyIssue::EdgeFaceMismatch {
                        edge: edge_id,
                        face: face_id,
                    });
                }
            }
        }

        issues
    }

    /// Updates multiple vertex positions as one validated operation.
    ///
    /// Every ID and coordinate is checked before any vertex is changed. Duplicate
    /// IDs are rejected so a caller cannot accidentally apply conflicting edits.
    pub fn set_vertex_positions(
        &mut self,
        updates: &[(VertexId, [f32; 3])],
    ) -> Result<(), MeshError> {
        let mut seen = std::collections::HashSet::with_capacity(updates.len());

        for &(id, position) in updates {
            if self.vertices.get(id.0 as usize).is_none() {
                return Err(MeshError::InvalidVertex(id));
            }
            if !seen.insert(id) {
                return Err(MeshError::DuplicateVertexUpdate(id));
            }
            if !position.iter().all(|component| component.is_finite()) {
                return Err(MeshError::NonFinitePosition);
            }
        }

        for &(id, position) in updates {
            self.vertices[id.0 as usize].position = position;
        }
        Ok(())
    }

    /// Applies a shared transform to selected vertices atomically.
    ///
    /// The transform order is scale, X/Y/Z Euler rotation, then translation,
    /// all relative to the supplied pivot. This is a geometry primitive for
    /// future viewport move/rotate/scale gizmos; it does not manage UI state.
    pub fn transform_vertices(
        &mut self,
        vertices: &[VertexId],
        transform: Transform3D,
    ) -> Result<(), MeshError> {
        let mut updates = Vec::with_capacity(vertices.len());
        for &id in vertices {
            let position = self
                .vertex_position(id)
                .ok_or(MeshError::InvalidVertex(id))?;
            updates.push((id, transform.apply(position)?));
        }
        self.set_vertex_positions(&updates)
    }

    /// Translates a set of vertices atomically by the supplied finite offset.
    pub fn translate_vertices(
        &mut self,
        vertices: &[VertexId],
        delta: [f32; 3],
    ) -> Result<(), MeshError> {
        if !delta.iter().all(|component| component.is_finite()) {
            return Err(MeshError::NonFinitePosition);
        }

        let mut updates = Vec::with_capacity(vertices.len());
        for &id in vertices {
            let position = self
                .vertex_position(id)
                .ok_or(MeshError::InvalidVertex(id))?;
            updates.push((
                id,
                [
                    position[0] + delta[0],
                    position[1] + delta[1],
                    position[2] + delta[2],
                ],
            ));
        }
        self.set_vertex_positions(&updates)
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

    /// Extrudes one face by duplicating its vertices and adding a cap and side faces.
    ///
    /// The source face remains in place. The new cap keeps the source winding, and
    /// each side is a quad. Existing UV layers are copied to the cap where source
    /// coordinates exist; side UVs use a predictable world-unit rectangle (edge
    /// length by extrusion distance). All numeric inputs and derived positions are
    /// checked before topology is mutated.
    pub fn extrude_face(
        &mut self,
        face_id: FaceId,
        offset: [f32; 3],
    ) -> Result<ExtrusionResult, MeshError> {
        let source = self
            .faces
            .get(face_id.0 as usize)
            .ok_or(MeshError::FaceNotFound(face_id))?
            .vertices
            .clone();
        if !offset.iter().all(|value| value.is_finite()) {
            return Err(MeshError::NonFinitePosition);
        }

        let mut new_positions = Vec::with_capacity(source.len());
        for &vertex in &source {
            let position = self
                .vertex_position(vertex)
                .ok_or(MeshError::InvalidVertex(vertex))?;
            let translated = [
                position[0] + offset[0],
                position[1] + offset[1],
                position[2] + offset[2],
            ];
            if !translated.iter().all(|value| value.is_finite()) {
                return Err(MeshError::NonFinitePosition);
            }
            new_positions.push(translated);
        }

        let distance =
            (offset[0] * offset[0] + offset[1] * offset[1] + offset[2] * offset[2]).sqrt();
        if !distance.is_finite() {
            return Err(MeshError::NonFinitePosition);
        }
        let mut edge_lengths = Vec::with_capacity(source.len());
        for index in 0..source.len() {
            let a = self
                .vertex_position(source[index])
                .ok_or(MeshError::InvalidVertex(source[index]))?;
            let b = self
                .vertex_position(source[(index + 1) % source.len()])
                .ok_or(MeshError::InvalidVertex(source[(index + 1) % source.len()]))?;
            let dx = b[0] - a[0];
            let dy = b[1] - a[1];
            let dz = b[2] - a[2];
            let length = (dx * dx + dy * dy + dz * dz).sqrt();
            if !length.is_finite() {
                return Err(MeshError::NonFinitePosition);
            }
            edge_lengths.push(length);
        }

        // All fallible numeric and ID checks are complete before mutation.
        let uv_layers: Vec<String> = self.uv_layers.keys().cloned().collect();
        let source_uvs: HashMap<String, Vec<Option<Uv>>> = uv_layers
            .iter()
            .map(|name| {
                let layer = &self.uv_layers[name];
                let values = (0..source.len())
                    .map(|corner| layer.get(face_id, corner))
                    .collect();
                (name.clone(), values)
            })
            .collect();

        let new_vertices: Vec<VertexId> = new_positions
            .into_iter()
            .map(|position| self.add_vertex(position))
            .collect();
        let top_face = self.add_face(&new_vertices)?;

        for layer_name in &uv_layers {
            if let Some(values) = source_uvs.get(layer_name) {
                for (corner, uv) in values.iter().enumerate() {
                    if let Some(uv) = uv {
                        self.set_uv(layer_name, top_face, corner, *uv)?;
                    }
                }
            }
        }

        let mut side_faces = Vec::with_capacity(source.len());
        for index in 0..source.len() {
            let next = (index + 1) % source.len();
            let side = self.add_face(&[
                source[index],
                source[next],
                new_vertices[next],
                new_vertices[index],
            ])?;
            side_faces.push(side);

            for layer_name in &uv_layers {
                let length = edge_lengths[index];
                let side_uvs = [
                    Uv::new(0.0, 0.0),
                    Uv::new(length, 0.0),
                    Uv::new(length, distance),
                    Uv::new(0.0, distance),
                ];
                for (corner, uv) in side_uvs.into_iter().enumerate() {
                    self.set_uv(layer_name, side, corner, uv)?;
                }
            }
        }

        Ok(ExtrusionResult {
            vertices: new_vertices,
            top_face,
            side_faces,
        })
    }

    /// Deletes a face, removes orphaned edges, and rebuilds adjacency and UV corner keys.
    ///
    /// Face and edge IDs after the removed face may change because this mesh uses
    /// compact vector-index IDs. The returned remap contains every surviving ID.
    /// Vertex IDs and vertex positions are unchanged.
    pub fn remove_face(&mut self, face_id: FaceId) -> Result<TopologyRemap, MeshError> {
        if self.faces.get(face_id.0 as usize).is_none() {
            return Err(MeshError::FaceNotFound(face_id));
        }

        let old_faces = self.faces.clone();
        let old_edges = self.edges.clone();
        let mut face_map = HashMap::with_capacity(old_faces.len().saturating_sub(1));
        let mut new_face_id = 0u32;
        for index in 0..old_faces.len() {
            let old_id = FaceId(index as u32);
            if old_id != face_id {
                face_map.insert(old_id, FaceId(new_face_id));
                new_face_id += 1;
            }
        }

        self.faces.remove(face_id.0 as usize);
        for layer in self.uv_layers.values_mut() {
            layer.remap_faces(&face_map);
        }

        let seams: HashMap<(u32, u32), bool> = old_edges
            .iter()
            .map(|edge| (edge_key(edge.vertices[0], edge.vertices[1]), edge.seam))
            .collect();

        self.edges.clear();
        self.edge_lookup.clear();
        let mut edge_map = HashMap::new();

        for (face_index, face) in self.faces.iter().enumerate() {
            let current_face = FaceId(face_index as u32);
            for corner in 0..face.vertices.len() {
                let a = face.vertices[corner];
                let b = face.vertices[(corner + 1) % face.vertices.len()];
                let key = edge_key(a, b);

                if let Some(&edge_id) = self.edge_lookup.get(&key) {
                    self.edges[edge_id.0 as usize].faces.push(current_face);
                } else {
                    let edge_id = EdgeId(self.edges.len() as u32);
                    self.edges.push(Edge {
                        vertices: [a, b],
                        seam: seams.get(&key).copied().unwrap_or(false),
                        faces: vec![current_face],
                    });
                    self.edge_lookup.insert(key, edge_id);
                }
            }
        }

        for (old_index, old_edge) in old_edges.iter().enumerate() {
            if let Some(&new_id) = self
                .edge_lookup
                .get(&edge_key(old_edge.vertices[0], old_edge.vertices[1]))
            {
                edge_map.insert(EdgeId(old_index as u32), new_id);
            }
        }

        Ok(TopologyRemap {
            faces: face_map,
            edges: edge_map,
        })
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
