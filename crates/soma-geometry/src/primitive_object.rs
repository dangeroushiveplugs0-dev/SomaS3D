//! Editable primitive-object descriptors.
//!
//! A primitive object retains its source parameters alongside its generated
//! mesh, allowing an editor to regenerate geometry without replacing the
//! object's identity. Scene storage and UI controls live in higher layers.

use crate::{generate_primitive, Mesh, PrimitiveError, PrimitiveKind};

/// Stable identity assigned by the future scene/document layer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PrimitiveObjectId(pub u64);

/// Failure while creating or editing a parameterized primitive object.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PrimitiveObjectError {
    Generation(PrimitiveError),
    RevisionOverflow,
}

impl From<PrimitiveError> for PrimitiveObjectError {
    fn from(value: PrimitiveError) -> Self {
        Self::Generation(value)
    }
}

/// A standard primitive's identity, editable source parameters, and mesh.
///
/// The ID is supplied by the caller so scene/document code can own the ID
/// allocation policy. Geometry edits are transactional: invalid parameters
/// never replace the last valid mesh or update the revision.
#[derive(Debug, Clone)]
pub struct PrimitiveObject {
    id: PrimitiveObjectId,
    name: String,
    kind: PrimitiveKind,
    mesh: Mesh,
    geometry_revision: u64,
}

impl PrimitiveObject {
    /// Creates a primitive object and generates its initial mesh.
    pub fn new(
        id: PrimitiveObjectId,
        name: impl Into<String>,
        kind: PrimitiveKind,
    ) -> Result<Self, PrimitiveObjectError> {
        let mesh = generate_primitive(kind)?;
        Ok(Self {
            id,
            name: name.into(),
            kind,
            mesh,
            geometry_revision: 0,
        })
    }

    pub fn id(&self) -> PrimitiveObjectId {
        self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    /// Renaming does not regenerate geometry or change its revision.
    pub fn rename(&mut self, name: impl Into<String>) {
        self.name = name.into();
    }

    /// Returns the editable source parameters used to build this object.
    pub fn kind(&self) -> PrimitiveKind {
        self.kind
    }

    /// Returns the last successfully generated mesh without exposing mutable
    /// access that could desynchronize it from the retained parameters.
    pub fn mesh(&self) -> &Mesh {
        &self.mesh
    }

    /// Monotonically increasing revision for successful geometry changes.
    pub fn geometry_revision(&self) -> u64 {
        self.geometry_revision
    }

    /// Updates parameters and regenerates geometry while preserving object ID.
    ///
    /// The new mesh is generated before any state is committed. Reapplying
    /// identical parameters is a no-op, which avoids needless rebuilds from
    /// UI controls that emit unchanged values.
    pub fn set_kind(&mut self, kind: PrimitiveKind) -> Result<(), PrimitiveObjectError> {
        if self.kind == kind {
            return Ok(());
        }

        let next_revision = self
            .geometry_revision
            .checked_add(1)
            .ok_or(PrimitiveObjectError::RevisionOverflow)?;
        let mesh = generate_primitive(kind)?;

        self.kind = kind;
        self.mesh = mesh;
        self.geometry_revision = next_revision;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::VertexId;

    #[test]
    fn parameter_edit_regenerates_mesh_and_preserves_object_identity() {
        let mut object = PrimitiveObject::new(
            PrimitiveObjectId(42),
            "Sphere",
            PrimitiveKind::UvSphere {
                radius: 1.0,
                segments: 8,
                rings: 4,
            },
        )
        .unwrap();
        assert_eq!(object.mesh().vertex_position(VertexId(0)), Some([0.0, 1.0, 0.0]));

        object
            .set_kind(PrimitiveKind::UvSphere {
                radius: 2.5,
                segments: 8,
                rings: 4,
            })
            .unwrap();

        assert_eq!(object.id(), PrimitiveObjectId(42));
        assert_eq!(object.name(), "Sphere");
        assert_eq!(object.geometry_revision(), 1);
        assert_eq!(object.mesh().vertex_position(VertexId(0)), Some([0.0, 2.5, 0.0]));
        assert_eq!(
            object.kind(),
            PrimitiveKind::UvSphere {
                radius: 2.5,
                segments: 8,
                rings: 4,
            }
        );
    }

    #[test]
    fn invalid_parameter_edit_keeps_last_valid_state() {
        let mut object = PrimitiveObject::new(
            PrimitiveObjectId(7),
            "Cube",
            PrimitiveKind::Cube { size: 2.0 },
        )
        .unwrap();
        let old_vertex_count = object.mesh().vertex_count();
        let old_position = object.mesh().vertex_position(VertexId(0));

        assert_eq!(
            object.set_kind(PrimitiveKind::Cube { size: f32::NAN }),
            Err(PrimitiveObjectError::Generation(
                PrimitiveError::NonFiniteDimension
            ))
        );
        assert_eq!(object.kind(), PrimitiveKind::Cube { size: 2.0 });
        assert_eq!(object.mesh().vertex_count(), old_vertex_count);
        assert_eq!(object.mesh().vertex_position(VertexId(0)), old_position);
        assert_eq!(object.geometry_revision(), 0);
    }

    #[test]
    fn identical_parameters_do_not_bump_geometry_revision() {
        let mut object = PrimitiveObject::new(
            PrimitiveObjectId(1),
            "Plane",
            PrimitiveKind::Plane {
                width: 2.0,
                depth: 3.0,
            },
        )
        .unwrap();
        object
            .set_kind(PrimitiveKind::Plane {
                width: 2.0,
                depth: 3.0,
            })
            .unwrap();
        assert_eq!(object.geometry_revision(), 0);
    }

    #[test]
    fn rename_preserves_identity_and_geometry_revision() {
        let mut object = PrimitiveObject::new(
            PrimitiveObjectId(9),
            "Cube",
            PrimitiveKind::Cube { size: 1.0 },
        )
        .unwrap();
        object.rename("Main Cube");
        assert_eq!(object.id(), PrimitiveObjectId(9));
        assert_eq!(object.name(), "Main Cube");
        assert_eq!(object.geometry_revision(), 0);
    }
}
