//! JNI bridge for the Android viewport and the persistent Rust scene.
use jni::objects::JClass;
use jni::sys::{jlong, jstring};
use jni::JNIEnv;
use soma_geometry::{PrimitiveKind, Scene, Transform3D};
use std::fmt::Write as _;
use std::sync::{Mutex, OnceLock};

static SCENE: OnceLock<Mutex<Scene>> = OnceLock::new();

fn with_scene<T>(operation: impl FnOnce(&mut Scene) -> Result<T, String>) -> Result<T, String> {
    let mutex = SCENE.get_or_init(|| {
        let mut scene = Scene::new();
        let _ = scene.add_primitive("Cube", PrimitiveKind::Cube { size: 2.0 });
        Mutex::new(scene)
    });
    let mut scene = mutex.lock().map_err(|_| "scene lock poisoned".to_owned())?;
    operation(&mut scene)
}

fn snapshot_json() -> Result<String, String> {
    with_scene(|scene| {
        let snapshot = scene.viewport_snapshot().map_err(|error| format!("snapshot failed: {error:?}"))?;
        let mut json = format!(
            "{{\"active\":{},\"meshes\":[",
            snapshot.active_object.map(|id| id.0.to_string()).unwrap_or_else(|| "null".to_owned())
        );
        for (mesh_index, object) in snapshot.meshes.iter().enumerate() {
            if mesh_index > 0 { json.push(','); }
            write!(&mut json, "{{\"id\":{},\"name\":\"{}\",\"positions\":[", object.id.0, object.name.replace('"', "\\\""))
                .map_err(|error| error.to_string())?;
            for (index, position) in object.positions.iter().enumerate() {
                if index > 0 { json.push(','); }
                write!(&mut json, "[{:.6},{:.6},{:.6}]", position[0], position[1], position[2])
                    .map_err(|error| error.to_string())?;
            }
            json.push_str("],\"polygons\":[");
            for (face_index, polygon) in object.polygons.iter().enumerate() {
                if face_index > 0 { json.push(','); }
                json.push('[');
                for (index, vertex) in polygon.iter().enumerate() {
                    if index > 0 { json.push(','); }
                    write!(&mut json, "{vertex}").map_err(|error| error.to_string())?;
                }
                json.push(']');
            }
            json.push_str("]}");
        }
        json.push_str("]}");
        Ok(json)
    })
}

fn add_primitive(kind: PrimitiveKind, label: &str) -> Result<i64, String> {
    with_scene(|scene| {
        let next_index = scene.objects().len() as f32;
        let id = scene.add_primitive(label, kind).map_err(|error| format!("add primitive failed: {error:?}"))?;
        let mut transform = Transform3D::default();
        transform.translation = [next_index * 2.8, 0.0, 0.0];
        scene.object_mut(id).ok_or_else(|| "new object missing".to_owned())?.set_transform(transform);
        Ok(id.0 as i64)
    })
}

#[no_mangle]
pub extern "system" fn Java_com_soma3d_app_NativeGeometry_sceneJson(mut env: JNIEnv, _class: JClass) -> jstring {
    let json = snapshot_json().unwrap_or_else(|error| format!("{{\"error\":\"{}\"}}", error.replace('"', "\\\"")));
    env.new_string(json).map(|value| value.into_raw()).unwrap_or(std::ptr::null_mut())
}

#[no_mangle]
pub extern "system" fn Java_com_soma3d_app_NativeGeometry_addCube(_env: JNIEnv, _class: JClass) -> jlong {
    add_primitive(PrimitiveKind::Cube { size: 2.0 }, "Cube").unwrap_or(-1) as jlong
}

#[no_mangle]
pub extern "system" fn Java_com_soma3d_app_NativeGeometry_addSphere(_env: JNIEnv, _class: JClass) -> jlong {
    add_primitive(PrimitiveKind::UvSphere { radius: 1.0, segments: 20, rings: 12 }, "UV Sphere").unwrap_or(-1) as jlong
}

#[no_mangle]
pub extern "system" fn Java_com_soma3d_app_NativeGeometry_setActiveObject(_env: JNIEnv, _class: JClass, id: jlong) -> jlong {
    with_scene(|scene| {
        scene.set_active_object(Some(soma_geometry::ObjectId(id as u64)))
            .map_err(|error| format!("select object failed: {error:?}"))?;
        Ok(id)
    }).unwrap_or(-1) as jlong
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scene_persists_added_objects_and_active_selection() {
        let first = add_primitive(PrimitiveKind::Cube { size: 1.0 }, "Test Cube").unwrap();
        let second = add_primitive(PrimitiveKind::UvSphere { radius: 1.0, segments: 8, rings: 6 }, "Test Sphere").unwrap();
        with_scene(|scene| scene.set_active_object(Some(soma_geometry::ObjectId(first as u64)).map_err(|error| format!("{error:?}"))).map(|_| ())).unwrap();
        let json = snapshot_json().unwrap();
        assert!(json.contains("\"meshes\":["));
        assert!(json.contains("\"Test Cube\""));
        assert!(json.contains("\"Test Sphere\""));
        assert!(json.contains(&format!("\"active\":{first}")));
        assert!(second > first);
    }
}
