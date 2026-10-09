//! Scene-aware modeling commands. Each object's local mesh has its own undo history.

use std::collections::HashMap;

use crate::{
    EditorError, FaceId, ModelingEditor, ObjectId, Scene, SceneError, SelectionMode, Transform3D,
    VertexId,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SceneModelingError {
    Scene(SceneError),
    Editor(EditorError),
    NoActiveObject,
}

impl From<SceneError> for SceneModelingError {
    fn from(value: SceneError) -> Self {
        Self::Scene(value)
    }
}
impl From<EditorError> for SceneModelingError {
    fn from(value: EditorError) -> Self {
        Self::Editor(value)
    }
}

/// Coordinates existing mesh-edit commands with scene object identity.
/// Selection and undo/redo are kept independently for each object.
#[derive(Debug, Clone)]
pub struct SceneModelingEditor {
    scene: Scene,
    editors: HashMap<ObjectId, ModelingEditor>,
    undo_limit: usize,
}

impl SceneModelingEditor {
    pub fn new(scene: Scene, undo_limit: usize) -> Self {
        Self {
            scene,
            editors: HashMap::new(),
            undo_limit,
        }
    }

    pub fn scene(&self) -> &Scene {
        &self.scene
    }
    pub fn scene_mut(&mut self) -> &mut Scene {
        &mut self.scene
    }

    pub fn active_object_id(&self) -> Option<ObjectId> {
        self.scene.active_object_id()
    }

    pub fn set_active_object(&mut self, id: Option<ObjectId>) -> Result<(), SceneModelingError> {
        self.scene.set_active_object(id)?;
        Ok(())
    }

    fn ensure_active_editor(&mut self) -> Result<ObjectId, SceneModelingError> {
        let id = self.scene.active_object_id().ok_or(SceneModelingError::NoActiveObject)?;
        if !self.editors.contains_key(&id) {
            let mesh = self.scene.object(id).ok_or(SceneError::ObjectNotFound(id))?.mesh().clone();
            self.editors.insert(id, ModelingEditor::new(mesh, self.undo_limit));
        }
        Ok(id)
    }

    fn sync_active_mesh(&mut self, id: ObjectId) -> Result<(), SceneModelingError> {
        let mesh = self
            .editors
            .get(&id)
            .ok_or(SceneError::ObjectNotFound(id))?
            .state()
            .mesh()
            .clone();
        let object = self.scene.object_mut(id).ok_or(SceneError::ObjectNotFound(id))?;
        object.commit_edited_mesh(mesh);
        Ok(())
    }

    pub fn set_selection_mode(&mut self, mode: SelectionMode) -> Result<(), SceneModelingError> {
        let id = self.ensure_active_editor()?;
        self.editors
            .get_mut(&id)
            .expect("active editor was ensured")
            .set_selection_mode(mode);
        Ok(())
    }

    pub fn select_vertex(&mut self, vertex: VertexId) -> Result<(), SceneModelingError> {
        let id = self.ensure_active_editor()?;
        self.editors
            .get_mut(&id)
            .expect("active editor was ensured")
            .select_vertex(vertex);
        Ok(())
    }

    pub fn select_face(&mut self, face: FaceId) -> Result<(), SceneModelingError> {
        let id = self.ensure_active_editor()?;
        self.editors
            .get_mut(&id)
            .expect("active editor was ensured")
            .select_face(face)?;
        Ok(())
    }

    pub fn transform_selected_vertices(&mut self, transform: Transform3D) -> Result<(), SceneModelingError> {
        let id = self.ensure_active_editor()?;
        self.editors
            .get_mut(&id)
            .expect("active editor was ensured")
            .transform_selected_vertices(transform)?;
        self.sync_active_mesh(id)
    }

    pub fn extrude_selected_face(&mut self, offset: [f32; 3]) -> Result<(), SceneModelingError> {
        let id = self.ensure_active_editor()?;
        self.editors
            .get_mut(&id)
            .expect("active editor was ensured")
            .extrude_selected_face(offset)?;
        self.sync_active_mesh(id)
    }

    pub fn delete_selected_faces(&mut self) -> Result<(), SceneModelingError> {
        let id = self.ensure_active_editor()?;
        self.editors
            .get_mut(&id)
            .expect("active editor was ensured")
            .delete_selected_faces()?;
        self.sync_active_mesh(id)
    }

    pub fn can_undo(&mut self) -> Result<bool, SceneModelingError> {
        let id = self.ensure_active_editor()?;
        Ok(self
            .editors
            .get(&id)
            .expect("active editor was ensured")
            .can_undo())
    }

    pub fn can_redo(&mut self) -> Result<bool, SceneModelingError> {
        let id = self.ensure_active_editor()?;
        Ok(self
            .editors
            .get(&id)
            .expect("active editor was ensured")
            .can_redo())
    }

    pub fn undo(&mut self) -> Result<bool, SceneModelingError> {
        let id = self.ensure_active_editor()?;
        let changed = self
            .editors
            .get_mut(&id)
            .expect("active editor was ensured")
            .undo();
        if changed {
            self.sync_active_mesh(id)?;
        }
        Ok(changed)
    }

    pub fn redo(&mut self) -> Result<bool, SceneModelingError> {
        let id = self.ensure_active_editor()?;
        let changed = self
            .editors
            .get_mut(&id)
            .expect("active editor was ensured")
            .redo();
        if changed {
            self.sync_active_mesh(id)?;
        }
        Ok(changed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{PrimitiveKind, VertexId};

    #[test]
    fn active_object_mesh_edits_preserve_object_transform_and_undo() {
        let mut scene = Scene::new();
        let id = scene
            .add_primitive("Cube", PrimitiveKind::Cube { size: 2.0 })
            .unwrap();
        let transform = Transform3D {
            translation: [4.0, 5.0, 6.0],
            ..Transform3D::default()
        };
        scene.object_mut(id).unwrap().set_transform(transform);
        let mut editor = SceneModelingEditor::new(scene, 8);
        let before = editor.scene().object(id).unwrap().mesh().vertex_position(VertexId(0)).unwrap();

        editor.select_vertex(VertexId(0)).unwrap();
        editor
            .transform_selected_vertices(Transform3D {
                translation: [0.0, 0.0, 1.0],
                ..Transform3D::default()
            })
            .unwrap();
        assert_eq!(editor.scene().object(id).unwrap().mesh().vertex_position(VertexId(0)), Some([before[0], before[1], before[2] + 1.0]));
        assert_eq!(editor.scene().object(id).unwrap().transform(), transform);
        assert!(!editor.scene().object(id).unwrap().is_parametric());

        assert!(editor.undo().unwrap());
        assert_eq!(editor.scene().object(id).unwrap().mesh().vertex_position(VertexId(0)), Some(before));
        assert_eq!(editor.scene().object(id).unwrap().transform(), transform);
        assert!(editor.redo().unwrap());
        assert_eq!(editor.scene().object(id).unwrap().mesh().vertex_position(VertexId(0)), Some([before[0], before[1], before[2] + 1.0]));
    }

    #[test]
    fn each_object_keeps_its_own_edit_history() {
        let mut scene = Scene::new();
        let first = scene.add_primitive("A", PrimitiveKind::Cube { size: 1.0 }).unwrap();
        let second = scene.add_primitive("B", PrimitiveKind::Cube { size: 1.0 }).unwrap();
        let mut editor = SceneModelingEditor::new(scene, 8);
        editor.select_vertex(VertexId(0)).unwrap();
        editor
            .transform_selected_vertices(Transform3D {
                translation: [0.0, 0.0, 2.0],
                ..Transform3D::default()
            })
            .unwrap();
        editor.set_active_object(Some(first)).unwrap();
        assert!(editor.can_undo().unwrap());
        editor.set_active_object(Some(second)).unwrap();
        assert!(!editor.can_undo().unwrap());
        assert_eq!(editor.scene().object(first).unwrap().mesh().vertex_position(VertexId(0)), Some([-0.5, -0.5, 1.5]));
    }
}
