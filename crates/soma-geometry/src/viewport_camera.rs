//! Renderer-neutral camera and projection math for the interactive viewport.
//!
//! Matrices use row-major storage and multiply column vectors. The perspective
//! projection follows the right-handed OpenGL clip-space convention (NDC z in
//! -1..=1), which render backends can adapt to their native clip convention.

/// Camera pose and perspective lens settings.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ViewportCamera {
    pub eye: [f32; 3],
    pub target: [f32; 3],
    pub up: [f32; 3],
    /// Vertical field of view in radians.
    pub vertical_fov_radians: f32,
    pub near_plane: f32,
    pub far_plane: f32,
}

impl Default for ViewportCamera {
    fn default() -> Self {
        Self {
            eye: [4.0, 3.0, 6.0],
            target: [0.0, 0.0, 0.0],
            up: [0.0, 1.0, 0.0],
            vertical_fov_radians: 50.0_f32.to_radians(),
            near_plane: 0.01,
            far_plane: 10_000.0,
        }
    }
}

/// Validated view, projection, and combined matrices.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ViewportCameraMatrices {
    /// World-to-camera transform.
    pub view: [[f32; 4]; 4],
    /// Camera-to-clip perspective transform.
    pub projection: [[f32; 4]; 4],
    /// World-to-clip transform, projection multiplied by view.
    pub view_projection: [[f32; 4]; 4],
}

/// Camera errors are returned instead of allowing NaNs into render data.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewportCameraError {
    NonFiniteInput,
    InvalidAspectRatio,
    InvalidFieldOfView,
    InvalidClipPlanes,
    DegenerateViewDirection,
    DegenerateUpDirection,
}

impl ViewportCamera {
    /// Builds matrices for a positive viewport width/height aspect ratio.
    pub fn matrices(self, aspect_ratio: f32) -> Result<ViewportCameraMatrices, ViewportCameraError> {
        if !self.eye.iter().chain(self.target.iter()).chain(self.up.iter()).all(|v| v.is_finite())
            || !self.vertical_fov_radians.is_finite()
            || !self.near_plane.is_finite()
            || !self.far_plane.is_finite()
        {
            return Err(ViewportCameraError::NonFiniteInput);
        }
        if !aspect_ratio.is_finite() || aspect_ratio <= 0.0 {
            return Err(ViewportCameraError::InvalidAspectRatio);
        }
        if self.vertical_fov_radians <= 0.0 || self.vertical_fov_radians >= std::f32::consts::PI {
            return Err(ViewportCameraError::InvalidFieldOfView);
        }
        if self.near_plane <= 0.0 || self.far_plane <= self.near_plane {
            return Err(ViewportCameraError::InvalidClipPlanes);
        }

        let forward = normalize(sub(self.target, self.eye))
            .ok_or(ViewportCameraError::DegenerateViewDirection)?;
        let right = normalize(cross(forward, self.up))
            .ok_or(ViewportCameraError::DegenerateUpDirection)?;
        let camera_up = cross(right, forward);

        let view = [
            [right[0], right[1], right[2], -dot(right, self.eye)],
            [camera_up[0], camera_up[1], camera_up[2], -dot(camera_up, self.eye)],
            [-forward[0], -forward[1], -forward[2], dot(forward, self.eye)],
            [0.0, 0.0, 0.0, 1.0],
        ];

        let f = 1.0 / (self.vertical_fov_radians * 0.5).tan();
        let nf = 1.0 / (self.near_plane - self.far_plane);
        let projection = [
            [f / aspect_ratio, 0.0, 0.0, 0.0],
            [0.0, f, 0.0, 0.0],
            [0.0, 0.0, (self.far_plane + self.near_plane) * nf, 2.0 * self.far_plane * self.near_plane * nf],
            [0.0, 0.0, -1.0, 0.0],
        ];
        let view_projection = multiply(projection, view);

        Ok(ViewportCameraMatrices { view, projection, view_projection })
    }
}

impl ViewportCameraMatrices {
    /// Projects a world-space point into normalized device coordinates.
    /// Returns None for non-finite points or clip-space w effectively zero.
    /// Callers can test NDC z against -1..=1 for clipping.
    pub fn project_world_to_ndc(&self, point: [f32; 3]) -> Option<[f32; 3]> {
        if !point.iter().all(|v| v.is_finite()) {
            return None;
        }
        let clip = multiply_vector(self.view_projection, [point[0], point[1], point[2], 1.0]);
        if !clip.iter().all(|v| v.is_finite()) || clip[3].abs() <= f32::EPSILON {
            return None;
        }
        Some([clip[0] / clip[3], clip[1] / clip[3], clip[2] / clip[3]])
    }

    /// Converts NDC coordinates into top-left-origin pixel coordinates.
    pub fn ndc_to_screen(ndc: [f32; 3], width: u32, height: u32) -> Option<[f32; 3]> {
        if width == 0 || height == 0 || !ndc.iter().all(|v| v.is_finite()) {
            return None;
        }
        Some([
            (ndc[0] + 1.0) * 0.5 * width as f32,
            (1.0 - ndc[1]) * 0.5 * height as f32,
            ndc[2],
        ])
    }
}

fn sub(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
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

fn normalize(v: [f32; 3]) -> Option<[f32; 3]> {
    let length = dot(v, v).sqrt();
    if !length.is_finite() || length <= f32::EPSILON {
        None
    } else {
        Some([v[0] / length, v[1] / length, v[2] / length])
    }
}

fn multiply(a: [[f32; 4]; 4], b: [[f32; 4]; 4]) -> [[f32; 4]; 4] {
    let mut result = [[0.0; 4]; 4];
    for row in 0..4 {
        for column in 0..4 {
            for inner in 0..4 {
                result[row][column] += a[row][inner] * b[inner][column];
            }
        }
    }
    result
}

fn multiply_vector(matrix: [[f32; 4]; 4], vector: [f32; 4]) -> [f32; 4] {
    let mut result = [0.0; 4];
    for row in 0..4 {
        for column in 0..4 {
            result[row] += matrix[row][column] * vector[column];
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn target_projects_to_viewport_center() {
        let camera = ViewportCamera {
            eye: [0.0, 0.0, 5.0],
            target: [0.0, 0.0, 0.0],
            ..ViewportCamera::default()
        };
        let matrices = camera.matrices(16.0 / 9.0).unwrap();
        let ndc = matrices.project_world_to_ndc([0.0, 0.0, 0.0]).unwrap();
        assert!(ndc[0].abs() < 1e-6);
        assert!(ndc[1].abs() < 1e-6);
        assert!((-1.0..1.0).contains(&ndc[2]));
        let screen = ViewportCameraMatrices::ndc_to_screen(ndc, 1600, 900).unwrap();
        assert!((screen[0] - 800.0).abs() < 1e-3);
        assert!((screen[1] - 450.0).abs() < 1e-3);
    }

    #[test]
    fn rejects_invalid_lens_and_degenerate_pose() {
        assert_eq!(
            ViewportCamera::default().matrices(0.0),
            Err(ViewportCameraError::InvalidAspectRatio)
        );
        let camera = ViewportCamera {
            target: [4.0, 3.0, 6.0],
            ..ViewportCamera::default()
        };
        assert_eq!(
            camera.matrices(1.0),
            Err(ViewportCameraError::DegenerateViewDirection)
        );
    }

    #[test]
    fn screen_conversion_rejects_empty_viewports() {
        assert_eq!(ViewportCameraMatrices::ndc_to_screen([0.0, 0.0, 0.0], 0, 10), None);
    }
}
