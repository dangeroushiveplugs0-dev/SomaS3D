//! Document-wide undo/redo for scene-level operations.
//!
//! Each successful command records a complete scene snapshot, so object creation,
//! deletion, renaming, transforms, and primitive updates share one history.

use crate::{
    EditHistory, FaceId, HairPreset, HairSettings, ObjectId, PrimitiveKind, Scene, SceneError,
    SceneObject, Transform3D,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DocumentEditError {
    Scene(SceneError),
}

impl From<SceneError> for DocumentEditError {
    fn from(value: SceneError) -> Self {
        Self::Scene(value)
    }
}

/// Scene document with a single bounded undo/redo timeline.
#[derive(Debug, Clone)]
pub struct SceneDocumentEditor {
    history: EditHistory<Scene>,
}

impl SceneDocumentEditor {
    pub fn new(scene: Scene, undo_limit: usize) -> Self {
        Self {
            history: EditHistory::new(scene, undo_limit),
        }
    }

    pub fn scene(&self) -> &Scene {
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

    pub fn add_primitive(
        &mut self,
        name: impl Into<String>,
        kind: PrimitiveKind,
    ) -> Result<ObjectId, DocumentEditError> {
        let name = name.into();
        let mut created = None;
        self.history.apply(|scene| {
            created = Some(scene.add_primitive(name, kind)?);
            Ok::<_, SceneError>(())
        })?;
        Ok(created.expect("successful primitive creation returns an object ID"))
    }

    pub fn add_hair_object(
        &mut self,
        name: impl Into<String>,
        source_object: ObjectId,
        selected_faces: &[FaceId],
        preset: HairPreset,
        settings: HairSettings,
    ) -> Result<ObjectId, DocumentEditError> {
        let name = name.into();
        let mut created = None;
        self.history.apply(|scene| {
            created = Some(scene.add_hair_object(
                name,
                source_object,
                selected_faces,
                preset,
                settings,
            )?);
            Ok::<_, SceneError>(())
        })?;
        Ok(created.expect("successful hair creation returns an object ID"))
    }

    pub fn restyle_hair(
        &mut self,
        id: ObjectId,
        settings: HairSettings,
    ) -> Result<(), DocumentEditError> {
        self.history.apply(|scene| scene.restyle_hair(id, settings))?;
        Ok(())
    }

    pub fn remove_hair_object(&mut self, id: ObjectId) -> Result<(), DocumentEditError> {
        self.history.apply(|scene| scene.remove_hair_object(id).map(|_| ()))?;
        Ok(())
    }

    pub fn remove_object(&mut self, id: ObjectId) -> Result<SceneObject, DocumentEditError> {
        let mut removed = None;
        self.history.apply(|scene| {
            removed = Some(scene.remove_object(id)?);
            Ok::<_, SceneError>(())
        })?;
        Ok(removed.expect("successful object removal returns the removed object"))
    }

    pub fn rename_object(
        &mut self,
        id: ObjectId,
        name: impl Into<String>,
    ) -> Result<(), DocumentEditError> {
        let name = name.into();
        self.history.apply(|scene| {
            let object = scene.object_mut(id).ok_or(SceneError::ObjectNotFound(id))?;
            if object.name() != name {
                object.rename(name);
            }
            Ok::<_, SceneError>(())
        })?;
        Ok(())
    }

    pub fn set_object_transform(
        &mut self,
        id: ObjectId,
        transform: Transform3D,
    ) -> Result<(), DocumentEditError> {
        self.history.apply(|scene| {
            let object = scene.object_mut(id).ok_or(SceneError::ObjectNotFound(id))?;
            object.set_transform(transform);
            Ok::<_, SceneError>(())
        })?;
        Ok(())
    }

    /// Applies a validated local-mesh edit on the same document undo/redo timeline.
    ///
    /// The mesh is edited on a clone and committed only when the callback succeeds.
    /// Editing a parametric object converts it to ordinary mesh geometry; undo restores
    /// its original parametric state and redo restores the edited mesh.
    pub fn edit_mesh(
        &mut self,
        id: ObjectId,
        edit: impl FnOnce(&mut crate::Mesh) -> Result<(), crate::MeshError>,
    ) -> Result<(), DocumentEditError> {
        self.history.apply(|scene| {
            let mut mesh = scene
                .object(id)
                .ok_or(SceneError::ObjectNotFound(id))?
                .mesh()
                .clone();
            edit(&mut mesh).map_err(SceneError::from)?;
            scene
                .object_mut(id)
                .ok_or(SceneError::ObjectNotFound(id))?
                .commit_edited_mesh(mesh);
            Ok::<_, SceneError>(())
        })?;
        Ok(())
    }

    pub fn update_primitive(
        &mut self,
        id: ObjectId,
        kind: PrimitiveKind,
    ) -> Result<(), DocumentEditError> {
        self.history
            .apply(|scene| scene.update_primitive(id, kind))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::PrimitiveKind;

    #[test]
    fn undo_redo_restores_object_creation_and_deletion() {
        let mut document = SceneDocumentEditor::new(Scene::new(), 8);
        let id = document
            .add_primitive("Cube", PrimitiveKind::Cube { size: 1.0 })
            .unwrap();
        assert_eq!(document.scene().objects().len(), 1);
        assert!(document.undo());
        assert!(document.scene().objects().is_empty());
        assert!(document.redo());
        assert_eq!(document.scene().object(id).unwrap().name(), "Cube");

        document.remove_object(id).unwrap();
        assert!(document.scene().object(id).is_none());
        assert!(document.undo());
        assert_eq!(document.scene().object(id).unwrap().name(), "Cube");
    }

    #[test]
    fn undo_redo_restores_rename_and_transform() {
        let mut document = SceneDocumentEditor::new(Scene::new(), 8);
        let id = document
            .add_primitive("Cube", PrimitiveKind::Cube { size: 1.0 })
            .unwrap();
        let transform = Transform3D {
            translation: [3.0, 2.0, 1.0],
            ..Transform3D::default()
        };
        document.rename_object(id, "Hero").unwrap();
        document.set_object_transform(id, transform).unwrap();
        assert_eq!(document.scene().object(id).unwrap().name(), "Hero");
        assert_eq!(document.scene().object(id).unwrap().transform(), transform);

        assert!(document.undo());
        assert_eq!(
            document.scene().object(id).unwrap().transform(),
            Transform3D::default()
        );
        assert!(document.undo());
        assert_eq!(document.scene().object(id).unwrap().name(), "Cube");
        assert!(document.redo());
        assert_eq!(document.scene().object(id).unwrap().name(), "Hero");
    }

    #[test]
    fn mesh_edits_share_document_undo_and_redo_timeline() {
        let mut document = SceneDocumentEditor::new(Scene::new(), 8);
        let id = document
            .add_primitive("Cube", PrimitiveKind::Cube { size: 1.0 })
            .unwrap();
        let original_position = document
            .scene()
            .object(id)
            .unwrap()
            .mesh()
            .vertex_position(crate::VertexId(0));

        document
            .edit_mesh(id, |mesh| {
                mesh.translate_vertices(&[crate::VertexId(0)], [2.0, 0.0, 0.0])
            })
            .unwrap();
        assert!(!document.scene().object(id).unwrap().is_parametric());
        assert_ne!(
            document
                .scene()
                .object(id)
                .unwrap()
                .mesh()
                .vertex_position(crate::VertexId(0)),
            original_position
        );

        assert!(document.undo());
        assert!(document.scene().object(id).unwrap().is_parametric());
        assert_eq!(
            document
                .scene()
                .object(id)
                .unwrap()
                .mesh()
                .vertex_position(crate::VertexId(0)),
            original_position
        );
        assert!(document.redo());
        assert!(!document.scene().object(id).unwrap().is_parametric());
        assert_ne!(
            document
                .scene()
                .object(id)
                .unwrap()
                .mesh()
                .vertex_position(crate::VertexId(0)),
            original_position
        );
    }

    #[test]
    fn failed_mesh_edit_keeps_scene_and_redo_history_unchanged() {
        let mut document = SceneDocumentEditor::new(Scene::new(), 8);
        let id = document
            .add_primitive("Cube", PrimitiveKind::Cube { size: 1.0 })
            .unwrap();
        document.rename_object(id, "Hero").unwrap();
        assert!(document.undo());
        let before = document
            .scene()
            .object(id)
            .unwrap()
            .mesh()
            .vertex_position(crate::VertexId(0));

        assert!(document
            .edit_mesh(id, |mesh| {
                mesh.translate_vertices(&[crate::VertexId(999)], [1.0, 0.0, 0.0])
            })
            .is_err());
        assert_eq!(
            document
                .scene()
                .object(id)
                .unwrap()
                .mesh()
                .vertex_position(crate::VertexId(0)),
            before
        );
        assert!(document.can_redo());
        assert!(document.scene().object(id).unwrap().is_parametric());
    }

    #[test]
    fn failed_command_preserves_document_and_redo_history() {
        let mut document = SceneDocumentEditor::new(Scene::new(), 8);
        let id = document
            .add_primitive("Cube", PrimitiveKind::Cube { size: 1.0 })
            .unwrap();
        document.rename_object(id, "Hero").unwrap();
        assert!(document.undo());

        assert_eq!(
            document.rename_object(ObjectId(999), "Missing"),
            Err(DocumentEditError::Scene(SceneError::ObjectNotFound(
                ObjectId(999)
            )))
        );
        assert_eq!(document.scene().object(id).unwrap().name(), "Cube");
        assert!(document.can_redo());
    }
}
