//! Small JNI boundary between the Android shell and the platform-independent scene model.
//! Only serializes a validated viewport snapshot; camera input and presentation stay in Kotlin.

use jni::objects::JClass;
use jni::sys::jstring;
use jni::JNIEnv;
use soma_geometry::{PrimitiveKind, Scene};
use std::fmt::Write as _;

fn build_scene_json() -> Result<String, String> {
    let mut scene = Scene::new();
    scene
        .add_primitive("Cube", PrimitiveKind::Cube { size: 2.0 })
        .map_err(|error| format!("scene creation failed: {error:?}"))?;

    let snapshot = scene
        .viewport_snapshot()
        .map_err(|error| format!("viewport snapshot failed: {error:?}"))?;
    let object = snapshot
        .meshes
        .first()
        .ok_or_else(|| "default scene has no mesh".to_owned())?;

    let mut json = String::from("{\"name\":\"Cube\",\"positions\":[");
    for (index, position) in object.positions.iter().enumerate() {
        if index > 0 {
            json.push(',');
        }
        write!(
            &mut json,
            "[{:.6},{:.6},{:.6}]",
            position[0], position[1], position[2]
        )
        .map_err(|error| error.to_string())?;
    }
    json.push_str("],\"polygons\":[");
    for (face_index, polygon) in object.polygons.iter().enumerate() {
        if face_index > 0 {
            json.push(',');
        }
        json.push('[');
        for (index, vertex) in polygon.iter().enumerate() {
            if index > 0 {
                json.push(',');
            }
            write!(&mut json, "{vertex}").map_err(|error| error.to_string())?;
        }
        json.push(']');
    }
    json.push_str("]}");
    Ok(json)
}

#[no_mangle]
pub extern "system" fn Java_com_soma3d_app_NativeGeometry_sceneJson(
    mut env: JNIEnv,
    _class: JClass,
) -> jstring {
    let json = build_scene_json()
        .unwrap_or_else(|error| format!("{{\"error\":\"{}\"}}", error.replace('"', "\\\"")));
    env.new_string(json)
        .map(|value| value.into_raw())
        .unwrap_or(std::ptr::null_mut())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bridge_exports_real_snapshot_vertices_and_polygons() {
        let json = build_scene_json().expect("default scene should serialize");
        assert!(json.contains("\"positions\":["));
        assert!(json.contains("\"polygons\":["));
        assert_eq!(json.matches('[').count(), 15);
        assert!(json.contains("[-1.000000,-1.000000,-1.000000]"));
    }
}
