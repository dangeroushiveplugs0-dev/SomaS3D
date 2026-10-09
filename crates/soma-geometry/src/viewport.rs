//! Renderer-neutral scene snapshots for the future interactive viewport.
//!
//! This layer evaluates object transforms and flattens the document into simple
//! positions, polygon indices, and hair polylines. Platform-specific renderers
//! can consume the snapshot without reaching into mutable document state.

use crate::{EvaluatedHairGuides, FaceId, MeshError, ObjectId, Scene, SceneError, VertexId};

/// One evaluated mesh object with world-space positions and polygon indices.
#[derive(Debug, Clone, PartialEq)]
pub struct ViewportMeshObject {
    pub id: ObjectId,
    pub name: String,
    /// World-space vertex positions, indexed by the polygon index buffers.
    pub positions: Vec<[f32; 3]>,
    /// Each polygon contains indices into the positions vector.
    pub polygons: Vec<Vec<u32>>,
    pub is_parametric: bool,
}

/// Immutable render input assembled from the current scene document.
#[derive(Debug, Clone, PartialEq)]
pub struct ViewportSceneSnapshot {
    pub active_object: Option<ObjectId>,
    pub meshes: Vec<ViewportMeshObject>,
    pub hair: Vec<EvaluatedHairGuides>,
}

impl Scene {
    /// Builds a complete renderer-neutral snapshot of mesh objects and hair.
    ///
    /// Meshes and guides are evaluated in world space. This method never
    /// mutates the document; if any object cannot be evaluated, no partial
    /// snapshot is returned.
    pub fn viewport_snapshot(&self) -> Result<ViewportSceneSnapshot, SceneError> {
        let mut meshes = Vec::with_capacity(self.objects().len());
        for object in self.objects() {
            let evaluated = object.evaluated_mesh()?;
            let mut positions = Vec::with_capacity(evaluated.vertex_count());
            for index in 0..evaluated.vertex_count() {
                let vertex_id = VertexId(index as u32);
                positions.push(
                    evaluated
                        .vertex_position(vertex_id)
                        .ok_or(SceneError::Mesh(MeshError::InvalidVertex(vertex_id)))?,
                );
            }

            let mut polygons = Vec::with_capacity(evaluated.face_count());
            for index in 0..evaluated.face_count() {
                let face_id = FaceId(index as u32);
                let face = evaluated
                    .face(face_id)
                    .ok_or(SceneError::Mesh(MeshError::FaceNotFound(face_id)))?;
                let mut polygon = Vec::with_capacity(face.vertices.len());
                for vertex in &face.vertices {
                    if vertex.0 as usize >= positions.len() {
                        return Err(SceneError::Mesh(MeshError::InvalidVertex(*vertex)));
                    }
                    polygon.push(vertex.0);
                }
                polygons.push(polygon);
            }

            meshes.push(ViewportMeshObject {
                id: object.id(),
                name: object.name().to_owned(),
                positions,
                polygons,
                is_parametric: object.is_parametric(),
            });
        }

        let mut hair = Vec::with_capacity(self.hair_objects().len());
        for object in self.hair_objects() {
            hair.push(self.evaluated_hair_guides(object.id())?);
        }

        Ok(ViewportSceneSnapshot {
            active_object: self.active_object_id(),
            meshes,
            hair,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{HairPreset, HairSettings, Mesh, PrimitiveKind, Transform3D};

    #[test]
    fn snapshot_contains_world_space_mesh_positions_and_polygon_indices() {
        let mut scene = Scene::new();
        let object_id = scene.add_primitive("Cube", PrimitiveKind::Cube).unwrap();
        scene
            .object_mut(object_id)
            .unwrap()
            .set_transform(Transform3D {
                translation: [2.0, 3.0, -4.0],
                ..Transform3D::default()
            });

        let snapshot = scene.viewport_snapshot().unwrap();
        let object = &snapshot.meshes[0];
        assert_eq!(snapshot.active_object, Some(object_id));
        assert_eq!(object.id, object_id);
        assert_eq!(object.name, "Cube");
        assert_eq!(object.positions.len(), 8);
        assert_eq!(object.polygons.len(), 6);
        assert!(object.positions.iter().any(|p| p[0] > 1.0 && p[1] > 2.0));
        assert!(object.is_parametric);
    }

    #[test]
    fn snapshot_includes_attached_hair_and_preserves_document_geometry() {
        let mut mesh = Mesh::new();
        let a = mesh.add_vertex([0.0, 0.0, 0.0]);
        let b = mesh.add_vertex([1.0, 0.0, 0.0]);
        let c = mesh.add_vertex([0.0, 0.0, 1.0]);
        let face = mesh.add_face(&[a, c, b]).unwrap();
        let mut scene = Scene::new();
        let source = scene.add_mesh("Character", mesh).unwrap();
        let hair_id = scene
            .add_hair_object(
                "Hair",
                source,
                &[face],
                HairPreset::ShortHair,
                HairSettings::short_hair(),
            )
            .unwrap();
        let before = scene.object(source).unwrap().mesh().vertex_position(a);

        let snapshot = scene.viewport_snapshot().unwrap();

        assert_eq!(snapshot.active_object, Some(hair_id));
        assert_eq!(snapshot.meshes.len(), 1);
        assert_eq!(snapshot.hair.len(), 1);
        assert_eq!(snapshot.hair[0].object_id, hair_id);
        assert_eq!(snapshot.hair[0].guides.len(), HairSettings::short_hair().amount);
        assert_eq!(scene.object(source).unwrap().mesh().vertex_position(a), before);
    }

    #[test]
    fn snapshot_fails_atomically_when_an_object_transform_is_invalid() {
        let mut scene = Scene::new();
        let id = scene.add_primitive("Cube", PrimitiveKind::Cube).unwrap();
        scene.object_mut(id).unwrap().set_transform(Transform3D {
            scale: [f32::NAN, 1.0, 1.0],
            ..Transform3D::default()
        });
        assert_eq!(
            scene.viewport_snapshot(),
            Err(SceneError::Mesh(MeshError::NonFinitePosition))
        );
    }
}
