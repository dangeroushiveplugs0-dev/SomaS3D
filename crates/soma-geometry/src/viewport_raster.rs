//! Deterministic CPU reference renderer for viewport integration tests and previews.
//!
//! This renderer intentionally has no platform or GPU dependencies. It proves
//! the complete snapshot -> draw buffers -> camera -> pixels path and provides
//! a small fallback image for diagnostics; production mobile rendering should
//! use a GPU backend.

use crate::{ViewportCamera, ViewportCameraError, ViewportDrawData};

const MAX_PIXELS: usize = 16_777_216;

/// RGBA8 image produced by the CPU viewport renderer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ViewportFrame {
    pub width: u32,
    pub height: u32,
    /// Row-major pixels, four bytes per pixel in red/green/blue/alpha order.
    pub rgba: Vec<u8>,
}

/// Renderer options for surface fill, wireframe, and background.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ViewportRenderSettings {
    pub background: [u8; 4],
    pub show_surfaces: bool,
    pub show_edges: bool,
    pub show_hair: bool,
}

impl Default for ViewportRenderSettings {
    fn default() -> Self {
        Self {
            background: [29, 32, 38, 255],
            show_surfaces: true,
            show_edges: true,
            show_hair: true,
        }
    }
}

/// Rendering failure for invalid dimensions or camera settings.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewportRenderError {
    EmptyViewport,
    ViewportTooLarge,
    InvalidCamera(ViewportCameraError),
}

impl ViewportDrawData {
    /// Rasterizes the prepared draw data into a top-left-origin RGBA8 image.
    ///
    /// This is a correctness-oriented CPU reference path, not the intended
    /// high-performance mobile GPU renderer. It uses a depth buffer, a fixed
    /// directional light, a neutral mesh material, dark edges, and authored
    /// hair colors. The active mesh receives a subtle orange selection tint.
    pub fn render_cpu(
        &self,
        camera: ViewportCamera,
        width: u32,
        height: u32,
        settings: ViewportRenderSettings,
    ) -> Result<ViewportFrame, ViewportRenderError> {
        if width == 0 || height == 0 {
            return Err(ViewportRenderError::EmptyViewport);
        }
        let pixel_count = (width as usize)
            .checked_mul(height as usize)
            .filter(|count| *count <= MAX_PIXELS)
            .ok_or(ViewportRenderError::ViewportTooLarge)?;
        let matrices = camera
            .matrices(width as f32 / height as f32)
            .map_err(ViewportRenderError::InvalidCamera)?;
        let mut frame = ViewportFrame {
            width,
            height,
            rgba: settings.background.repeat(pixel_count),
        };
        let mut depth = vec![f32::INFINITY; pixel_count];

        if settings.show_surfaces {
            for mesh in &self.meshes {
                for triangle in mesh.triangle_indices.chunks_exact(3) {
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
                    let Some(pa) = project(&matrices, a, width, height) else {
                        continue;
                    };
                    let Some(pb) = project(&matrices, b, width, height) else {
                        continue;
                    };
                    let Some(pc) = project(&matrices, c, width, height) else {
                        continue;
                    };
                    let normal = normalize(cross(subtract(b, a), subtract(c, a)));
                    let light = normal
                        .map(|n| {
                            (dot(n, normalize([-0.35, 0.8, 0.5]).unwrap()).abs() * 0.72 + 0.22)
                                .clamp(0.0, 1.0)
                        })
                        .unwrap_or(0.25);
                    let base = if self.active_object == Some(mesh.object_id) {
                        [204_u8, 139, 91]
                    } else {
                        [150_u8, 165, 181]
                    };
                    let color = [
                        (base[0] as f32 * light) as u8,
                        (base[1] as f32 * light) as u8,
                        (base[2] as f32 * light) as u8,
                        255,
                    ];
                    raster_triangle(&mut frame, &mut depth, [pa, pb, pc], color);
                }
            }
        }

        if settings.show_edges {
            for mesh in &self.meshes {
                let color = if self.active_object == Some(mesh.object_id) {
                    [255, 190, 118, 255]
                } else {
                    [24, 27, 32, 255]
                };
                for edge in mesh.edge_indices.chunks_exact(2) {
                    let Some(&a) = mesh.positions.get(edge[0] as usize) else {
                        continue;
                    };
                    let Some(&b) = mesh.positions.get(edge[1] as usize) else {
                        continue;
                    };
                    let (Some(a), Some(b)) = (
                        project(&matrices, a, width, height),
                        project(&matrices, b, width, height),
                    ) else {
                        continue;
                    };
                    raster_line(&mut frame, &mut depth, a, b, color);
                }
            }
        }

        if settings.show_hair {
            for hair in &self.hair {
                let color = [
                    (hair.color[0].clamp(0.0, 1.0) * 255.0) as u8,
                    (hair.color[1].clamp(0.0, 1.0) * 255.0) as u8,
                    (hair.color[2].clamp(0.0, 1.0) * 255.0) as u8,
                    (hair.color[3].clamp(0.0, 1.0) * 255.0) as u8,
                ];
                for segment in hair.line_positions.chunks_exact(2) {
                    let (Some(a), Some(b)) = (
                        project(&matrices, segment[0], width, height),
                        project(&matrices, segment[1], width, height),
                    ) else {
                        continue;
                    };
                    raster_line(&mut frame, &mut depth, a, b, color);
                }
            }
        }

        Ok(frame)
    }
}

fn project(
    matrices: &crate::ViewportCameraMatrices,
    point: [f32; 3],
    width: u32,
    height: u32,
) -> Option<[f32; 3]> {
    let ndc = matrices.project_world_to_ndc(point)?;
    if !(-1.0..=1.0).contains(&ndc[2]) {
        return None;
    }
    crate::ViewportCameraMatrices::ndc_to_screen(ndc, width, height)
}

fn raster_triangle(
    frame: &mut ViewportFrame,
    depth: &mut [f32],
    points: [[f32; 3]; 3],
    color: [u8; 4],
) {
    let area = edge_function(points[0], points[1], points[2][0], points[2][1]);
    if !area.is_finite() || area.abs() <= f32::EPSILON {
        return;
    }
    let min_x = points
        .iter()
        .map(|p| p[0].floor() as i32)
        .min()
        .unwrap()
        .max(0);
    let max_x = points
        .iter()
        .map(|p| p[0].ceil() as i32)
        .max()
        .unwrap()
        .min(frame.width as i32 - 1);
    let min_y = points
        .iter()
        .map(|p| p[1].floor() as i32)
        .min()
        .unwrap()
        .max(0);
    let max_y = points
        .iter()
        .map(|p| p[1].ceil() as i32)
        .max()
        .unwrap()
        .min(frame.height as i32 - 1);
    if min_x > max_x || min_y > max_y {
        return;
    }

    for y in min_y..=max_y {
        for x in min_x..=max_x {
            let px = x as f32 + 0.5;
            let py = y as f32 + 0.5;
            let w0 = edge_function(points[1], points[2], px, py) / area;
            let w1 = edge_function(points[2], points[0], px, py) / area;
            let w2 = 1.0 - w0 - w1;
            if w0 < -1.0e-5 || w1 < -1.0e-5 || w2 < -1.0e-5 {
                continue;
            }
            let z = w0 * points[0][2] + w1 * points[1][2] + w2 * points[2][2];
            let index = y as usize * frame.width as usize + x as usize;
            if z < depth[index] {
                depth[index] = z;
                write_pixel(frame, index, color);
            }
        }
    }
}

fn raster_line(
    frame: &mut ViewportFrame,
    depth: &mut [f32],
    a: [f32; 3],
    b: [f32; 3],
    color: [u8; 4],
) {
    let dx = b[0] - a[0];
    let dy = b[1] - a[1];
    let steps = dx.abs().max(dy.abs()).ceil() as usize;
    if steps == 0 {
        draw_depth_pixel(frame, depth, a, color);
        return;
    }
    for step in 0..=steps {
        let t = step as f32 / steps as f32;
        draw_depth_pixel(
            frame,
            depth,
            [a[0] + dx * t, a[1] + dy * t, a[2] + (b[2] - a[2]) * t],
            color,
        );
    }
}

fn draw_depth_pixel(frame: &mut ViewportFrame, depth: &mut [f32], point: [f32; 3], color: [u8; 4]) {
    let x = point[0].round() as i32;
    let y = point[1].round() as i32;
    if x < 0 || y < 0 || x >= frame.width as i32 || y >= frame.height as i32 {
        return;
    }
    let index = y as usize * frame.width as usize + x as usize;
    if point[2] <= depth[index] + 1.0e-4 {
        depth[index] = depth[index].min(point[2]);
        write_pixel(frame, index, color);
    }
}

fn write_pixel(frame: &mut ViewportFrame, index: usize, color: [u8; 4]) {
    let offset = index * 4;
    frame.rgba[offset..offset + 4].copy_from_slice(&color);
}

fn edge_function(a: [f32; 3], b: [f32; 3], x: f32, y: f32) -> f32 {
    (x - a[0]) * (b[1] - a[1]) - (y - a[1]) * (b[0] - a[0])
}

fn subtract(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{PrimitiveKind, Scene};

    #[test]
    fn renders_scene_snapshot_into_rgba_pixels() {
        let mut scene = Scene::new();
        scene
            .add_primitive("Cube", PrimitiveKind::Cube { size: 1.0 })
            .unwrap();
        let draw = scene.viewport_snapshot().unwrap().draw_data();
        let frame = draw
            .render_cpu(
                ViewportCamera::default(),
                96,
                64,
                ViewportRenderSettings::default(),
            )
            .unwrap();

        assert_eq!(frame.rgba.len(), 96 * 64 * 4);
        assert!(frame
            .rgba
            .chunks_exact(4)
            .any(|pixel| pixel[..3] != [29, 32, 38]));
        assert!(frame.rgba.chunks_exact(4).all(|pixel| pixel[3] == 255));
    }

    #[test]
    fn render_rejects_empty_or_excessive_viewports() {
        let draw = crate::ViewportDrawData {
            active_object: None,
            meshes: Vec::new(),
            hair: Vec::new(),
        };
        assert_eq!(
            draw.render_cpu(
                ViewportCamera::default(),
                0,
                64,
                ViewportRenderSettings::default()
            ),
            Err(ViewportRenderError::EmptyViewport)
        );
        assert_eq!(
            draw.render_cpu(
                ViewportCamera::default(),
                5000,
                5000,
                ViewportRenderSettings::default()
            ),
            Err(ViewportRenderError::ViewportTooLarge)
        );
    }
}
