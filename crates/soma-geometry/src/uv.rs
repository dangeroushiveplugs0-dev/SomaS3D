use std::collections::HashMap;

use crate::FaceId;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Uv {
    pub u: f32,
    pub v: f32,
}

impl Uv {
    pub const fn new(u: f32, v: f32) -> Self {
        Self { u, v }
    }

    pub fn is_finite(self) -> bool {
        self.u.is_finite() && self.v.is_finite()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UvError {
    FaceNotFound(FaceId),
    CornerOutOfRange { face: FaceId, corner: usize },
    NonFinite,
}

#[derive(Debug, Clone)]
pub struct UvLayer {
    name: String,
    values: HashMap<(FaceId, usize), Uv>,
}

impl UvLayer {
    pub(crate) fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            values: HashMap::new(),
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn set(
        &mut self,
        face: FaceId,
        corner: usize,
        uv: Uv,
    ) -> Result<(), UvError> {
        if !uv.is_finite() {
            return Err(UvError::NonFinite);
        }

        // The mesh owns the authoritative corner count. Mesh::set_uv performs
        // the range check before delegating here.
        self.values.insert((face, corner), uv);
        Ok(())
    }

    pub fn get(&self, face: FaceId, corner: usize) -> Option<Uv> {
        self.values.get(&(face, corner)).copied()
    }

    pub(crate) fn values(&self) -> &HashMap<(FaceId, usize), Uv> {
        &self.values
    }
}
