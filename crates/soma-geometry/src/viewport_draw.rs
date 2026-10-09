//! Compact, renderer-neutral draw buffers derived from a viewport snapshot.
//!
//! This is the next boundary toward a visible renderer: it expands polygon
//! faces into triangle indices, extracts unique mesh edges, and flattens hair
//! guides into line-segment endpoints. It deliberately does not create a GPU
//! device, camera, shaders, or platform UI.

use crate::{ObjectId, ViewportSceneSnapshot};
use std::collections::BTreeSet;

/// Indexed surface and wireframe buffers for one mesh object.
#[derive(Debug, Clone, PartialEq)]
pub struct ViewportMeshDrawData {
    pub object_id: ObjectId,
    pub name: String,
    /// World-space positions shared by surface and edge buffers.
    pub positions: Vec<[f32; 3]>,
    /// Triangle-list indices. Convex polygons are triangulated as a fan.
    pub triangle_indices: Vec<u32>,
    /// Unique undirected edge pairs, suitable for a line-list draw call.
    pub edge_indices: Vec<u32>,
    pub is_parametric: bool,
}

/// One hair object's line-list data, retaining its authored color and identity.
#[derive(Debug, Clone, PartialEq)]
pub struct ViewportHairDrawData {
    pub object_id: ObjectId,
    pub name: String,
    pub color: [f32; 4],
    /// Consecutive endpoint pairs; each pair represents one guide segment.
    pub line_positions: Vec<[f32; 3]>,
}

/// A complete CPU-side draw packet for a renderer to upload or consume.
#[derive(Debug, Clone, PartialEq)]
pub struct ViewportDrawData {
    pub active_object: Option<ObjectId>,
    pub meshes: Vec<ViewportMeshDrawData>,
    pub hair: Vec<ViewportHairDrawData>,
}

impl ViewportSceneSnapshot {
    /// Builds indexed surface/wireframe buffers and hair line lists.
    ///
    /// Polygon faces are triangulated as a fan, which is correct for the
    /// convex faces produced by the current primitive and extrusion APIs.
    /// A future general-purpose triangulator should replace this for arbitrary
    /// concave imported polygons.
    pub fn draw_data(&self) -> ViewportDrawData {
        let meshes = self
            .meshes
            .iter()
            .map(|mesh| {
                let mut triangle_indices = Vec::new();
                let mut unique_edges = BTreeSet::new();

                for polygon in &mesh.polygons {
                    if polygon.len() < 2 {
                        continue;
                    }
                    for index in 0..polygon.len() {
                        let a = polygon[index];
                        let b = polygon[(index + 1) % polygon.len()];
                        unique_edges.insert(if a <= b { (a, b) } else { (b, a) });
                    }
                    if polygon.len() >= 3 {
                        for index in 1..polygon.len() - 1 {
                            triangle_indices.extend_from_slice(&[
                                polygon[0],
                                polygon[index],
                                polygon[index + 1],
                            ]);
                        }
                    }
                }

                let edge_indices = unique_edges.into_iter().flat_map(|(a, b)| [a, b]).collect();

                ViewportMeshDrawData {
                    object_id: mesh.id,
                    name: mesh.name.clone(),
                    positions: mesh.positions.clone(),
                    triangle_indices,
                    edge_indices,
                    is_parametric: mesh.is_parametric,
                }
            })
            .collect();

        let hair = self
            .hair
            .iter()
            .map(|object| {
                let mut line_positions = Vec::new();
                for guide in &object.guides {
                    for segment in guide.windows(2) {
                        line_positions.extend_from_slice(&[segment[0], segment[1]]);
                    }
                }
                ViewportHairDrawData {
                    object_id: object.object_id,
                    name: object.name.clone(),
                    color: object.color,
                    line_positions,
                }
            })
            .collect();

        ViewportDrawData {
            active_object: self.active_object,
            meshes,
            hair,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{HairPreset, HairSettings, PrimitiveKind, Scene};

    #[test]
    fn cube_draw_data_has_triangle_and_unique_edge_buffers() {
        let mut scene = Scene::new();
        let cube = scene
            .add_primitive("Cube", PrimitiveKind::Cube { size: 1.0 })
            .unwrap();

        let draw = scene.viewport_snapshot().unwrap().draw_data();

        assert_eq!(draw.active_object, Some(cube));
        assert_eq!(draw.meshes.len(), 1);
        let mesh = &draw.meshes[0];
        assert_eq!(mesh.positions.len(), 8);
        assert_eq!(mesh.triangle_indices.len(), 36);
        assert_eq!(mesh.edge_indices.len(), 24);
        assert!(mesh
            .triangle_indices
            .iter()
            .all(|index| (*index as usize) < mesh.positions.len()));
        assert!(mesh
            .edge_indices
            .iter()
            .all(|index| (*index as usize) < mesh.positions.len()));
    }

    #[test]
    fn hair_guides_become_line_segments_without_losing_color_or_identity() {
        let mut scene = Scene::new();
        let mut source_mesh = crate::Mesh::new();
        let a = source_mesh.add_vertex([0.0, 0.0, 0.0]);
        let b = source_mesh.add_vertex([1.0, 0.0, 0.0]);
        let c = source_mesh.add_vertex([0.0, 0.0, 1.0]);
        let face = source_mesh.add_face(&[a, c, b]).unwrap();
        let source = scene.add_mesh("Scalp", source_mesh).unwrap();
        let settings = HairSettings {
            amount: 3,
            ..HairSettings::short_hair()
        };
        let hair_id = scene
            .add_hair_object("Hair", source, &[face], HairPreset::ShortHair, settings)
            .unwrap();

        let draw = scene.viewport_snapshot().unwrap().draw_data();

        assert_eq!(draw.hair.len(), 1);
        let hair = &draw.hair[0];
        assert_eq!(hair.object_id, hair_id);
        assert_eq!(hair.name, "Hair");
        assert_eq!(hair.color, settings.color);
        // Each guide currently has six points, hence five line segments.
        assert_eq!(hair.line_positions.len(), 3 * 5 * 2);
    }

    #[test]
    fn shared_polygon_edges_are_emitted_only_once() {
        let mut scene = Scene::new();
        let cube = scene
            .add_primitive("Cube", PrimitiveKind::Cube { size: 1.0 })
            .unwrap();
        let draw = scene.viewport_snapshot().unwrap().draw_data();
        let mesh = draw
            .meshes
            .iter()
            .find(|mesh| mesh.object_id == cube)
            .unwrap();

        let edge_pairs: Vec<_> = mesh.edge_indices.chunks_exact(2).collect();
        assert_eq!(edge_pairs.len(), 12);
        let mut sorted = edge_pairs
            .iter()
            .map(|edge| (edge[0].min(edge[1]), edge[0].max(edge[1])))
            .collect::<Vec<_>>();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.len(), edge_pairs.len());
    }
}
