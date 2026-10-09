//! Minimal document scene and object identity layer.
//!
//! This module deliberately contains no rendering, input, or platform code.
//! Objects keep geometry in local space; their transforms are evaluated by
//! the future viewport. Parametric objects can be edited until topology
//! editing explicitly converts them to ordinary mesh objects.

use crate::{generate_primitive, Mesh, PrimitiveError, PrimitiveKind, Transform3D};

/// Stable identity assigned by a scene. IDs are never reused within a scene.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ObjectId(pub u64);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SceneError {
    Primitive(PrimitiveError),
    ObjectNotFound(ObjectId),
    DuplicateObjectId(ObjectId),
    IdExhausted,
    RevisionOverflow,
    NotParametric(ObjectId),
}

impl From<PrimitiveError> for SceneError {
    fn from(value: PrimitiveError) -> Self {
        Self::Primitive(value)
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
}

/// Scene/document container for objects and active-object state.
#[derive(Debug, Clone, Default)]
pub struct Scene {
    objects: Vec<SceneObject>,
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
            if self.object(id).is_none() {
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
        if self.active_object == Some(id) {
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
