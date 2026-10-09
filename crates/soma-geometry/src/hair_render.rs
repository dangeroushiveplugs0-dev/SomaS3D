//! Viewport-facing evaluated hair guide data.
//!
//! This module converts source-local hair guides into world-space polylines.
//! It stays renderer-agnostic: a viewport can upload these lines to its GPU
//! backend without knowing how hair is stored in the scene document.

use crate::{HairError, ObjectId, Scene, SceneError};

/// World-space guide polylines and styling data ready for viewport submission.
#[derive(Debug, Clone, PartialEq)]
pub struct EvaluatedHairGuides {
    pub object_id: ObjectId,
    pub name: String,
    pub color: [f32; 4],
    /// One world-space polyline per strand; each contains root through tip.
    pub guides: Vec<Vec<[f32; 3]>>,
}

impl Scene {
    /// Evaluates hair guides through their source object's transform.
    ///
    /// The scene's stored guides remain in local space. This returns a
    /// detached render snapshot, so render preparation cannot mutate document
    /// geometry or accidentally apply an object's transform twice.
    pub fn evaluated_hair_guides(
        &self,
        hair_id: ObjectId,
    ) -> Result<EvaluatedHairGuides, SceneError> {
        let hair_object = self
            .hair_object(hair_id)
            .ok_or(SceneError::ObjectNotFound(hair_id))?;
        let source = self
            .object(hair_object.source_object())
            .ok_or(SceneError::ObjectNotFound(hair_object.source_object()))?;
        let transform = source.transform();
        let hair = hair_object.hair();

        let mut guides = Vec::with_capacity(hair.guides().len());
        for guide in hair.guides() {
            let mut points = Vec::with_capacity(guide.points.len());
            for &point in &guide.points {
                points.push(transform.apply(point)?);
            }
            guides.push(points);
        }

        Ok(EvaluatedHairGuides {
            object_id: hair_id,
            name: hair_object.name().to_owned(),
            color: hair.settings().color,
            guides,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{FaceId, HairPreset, HairSettings, Mesh, Transform3D};

    fn scene_with_hair() -> (Scene, ObjectId, ObjectId) {
        let mut mesh = Mesh::new();
        let a = mesh.add_vertex([0.0, 0.0, 0.0]);
        let b = mesh.add_vertex([1.0, 0.0, 0.0]);
        let c = mesh.add_vertex([0.0, 0.0, 1.0]);
        let face = mesh.add_face(&[a, c, b]).unwrap();
        let mut scene = Scene::new();
        let source = scene.add_mesh("Character", mesh).unwrap();
        let hair = scene
            .add_hair_object(
                "Scalp Hair",
                source,
                &[face],
                HairPreset::FlowingHair,
                HairSettings::flowing_hair(),
            )
            .unwrap();
        (scene, source, hair)
    }

    #[test]
    fn evaluated_guides_apply_source_transform_and_keep_local_guides_unchanged() {
        let (mut scene, source, hair_id) = scene_with_hair();
        let local_before = scene.hair_object(hair_id).unwrap().hair().guides()[0]
            .points
            .clone();
        scene.object_mut(source).unwrap().set_transform(Transform3D {
            translation: [3.0, -2.0, 5.0],
            ..Transform3D::default()
        });

        let evaluated = scene.evaluated_hair_guides(hair_id).unwrap();

        assert_eq!(evaluated.object_id, hair_id);
        assert_eq!(evaluated.name, "Scalp Hair");
        assert_eq!(evaluated.guides.len(), 512);
        assert_eq!(evaluated.guides[0].len(), local_before.len());
        assert_eq!(
            evaluated.guides[0][0],
            [local_before[0][0] + 3.0, local_before[0][1] - 2.0, local_before[0][2] + 5.0]
        );
        assert_eq!(
            scene.hair_object(hair_id).unwrap().hair().guides()[0].points,
            local_before
        );
    }

    #[test]
    fn evaluated_guides_preserve_hair_color_for_viewport_styling() {
        let (scene, _, hair_id) = scene_with_hair();
        let evaluated = scene.evaluated_hair_guides(hair_id).unwrap();
        assert_eq!(evaluated.color, HairSettings::flowing_hair().color);
    }

    #[test]
    fn invalid_source_transform_fails_evaluation_without_changing_guides() {
        let (mut scene, source, hair_id) = scene_with_hair();
        scene.object_mut(source).unwrap().set_transform(Transform3D {
            translation: [f32::NAN, 0.0, 0.0],
            ..Transform3D::default()
        });
        assert_eq!(
            scene.evaluated_hair_guides(hair_id),
            Err(SceneError::Mesh(crate::MeshError::NonFinitePosition))
        );
        assert_eq!(scene.hair_object(hair_id).unwrap().hair().guides().len(), 512);
    }

    #[test]
    fn missing_hair_id_is_reported() {
        let (scene, _, _) = scene_with_hair();
        assert_eq!(
            scene.evaluated_hair_guides(ObjectId(999)),
            Err(SceneError::ObjectNotFound(ObjectId(999)))
        );
    }

    #[allow(dead_code)]
    fn face_id_type_is_available_to_downstream_render_adapters(_: FaceId) {}
}
