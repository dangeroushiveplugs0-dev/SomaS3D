//! Touch-friendly camera navigation and fit-to-bounds helpers.
//!
//! Input is expressed in logical pixels/radians so a platform UI can map touch
//! gestures to these deterministic, renderer-independent operations.

use crate::{ViewportCamera, ViewportCameraError};

/// Axis-aligned bounds in world space.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ViewportBounds {
    pub min: [f32; 3],
    pub max: [f32; 3],
}

/// Stateful navigation helper around a perspective viewport camera.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ViewportNavigation {
    pub camera: ViewportCamera,
    pub min_distance: f32,
    pub max_distance: f32,
}

impl Default for ViewportNavigation {
    fn default() -> Self {
        Self {
            camera: ViewportCamera::default(),
            min_distance: 0.01,
            max_distance: 1_000_000.0,
        }
    }
}

impl ViewportNavigation {
    /// Orbits around the target. Yaw rotates around world Y; pitch is clamped
    /// away from the poles to avoid a sudden camera-up inversion.
    pub fn orbit(&mut self, yaw_delta_radians: f32, pitch_delta_radians: f32) -> bool {
        if !yaw_delta_radians.is_finite() || !pitch_delta_radians.is_finite() {
            return false;
        }
        let offset = subtract(self.camera.eye, self.camera.target);
        let distance = length(offset);
        if !distance.is_finite() || distance <= f32::EPSILON {
            return false;
        }
        let yaw = offset[0].atan2(offset[2]) + yaw_delta_radians;
        let pitch = (offset[1] / distance).clamp(-1.0, 1.0).asin();
        let pitch = (pitch + pitch_delta_radians).clamp(-1.553_343, 1.553_343);
        let horizontal = distance * pitch.cos();
        self.camera.eye = add(
            self.camera.target,
            [
                horizontal * yaw.sin(),
                distance * pitch.sin(),
                horizontal * yaw.cos(),
            ],
        );
        true
    }

    /// Pans parallel to the camera plane. Pixel deltas are scaled by the
    /// viewport height and current target distance for predictable touch use.
    pub fn pan_pixels(&mut self, delta_x: f32, delta_y: f32, viewport_height: u32) -> bool {
        if !delta_x.is_finite() || !delta_y.is_finite() || viewport_height == 0 {
            return false;
        }
        let offset = subtract(self.camera.target, self.camera.eye);
        let distance = length(offset);
        let Some(forward) = normalize(offset) else {
            return false;
        };
        let Some(right) = normalize(cross(forward, self.camera.up)) else {
            return false;
        };
        let up = cross(right, forward);
        let world_per_pixel = 2.0 * distance * (self.camera.vertical_fov_radians * 0.5).tan()
            / viewport_height as f32;
        let shift = add(
            scale(right, -delta_x * world_per_pixel),
            scale(up, delta_y * world_per_pixel),
        );
        self.camera.eye = add(self.camera.eye, shift);
        self.camera.target = add(self.camera.target, shift);
        true
    }

    /// Applies a multiplicative zoom. Values above 1 zoom in; values below 1
    /// zoom out. Distance is clamped to configured limits.
    pub fn zoom(&mut self, zoom_factor: f32) -> bool {
        if !zoom_factor.is_finite() || zoom_factor <= 0.0 {
            return false;
        }
        let offset = subtract(self.camera.eye, self.camera.target);
        let distance = length(offset);
        if !distance.is_finite() || distance <= f32::EPSILON {
            return false;
        }
        let next_distance = (distance / zoom_factor).clamp(self.min_distance, self.max_distance);
        self.camera.eye = add(self.camera.target, scale(offset, next_distance / distance));
        true
    }

    /// Frames an axis-aligned world-space bound while preserving camera view
    /// direction. Margin should be at least 1; 1.15 leaves breathing room.
    pub fn fit_bounds(
        &mut self,
        bounds: ViewportBounds,
        aspect_ratio: f32,
        margin: f32,
    ) -> Result<(), ViewportCameraError> {
        if !bounds
            .min
            .iter()
            .chain(bounds.max.iter())
            .all(|v| v.is_finite())
            || !margin.is_finite()
            || margin < 1.0
        {
            return Err(ViewportCameraError::NonFiniteInput);
        }
        if (0..3).any(|axis| bounds.min[axis] > bounds.max[axis]) {
            return Err(ViewportCameraError::NonFiniteInput);
        }
        self.camera.matrices(aspect_ratio)?;
        let center = scale(add(bounds.min, bounds.max), 0.5);
        let extent = scale(subtract(bounds.max, bounds.min), 0.5);
        let radius = length(extent);
        let direction =
            normalize(subtract(self.camera.eye, self.camera.target)).unwrap_or([0.0, 0.0, 1.0]);
        let horizontal_fov =
            2.0 * ((self.camera.vertical_fov_radians * 0.5).tan() * aspect_ratio).atan();
        let limiting_fov = self.camera.vertical_fov_radians.min(horizontal_fov);
        let distance = if radius <= f32::EPSILON {
            self.min_distance
        } else {
            (radius / (limiting_fov * 0.5).sin() * margin)
                .clamp(self.min_distance, self.max_distance)
        };
        self.camera.target = center;
        self.camera.eye = add(center, scale(direction, distance));
        Ok(())
    }
}

fn add(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

fn subtract(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn scale(v: [f32; 3], factor: f32) -> [f32; 3] {
    [v[0] * factor, v[1] * factor, v[2] * factor]
}

fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn length(v: [f32; 3]) -> f32 {
    dot(v, v).sqrt()
}

fn normalize(v: [f32; 3]) -> Option<[f32; 3]> {
    let magnitude = length(v);
    if !magnitude.is_finite() || magnitude <= f32::EPSILON {
        None
    } else {
        Some(scale(v, 1.0 / magnitude))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn orbit_preserves_distance_and_keeps_target_fixed() {
        let mut navigation = ViewportNavigation::default();
        let target = navigation.camera.target;
        let distance = length(subtract(navigation.camera.eye, target));
        assert!(navigation.orbit(0.5, 0.2));
        assert_eq!(navigation.camera.target, target);
        assert!((length(subtract(navigation.camera.eye, target)) - distance).abs() < 1e-4);
    }

    #[test]
    fn zoom_clamps_distance_and_rejects_invalid_factors() {
        let mut navigation = ViewportNavigation {
            min_distance: 1.0,
            max_distance: 20.0,
            ..ViewportNavigation::default()
        };
        assert!(navigation.zoom(1000.0));
        assert!(
            (length(subtract(navigation.camera.eye, navigation.camera.target)) - 1.0).abs() < 1e-4
        );
        assert!(!navigation.zoom(f32::NAN));
    }

    #[test]
    fn pan_moves_eye_and_target_together() {
        let mut navigation = ViewportNavigation::default();
        let offset = subtract(navigation.camera.eye, navigation.camera.target);
        assert!(navigation.pan_pixels(20.0, -10.0, 800));
        assert_eq!(
            subtract(navigation.camera.eye, navigation.camera.target),
            offset
        );
    }

    #[test]
    fn fit_bounds_centers_camera_and_rejects_inverted_bounds() {
        let mut navigation = ViewportNavigation::default();
        navigation
            .fit_bounds(
                ViewportBounds {
                    min: [-1.0, -2.0, -3.0],
                    max: [1.0, 2.0, 3.0],
                },
                16.0 / 9.0,
                1.15,
            )
            .unwrap();
        assert_eq!(navigation.camera.target, [0.0, 0.0, 0.0]);
        assert!(length(subtract(navigation.camera.eye, navigation.camera.target)) > 3.0);
        assert!(navigation
            .fit_bounds(
                ViewportBounds {
                    min: [1.0, 0.0, 0.0],
                    max: [-1.0, 1.0, 1.0],
                },
                1.0,
                1.0,
            )
            .is_err());
    }
}
