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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct UvCorner {
    pub face: FaceId,
    pub corner: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UvError {
    FaceNotFound(FaceId),
    CornerOutOfRange { face: FaceId, corner: usize },
    NonFinite,
    MissingCoordinate(UvCorner),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UvIsland {
    /// Faces connected in UV space without crossing seams or UV discontinuities.
    pub faces: Vec<FaceId>,
    /// Every face corner in the island; shared 3D vertices remain distinct corners.
    pub corners: Vec<UvCorner>,
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

    pub fn set(&mut self, face: FaceId, corner: usize, uv: Uv) -> Result<(), UvError> {
        if !uv.is_finite() {
            return Err(UvError::NonFinite);
        }
        self.values.insert((face, corner), uv);
        Ok(())
    }

    pub fn get(&self, face: FaceId, corner: usize) -> Option<Uv> {
        self.values.get(&(face, corner)).copied()
    }

    pub fn corners(&self) -> impl Iterator<Item = UvCorner> + '_ {
        self.values
            .keys()
            .map(|&(face, corner)| UvCorner { face, corner })
    }
}
