use jni::objects::{JClass, JString};
use jni::sys::{jint, jstring};
use jni::JNIEnv;
use once_cell::sync::Lazy;
use pojav_core::{auth, controls, modloaders};
use pojav_ui::LauncherUiState;
use std::sync::Mutex;

static UI_STATE: Lazy<Mutex<LauncherUiState>> = Lazy::new(|| Mutex::new(LauncherUiState::new()));

#[no_mangle]
pub extern "system" fn Java_net_kdt_pojavlaunch_rust_RustCoreBridge_nativeGetUiSnapshotJson(
    env: JNIEnv,
    _class: JClass,
) -> jstring {
    let json = UI_STATE.lock().map(|s| s.to_json()).unwrap_or_else(|_| "{}".into());
    env.new_string(json).map(|s| s.into_raw()).unwrap_or(std::ptr::null_mut())
}

#[no_mangle]
pub extern "system" fn Java_net_kdt_pojavlaunch_rust_RustCoreBridge_nativeLoginOffline(
    mut env: JNIEnv,
    _class: JClass,
    username: JString,
) -> jstring {
    let user = env.get_string(&username).map(|s| s.to_str().unwrap_or_default().to_string()).unwrap_or_default();
    let res_json = match auth::create_offline_account(&user) {
        Ok(acc) => {
            if let Ok(mut ui) = UI_STATE.lock() {
                ui.snapshot.active_account = Some(acc.clone());
            }
            serde_json::to_string(&acc).unwrap_or_else(|_| "{}".into())
        }
        Err(e) => format!(r#"{{"error":"{}"}}"#, e),
    };
    env.new_string(res_json).map(|s| s.into_raw()).unwrap_or(std::ptr::null_mut())
}

#[no_mangle]
pub extern "system" fn Java_net_kdt_pojavlaunch_rust_RustCoreBridge_nativeSearchModrinth(
    mut env: JNIEnv,
    _class: JClass,
    query: JString,
    limit: jint,
) -> jstring {
    let q = env.get_string(&query).map(|s| s.to_str().unwrap_or_default().to_string()).unwrap_or_default();
    let res_json = match modloaders::search_modrinth(&q, limit.max(1) as usize) {
        Ok(list) => serde_json::to_string(&list).unwrap_or_else(|_| "[]".into()),
        Err(e) => format!(r#"{{"error":"{}"}}"#, e),
    };
    env.new_string(res_json).map(|s| s.into_raw()).unwrap_or(std::ptr::null_mut())
}

#[no_mangle]
pub extern "system" fn Java_net_kdt_pojavlaunch_rust_RustCoreBridge_nativeMapAndroidKeyToGlfw(
    _env: JNIEnv,
    _class: JClass,
    android_keycode: jint,
) -> jint {
    controls::android_key_to_glfw(android_keycode)
}
