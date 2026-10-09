//! Screen-space ray construction and CPU triangle picking for the viewport.
//!
//! This keeps selection independent of any GPU API. A UI can turn a tap into a
//! ray, then use the latest immutable draw packet to identify the nearest mesh
//! triangle and world-space hit point.

use crate::{ObjectId, ViewportCameraMatrices, ViewportDrawData};

/// Normalized world-space ray. Direction is always unit length.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ViewportRay {
    pub origin: [f32; 3],
    pub direction: [f32; 3],
}

/// Nearest mesh triangle intersected by a viewport ray.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ViewportPickHit {
    pub object_id: ObjectId,
    /// Zero-based triangle number within this object's triangle index buffer.
    pub triangle_index: usize,
    /// World-space hit location.
    pub position: [f32; 3],
    /// Distance from ray origin to the hit point.
    pub distance: f32,
    /// Barycentric weights corresponding to the triangle's three vertices.
    pub barycentric: [f32; 3],
}

impl ViewportCameraMatrices {
    /// Creates a world-space ray through a pixel in a top-left-origin viewport.
    ///
    /// Pixels outside the viewport and singular/non-finite matrices return None.
    pub fn screen_ray(
        &self,
        pixel_x: f32,
        pixel_y: f32,
        width: u32,
        height: u32,
    ) -> Option<ViewportRay> {
        if width == 0
            || height == 0
            || !pixel_x.is_finite()
            || !pixel_y.is_finite()
            || pixel_x < 0.0
            || pixel_y < 0.0
            || pixel_x >= width as f32
            || pixel_y >= height as f32
        {
            return None;
        }
        let inverse = invert(self.view_projection)?;
        let x = 2.0 * pixel_x / width as f32 - 1.0;
        let y = 1.0 - 2.0 * pixel_y / height as f32;
        let near = unproject(inverse, [x, y, -1.0])?;
        let far = unproject(inverse, [x, y, 1.0])?;
        let direction = normalize(subtract(far, near))?;
        Some(ViewportRay {
            origin: near,
            direction,
        })
    }
}

impl ViewportDrawData {
    /// Returns the nearest surface triangle hit, ignoring malformed triangles.
    ///
    /// This is CPU-side object/component picking. Hair guide picking and edge/
    /// vertex screen-space tolerances can be layered on top separately.
    pub fn pick_mesh(&self, ray: ViewportRay) -> Option<ViewportPickHit> {
        if !ray.origin.iter().all(|v| v.is_finite())
            || !ray.direction.iter().all(|v| v.is_finite())
            || (length(ray.direction) - 1.0).abs() > 1.0e-3
        {
            return None;
        }

        let mut nearest: Option<ViewportPickHit> = None;
        for mesh in &self.meshes {
            for (triangle_index, triangle) in mesh.triangle_indices.chunks_exact(3).enumerate() {
                let Some(&a) = mesh.positions.get(triangle[0] as usize) else {
                    continue;
                };
                let Some(&b) = mesh.positions.get(triangle[1] as usize) else {
                    continue;
                };
                let Some(&c) = mesh.positions.get(triangle[2] as usize) else {
                    continue;
                };
                if !a
                    .iter()
                    .chain(b.iter())
                    .chain(c.iter())
                    .all(|v| v.is_finite())
                {
                    continue;
                }
                let Some((distance, barycentric)) = intersect_triangle(ray, a, b, c) else {
                    continue;
                };
                if nearest.is_some_and(|hit| hit.distance <= distance) {
                    continue;
                }
                nearest = Some(ViewportPickHit {
                    object_id: mesh.object_id,
                    triangle_index,
                    position: add(ray.origin, scale(ray.direction, distance)),
                    distance,
                    barycentric,
                });
            }
        }
        nearest
    }
}

fn intersect_triangle(
    ray: ViewportRay,
    a: [f32; 3],
    b: [f32; 3],
    c: [f32; 3],
) -> Option<(f32, [f32; 3])> {
    let edge1 = subtract(b, a);
    let edge2 = subtract(c, a);
    let p = cross(ray.direction, edge2);
    let determinant = dot(edge1, p);
    if !determinant.is_finite() || determinant.abs() <= 1.0e-7 {
        return None;
    }
    let inverse_determinant = 1.0 / determinant;
    let from_a = subtract(ray.origin, a);
    let u = dot(from_a, p) * inverse_determinant;
    if !(0.0..=1.0).contains(&u) {
        return None;
    }
    let q = cross(from_a, edge1);
    let v = dot(ray.direction, q) * inverse_determinant;
    if v < 0.0 || u + v > 1.0 {
        return None;
    }
    let distance = dot(edge2, q) * inverse_determinant;
    if !distance.is_finite() || distance < 0.0 {
        return None;
    }
    Some((distance, [1.0 - u - v, u, v]))
}

fn unproject(inverse: [[f32; 4]; 4], ndc: [f32; 3]) -> Option<[f32; 3]> {
    let input = [ndc[0], ndc[1], ndc[2], 1.0];
    let mut output = [0.0; 4];
    for row in 0..4 {
        output[row] = (0..4)
            .map(|column| inverse[row][column] * input[column])
            .sum();
    }
    if !output.iter().all(|v| v.is_finite()) || output[3].abs() <= f32::EPSILON {
        return None;
    }
    Some([
        output[0] / output[3],
        output[1] / output[3],
        output[2] / output[3],
    ])
}

fn invert(matrix: [[f32; 4]; 4]) -> Option<[[f32; 4]; 4]> {
    if !matrix.iter().flatten().all(|v| v.is_finite()) {
        return None;
    }
    let mut augmented = [[0.0_f64; 8]; 4];
    for row in 0..4 {
        for column in 0..4 {
            augmented[row][column] = matrix[row][column] as f64;
        }
        augmented[row][row + 4] = 1.0;
    }

    for pivot_column in 0..4 {
        let pivot_row = (pivot_column..4).max_by(|a, b| {
            augmented[*a][pivot_column]
                .abs()
                .total_cmp(&augmented[*b][pivot_column].abs())
        })?;
        if augmented[pivot_row][pivot_column].abs() <= 1.0e-12 {
            return None;
        }
        augmented.swap(pivot_column, pivot_row);
        let pivot = augmented[pivot_column][pivot_column];
        for value in &mut augmented[pivot_column] {
            *value /= pivot;
        }
        let pivot_values = augmented[pivot_column];
        for row in 0..4 {
            if row == pivot_column {
                continue;
            }
            let factor = augmented[row][pivot_column];
            for column in 0..8 {
                augmented[row][column] -= factor * pivot_values[column];
            }
        }
    }

    let mut inverse = [[0.0; 4]; 4];
    for row in 0..4 {
        for column in 0..4 {
            let value = augmented[row][column + 4] as f32;
            if !value.is_finite() {
                return None;
            }
            inverse[row][column] = value;
        }
    }
    Some(inverse)
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
    use crate::{PrimitiveKind, Scene, ViewportCamera};

    #[test]
    fn center_screen_ray_points_through_camera_target() {
        let camera = ViewportCamera {
            eye: [0.0, 0.0, 5.0],
            target: [0.0, 0.0, 0.0],
            ..ViewportCamera::default()
        };
        let matrices = camera.matrices(1.0).unwrap();
        let ray = matrices.screen_ray(50.0, 50.0, 100, 100).unwrap();
        assert!(ray.direction[0].abs() < 1.0e-5);
        assert!(ray.direction[1].abs() < 1.0e-5);
        assert!(ray.direction[2] < -0.999);
        assert!(ray.origin[2] < 5.0 && ray.origin[2] > 4.9);
    }

    #[test]
    fn screen_ray_rejects_pixels_outside_viewport() {
        let matrices = ViewportCamera::default().matrices(1.0).unwrap();
        assert!(matrices.screen_ray(-1.0, 2.0, 100, 100).is_none());
        assert!(matrices.screen_ray(1.0, 1.0, 0, 100).is_none());
    }

    #[test]
    fn picking_returns_nearest_mesh_hit_with_barycentric_weights() {
        let mut scene = Scene::new();
        let far_id = scene
            .add_primitive("Far", PrimitiveKind::Cube { size: 1.0 })
            .unwrap();
        scene
            .object_mut(far_id)
            .unwrap()
            .set_transform(crate::Transform3D {
                translation: [0.0, 0.0, -2.0],
                ..crate::Transform3D::default()
            });
        let near_id = scene
            .add_primitive("Near", PrimitiveKind::Cube { size: 1.0 })
            .unwrap();
        let draw = scene.viewport_snapshot().unwrap().draw_data();
        let camera = ViewportCamera {
            eye: [0.0, 0.0, 5.0],
            target: [0.0, 0.0, 0.0],
            ..ViewportCamera::default()
        };
        let matrices = camera.matrices(1.0).unwrap();
        let ray = matrices.screen_ray(50.0, 50.0, 100, 100).unwrap();
        let hit = draw.pick_mesh(ray).unwrap();
        assert_eq!(hit.object_id, near_id);
        assert!(hit.position[2] > 0.49);
        assert!((hit.barycentric.iter().sum::<f32>() - 1.0).abs() < 1.0e-5);
    }

    #[test]
    fn picking_ignores_triangles_behind_the_ray_origin() {
        let draw = ViewportDrawData {
            active_object: None,
            meshes: Vec::new(),
            hair: Vec::new(),
        };
        let ray = ViewportRay {
            origin: [0.0, 0.0, 0.0],
            direction: [0.0, 0.0, -1.0],
        };
        assert!(draw.pick_mesh(ray).is_none());
    }
}
