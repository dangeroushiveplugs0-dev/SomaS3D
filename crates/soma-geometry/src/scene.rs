//! Minimal document scene and object identity layer.
//!
//! This module deliberately contains no rendering, input, or platform code.
//! Objects keep geometry in local space; their transforms are evaluated by
//! the future viewport. Parametric objects can be edited until topology
//! editing explicitly converts them to ordinary mesh objects.

use crate::{
    generate_primitive, FaceId, HairError, HairObject, HairPreset, HairSettings, Mesh, MeshError,
    PrimitiveError, PrimitiveKind, Transform3D, VertexId,
};

/// Stable identity assigned by a scene. IDs are never reused within a scene.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ObjectId(pub u64);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SceneError {
    Primitive(PrimitiveError),
    Mesh(MeshError),
    Hair(HairError),
    ObjectNotFound(ObjectId),
    DuplicateObjectId(ObjectId),
    IdExhausted,
    RevisionOverflow,
    NotParametric(ObjectId),
}

impl From<MeshError> for SceneError {
    fn from(value: MeshError) -> Self {
        Self::Mesh(value)
    }
}

impl From<PrimitiveError> for SceneError {
    fn from(value: PrimitiveError) -> Self {
        Self::Primitive(value)
    }
}

impl From<HairError> for SceneError {
    fn from(value: HairError) -> Self {
        Self::Hair(value)
    }
}

#[derive(Debug, Clone)]
enum ObjectGeometry {
    Parametric {
        kind: PrimitiveKind,
        mesh: Mesh,
        revision: u64,
    },
    Mesh(Mesh),
}

/// One scene object with local geometry and a separate object transform.
#[derive(Debug, Clone)]
pub struct SceneObject {
    id: ObjectId,
    name: String,
    geometry: ObjectGeometry,
    transform: Transform3D,
}

impl SceneObject {
    pub fn id(&self) -> ObjectId {
        self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn rename(&mut self, name: impl Into<String>) {
        self.name = name.into();
    }

    pub fn transform(&self) -> Transform3D {
        self.transform
    }

    pub fn set_transform(&mut self, transform: Transform3D) {
        self.transform = transform;
    }

    pub fn mesh(&self) -> &Mesh {
        match &self.geometry {
            ObjectGeometry::Parametric { mesh, .. } | ObjectGeometry::Mesh(mesh) => mesh,
        }
    }

    pub fn is_parametric(&self) -> bool {
        matches!(&self.geometry, ObjectGeometry::Parametric { .. })
    }

    pub fn primitive_kind(&self) -> Option<PrimitiveKind> {
        match &self.geometry {
            ObjectGeometry::Parametric { kind, .. } => Some(*kind),
            ObjectGeometry::Mesh(_) => None,
        }
    }

    pub fn geometry_revision(&self) -> u64 {
        match &self.geometry {
            ObjectGeometry::Parametric { revision, .. } => *revision,
            ObjectGeometry::Mesh(_) => 0,
        }
    }

    /// Replaces local geometry with an edited mesh and marks the object non-parametric.
    /// Callers should provide a fully validated candidate mesh so failed edits stay atomic.
    pub(crate) fn commit_edited_mesh(&mut self, mesh: Mesh) {
        self.geometry = ObjectGeometry::Mesh(mesh);
    }

    /// Returns a transformed copy of this object's mesh for viewport evaluation.
    /// The stored mesh remains in local space and is never modified.
    pub fn evaluated_mesh(&self) -> Result<Mesh, SceneError> {
        let mut mesh = self.mesh().clone();
        let vertices: Vec<_> = (0..mesh.vertex_count())
            .map(|index| VertexId(index as u32))
            .collect();
        mesh.transform_vertices(&vertices, self.transform)?;
        Ok(mesh)
    }
}

/// Hair scene object whose guide roots are stored in the source object's local space.
/// Its source object transform is therefore also the transform used for hair rendering.
#[derive(Debug, Clone, PartialEq)]
pub struct HairSceneObject {
    id: ObjectId,
    name: String,
    source_object: ObjectId,
    hair: HairObject,
}

impl HairSceneObject {
    pub fn id(&self) -> ObjectId {
        self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn source_object(&self) -> ObjectId {
        self.source_object
    }

    pub fn hair(&self) -> &HairObject {
        &self.hair
    }
}

/// Scene/document container for mesh objects, procedural hair, and active-object state.
#[derive(Debug, Clone, Default)]
pub struct Scene {
    objects: Vec<SceneObject>,
    hair_objects: Vec<HairSceneObject>,
    active_object: Option<ObjectId>,
    next_id: u64,
}

impl Scene {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn objects(&self) -> &[SceneObject] {
        &self.objects
    }

    pub fn hair_objects(&self) -> &[HairSceneObject] {
        &self.hair_objects
    }

    pub fn hair_object(&self, id: ObjectId) -> Option<&HairSceneObject> {
        self.hair_objects.iter().find(|object| object.id == id)
    }

    pub fn object(&self, id: ObjectId) -> Option<&SceneObject> {
        self.objects.iter().find(|object| object.id == id)
    }

    pub fn object_mut(&mut self, id: ObjectId) -> Option<&mut SceneObject> {
        self.objects.iter_mut().find(|object| object.id == id)
    }

    pub fn active_object_id(&self) -> Option<ObjectId> {
        self.active_object
    }

    pub fn set_active_object(&mut self, id: Option<ObjectId>) -> Result<(), SceneError> {
        if let Some(id) = id {
            if self.object(id).is_none() && self.hair_object(id).is_none() {
                return Err(SceneError::ObjectNotFound(id));
            }
        }
        self.active_object = id;
        Ok(())
    }

    /// Creates a parametric primitive and makes it the active object.
    pub fn add_primitive(
        &mut self,
        name: impl Into<String>,
        kind: PrimitiveKind,
    ) -> Result<ObjectId, SceneError> {
        let mesh = generate_primitive(kind)?;
        let id = self.allocate_id()?;
        self.objects.push(SceneObject {
            id,
            name: name.into(),
            geometry: ObjectGeometry::Parametric {
                kind,
                mesh,
                revision: 0,
            },
            transform: Transform3D::default(),
        });
        self.active_object = Some(id);
        Ok(id)
    }

    /// Adds an ordinary editable mesh as a new scene object.
    pub fn add_mesh(
        &mut self,
        name: impl Into<String>,
        mesh: Mesh,
    ) -> Result<ObjectId, SceneError> {
        let id = self.allocate_id()?;
        self.objects.push(SceneObject {
            id,
            name: name.into(),
            geometry: ObjectGeometry::Mesh(mesh),
            transform: Transform3D::default(),
        });
        self.active_object = Some(id);
        Ok(id)
    }

    /// Creates a procedural hair object attached to selected faces of a mesh object.
    /// Hair is stored separately and does not alter the source mesh topology.
    pub fn add_hair_object(
        &mut self,
        name: impl Into<String>,
        source_object: ObjectId,
        selected_faces: &[FaceId],
        preset: HairPreset,
        settings: HairSettings,
    ) -> Result<ObjectId, SceneError> {
        let source_mesh = self
            .object(source_object)
            .ok_or(SceneError::ObjectNotFound(source_object))?
            .mesh();
        let hair = HairObject::generate(source_mesh, selected_faces, preset, settings)?;
        let id = self.allocate_id()?;
        self.hair_objects.push(HairSceneObject {
            id,
            name: name.into(),
            source_object,
            hair,
        });
        self.active_object = Some(id);
        Ok(id)
    }

    /// Rebuilds a hair object's guides against its current source mesh.
    /// The existing guides remain intact if generation fails.
    pub fn restyle_hair(
        &mut self,
        id: ObjectId,
        settings: HairSettings,
    ) -> Result<(), SceneError> {
        let hair_object = self
            .hair_object(id)
            .ok_or(SceneError::ObjectNotFound(id))?;
        let source_object = hair_object.source_object;
        let source_faces = hair_object.hair.source_faces().to_vec();
        let preset = hair_object.hair.preset();
        let source_mesh = self
            .object(source_object)
            .ok_or(SceneError::ObjectNotFound(source_object))?
            .mesh();
        let replacement = HairObject::generate(source_mesh, &source_faces, preset, settings)?;
        self.hair_object_mut(id)
            .ok_or(SceneError::ObjectNotFound(id))?
            .hair = replacement;
        Ok(())
    }

    fn hair_object_mut(&mut self, id: ObjectId) -> Option<&mut HairSceneObject> {
        self.hair_objects.iter_mut().find(|object| object.id == id)
    }

    /// Removes a hair object and clears active selection if needed.
    pub fn remove_hair_object(&mut self, id: ObjectId) -> Result<HairSceneObject, SceneError> {
        let index = self
            .hair_objects
            .iter()
            .position(|object| object.id == id)
            .ok_or(SceneError::ObjectNotFound(id))?;
        let removed = self.hair_objects.remove(index);
        if self.active_object == Some(id) {
            self.active_object = None;
        }
        Ok(removed)
    }

    /// Regenerates a primitive without changing its object ID or transform.
    pub fn update_primitive(
        &mut self,
        id: ObjectId,
        kind: PrimitiveKind,
    ) -> Result<(), SceneError> {
        let object = self.object_mut(id).ok_or(SceneError::ObjectNotFound(id))?;
        let (old_kind, old_revision) = match &object.geometry {
            ObjectGeometry::Parametric { kind, revision, .. } => (*kind, *revision),
            ObjectGeometry::Mesh(_) => return Err(SceneError::NotParametric(id)),
        };
        if old_kind == kind {
            return Ok(());
        }
        let revision = old_revision
            .checked_add(1)
            .ok_or(SceneError::RevisionOverflow)?;
        let mesh = generate_primitive(kind)?;
        object.geometry = ObjectGeometry::Parametric {
            kind,
            mesh,
            revision,
        };
        Ok(())
    }

    /// Converts a parametric object to ordinary mesh geometry, preserving its
    /// current mesh, ID, name, and transform. Future topology tools can call
    /// this before applying direct mesh edits.
    pub fn make_editable_mesh(&mut self, id: ObjectId) -> Result<(), SceneError> {
        let object = self.object_mut(id).ok_or(SceneError::ObjectNotFound(id))?;
        let mesh = match &object.geometry {
            ObjectGeometry::Parametric { mesh, .. } => mesh.clone(),
            ObjectGeometry::Mesh(_) => return Ok(()),
        };
        object.geometry = ObjectGeometry::Mesh(mesh);
        Ok(())
    }

    /// Removes an object and clears active selection if it was active.
    pub fn remove_object(&mut self, id: ObjectId) -> Result<SceneObject, SceneError> {
        let index = self
            .objects
            .iter()
            .position(|object| object.id == id)
            .ok_or(SceneError::ObjectNotFound(id))?;
        let removed = self.objects.remove(index);
        let removed_hair_ids: Vec<_> = self
            .hair_objects
            .iter()
            .filter(|hair| hair.source_object == id)
            .map(|hair| hair.id)
            .collect();
        self.hair_objects
            .retain(|hair| hair.source_object != id);
        if self
            .active_object
            .is_some_and(|active| active == id || removed_hair_ids.contains(&active))
        {
            self.active_object = None;
        }
        Ok(removed)
    }

    fn allocate_id(&mut self) -> Result<ObjectId, SceneError> {
        let id = ObjectId(self.next_id);
        self.next_id = self.next_id.checked_add(1).ok_or(SceneError::IdExhausted)?;
        Ok(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scene_allocates_unique_ids_and_tracks_active_object() {
        let mut scene = Scene::new();
        let first = scene
            .add_primitive("Cube", PrimitiveKind::Cube { size: 1.0 })
            .unwrap();
        let second = scene
            .add_primitive(
                "Plane",
                PrimitiveKind::Plane {
                    width: 2.0,
                    depth: 3.0,
                },
            )
            .unwrap();
        assert_ne!(first, second);
        assert_eq!(scene.objects().len(), 2);
        assert_eq!(scene.active_object_id(), Some(second));
        scene.set_active_object(Some(first)).unwrap();
        assert_eq!(scene.active_object_id(), Some(first));
    }

    #[test]
    fn hair_is_a_separate_scene_object_and_tracks_source_identity() {
        let mut scene = Scene::new();
        let mut mesh = Mesh::new();
        let a = mesh.add_vertex([0.0, 0.0, 0.0]);
        let b = mesh.add_vertex([1.0, 0.0, 0.0]);
        let c = mesh.add_vertex([0.0, 0.0, 1.0]);
        let face = mesh.add_face(&[a, c, b]).unwrap();
        let source = scene.add_mesh("Character", mesh).unwrap();
        let hair_id = scene
            .add_hair_object(
                "Scalp Hair",
                source,
                &[face],
                HairPreset::FlowingHair,
                HairSettings::flowing_hair(),
            )
            .unwrap();

        assert_ne!(source, hair_id);
        assert_eq!(scene.active_object_id(), Some(hair_id));
        assert_eq!(scene.hair_objects().len(), 1);
        assert_eq!(scene.hair_object(hair_id).unwrap().source_object(), source);
        assert_eq!(scene.hair_object(hair_id).unwrap().hair().guides().len(), 512);
        assert_eq!(scene.object(source).unwrap().mesh().vertex_count(), 3);
        scene.set_active_object(Some(source)).unwrap();
    }

    #[test]
    fn removing_source_object_also_removes_attached_hair() {
        let mut scene = Scene::new();
        let mut mesh = Mesh::new();
        let a = mesh.add_vertex([0.0, 0.0, 0.0]);
        let b = mesh.add_vertex([1.0, 0.0, 0.0]);
        let c = mesh.add_vertex([0.0, 0.0, 1.0]);
        let face = mesh.add_face(&[a, c, b]).unwrap();
        let source = scene.add_mesh("Character", mesh).unwrap();
        let hair = scene
            .add_hair_object(
                "Hair",
                source,
                &[face],
                HairPreset::ShortHair,
                HairSettings::short_hair(),
            )
            .unwrap();

        scene.remove_object(source).unwrap();

        assert!(scene.hair_object(hair).is_none());
        assert!(scene.objects().is_empty());
        assert!(scene.hair_objects().is_empty());
        assert_eq!(scene.active_object_id(), None);
    }

    #[test]
    fn parameter_updates_preserve_identity_transform_and_are_atomic() {
        let mut scene = Scene::new();
        let id = scene
            .add_primitive(
                "Sphere",
                PrimitiveKind::UvSphere {
                    radius: 1.0,
                    segments: 8,
                    rings: 4,
                },
            )
            .unwrap();
        let transform = Transform3D {
            translation: [3.0, 2.0, -1.0],
            ..Transform3D::default()
        };
        scene.object_mut(id).unwrap().set_transform(transform);
        scene
            .update_primitive(
                id,
                PrimitiveKind::UvSphere {
                    radius: 2.0,
                    segments: 8,
                    rings: 4,
                },
            )
            .unwrap();
        let object = scene.object(id).unwrap();
        assert_eq!(object.id(), id);
        assert_eq!(object.transform(), transform);
        assert_eq!(object.geometry_revision(), 1);
        assert_eq!(object.mesh().vertex_count(), 26);

        let result = scene.update_primitive(id, PrimitiveKind::Cube { size: f32::NAN });
        assert_eq!(
            result,
            Err(SceneError::Primitive(PrimitiveError::NonFiniteDimension))
        );
        assert_eq!(
            scene.object(id).unwrap().primitive_kind(),
            Some(PrimitiveKind::UvSphere {
                radius: 2.0,
                segments: 8,
                rings: 4,
            })
        );
        assert_eq!(scene.object(id).unwrap().geometry_revision(), 1);
    }

    #[test]
    fn converting_parametric_object_preserves_mesh_and_transform() {
        let mut scene = Scene::new();
        let id = scene
            .add_primitive("Cube", PrimitiveKind::Cube { size: 2.0 })
            .unwrap();
        let transform = Transform3D {
            scale: [2.0, 1.0, 0.5],
            ..Transform3D::default()
        };
        scene.object_mut(id).unwrap().set_transform(transform);
        let before = scene.object(id).unwrap().mesh().vertex_count();

        scene.make_editable_mesh(id).unwrap();

        let object = scene.object(id).unwrap();
        assert!(!object.is_parametric());
        assert_eq!(object.primitive_kind(), None);
        assert_eq!(object.mesh().vertex_count(), before);
        assert_eq!(object.transform(), transform);
        assert_eq!(object.id(), id);
    }

    #[test]
    fn evaluated_mesh_applies_object_transform_without_changing_local_geometry() {
        let mut scene = Scene::new();
        let id = scene
            .add_primitive("Cube", PrimitiveKind::Cube { size: 2.0 })
            .unwrap();
        let object = scene.object(id).unwrap();
        let local_before = object.mesh().vertex_position(VertexId(0)).unwrap();
        scene.object_mut(id).unwrap().set_transform(Transform3D {
            translation: [3.0, -2.0, 5.0],
            ..Transform3D::default()
        });

        let evaluated = scene.object(id).unwrap().evaluated_mesh().unwrap();
        assert_eq!(
            evaluated.vertex_position(VertexId(0)),
            Some([
                local_before[0] + 3.0,
                local_before[1] - 2.0,
                local_before[2] + 5.0,
            ])
        );
        assert_eq!(
            scene
                .object(id)
                .unwrap()
                .mesh()
                .vertex_position(VertexId(0)),
            Some(local_before)
        );
        assert!(evaluated.validate_topology().is_empty());
    }

    #[test]
    fn invalid_object_transform_fails_evaluation_without_mutating_scene_mesh() {
        let mut scene = Scene::new();
        let id = scene
            .add_primitive("Cube", PrimitiveKind::Cube { size: 1.0 })
            .unwrap();
        scene.object_mut(id).unwrap().set_transform(Transform3D {
            translation: [f32::NAN, 0.0, 0.0],
            ..Transform3D::default()
        });
        assert!(matches!(
            scene.object(id).unwrap().evaluated_mesh(),
            Err(SceneError::Mesh(MeshError::NonFinitePosition))
        ));
        assert_eq!(
            scene
                .object(id)
                .unwrap()
                .mesh()
                .vertex_position(VertexId(0)),
            Some([-0.5, -0.5, -0.5])
        );
    }

    #[test]
    fn removing_active_object_clears_active_id_and_keeps_other_objects() {
        let mut scene = Scene::new();
        let first = scene
            .add_primitive("A", PrimitiveKind::Cube { size: 1.0 })
            .unwrap();
        let second = scene
            .add_primitive("B", PrimitiveKind::Cube { size: 2.0 })
            .unwrap();
        scene.remove_object(second).unwrap();
        assert_eq!(scene.active_object_id(), None);
        assert!(scene.object(first).is_some());
        assert!(scene.object(second).is_none());
    }

    #[test]
    fn invalid_active_object_does_not_change_active_state() {
        let mut scene = Scene::new();
        let id = scene
            .add_primitive("Cube", PrimitiveKind::Cube { size: 1.0 })
            .unwrap();
        assert_eq!(
            scene.set_active_object(Some(ObjectId(99))),
            Err(SceneError::ObjectNotFound(ObjectId(99)))
        );
        assert_eq!(scene.active_object_id(), Some(id));
    }

    #[test]
    fn mesh_objects_reject_parametric_updates() {
        let mut scene = Scene::new();
        let id = scene
            .add_primitive("Cube", PrimitiveKind::Cube { size: 1.0 })
            .unwrap();
        scene.make_editable_mesh(id).unwrap();
        assert_eq!(
            scene.update_primitive(id, PrimitiveKind::Cube { size: 3.0 }),
            Err(SceneError::NotParametric(id))
        );
    }
}
