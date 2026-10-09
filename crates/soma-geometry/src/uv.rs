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

/// Non-destructive parameters for common UV editor transforms.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum UvTransform {
    Translate { delta: [f32; 2] },
    Scale { factor: [f32; 2], pivot: Uv },
    Rotate { radians: f32, pivot: Uv },
}

impl UvTransform {
    pub fn apply(self, uv: Uv) -> Result<Uv, UvError> {
        let result = match self {
            Self::Translate { delta } => {
                if !delta.iter().all(|value| value.is_finite()) {
                    return Err(UvError::NonFinite);
                }
                Uv::new(uv.u + delta[0], uv.v + delta[1])
            }
            Self::Scale { factor, pivot } => {
                if !factor.iter().all(|value| value.is_finite()) || !pivot.is_finite() {
                    return Err(UvError::NonFinite);
                }
                Uv::new(
                    pivot.u + (uv.u - pivot.u) * factor[0],
                    pivot.v + (uv.v - pivot.v) * factor[1],
                )
            }
            Self::Rotate { radians, pivot } => {
                if !radians.is_finite() || !pivot.is_finite() {
                    return Err(UvError::NonFinite);
                }
                let (sin, cos) = radians.sin_cos();
                let u = uv.u - pivot.u;
                let v = uv.v - pivot.v;
                Uv::new(pivot.u + u * cos - v * sin, pivot.v + u * sin + v * cos)
            }
        };
        if result.is_finite() {
            Ok(result)
        } else {
            Err(UvError::NonFinite)
        }
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

#[cfg(test)]
mod transform_tests {
    use super::*;

    #[test]
    fn translate_keeps_uvs_outside_the_unit_square_valid() {
        let result = UvTransform::Translate { delta: [2.0, -1.0] }
            .apply(Uv::new(-0.5, 3.0))
            .unwrap();
        assert_eq!(result, Uv::new(1.5, 2.0));
    }

    #[test]
    fn scale_uses_the_supplied_pivot() {
        let result = UvTransform::Scale {
            factor: [2.0, 0.5],
            pivot: Uv::new(1.0, 1.0),
        }
        .apply(Uv::new(2.0, 3.0))
        .unwrap();
        assert_eq!(result, Uv::new(3.0, 2.0));
    }

    #[test]
    fn rotation_uses_radians_and_preserves_pivot() {
        let pivot = Uv::new(0.5, 0.5);
        let result = UvTransform::Rotate {
            radians: std::f32::consts::FRAC_PI_2,
            pivot,
        }
        .apply(Uv::new(1.0, 0.5))
        .unwrap();
        assert!((result.u - 0.5).abs() < 1.0e-5);
        assert!((result.v - 1.0).abs() < 1.0e-5);
    }

    #[test]
    fn invalid_transform_parameters_are_rejected() {
        assert_eq!(
            UvTransform::Translate {
                delta: [f32::INFINITY, 0.0],
            }
            .apply(Uv::new(0.0, 0.0)),
            Err(UvError::NonFinite)
        );
    }
}
