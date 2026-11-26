use jni::objects::{JClass, JObject, JObjectArray, JString};
use jni::sys::{jboolean, jint, jlong, JNI_FALSE, JNI_TRUE};
use jni::JNIEnv;
use std::ffi::CString;
use std::fs;
use std::sync::atomic::{AtomicI64, Ordering};

static SHARED_AWT_SURFACE: AtomicI64 = AtomicI64::new(0);

#[no_mangle]
pub extern "C" fn bigcore_set_affinity() {
    let mut max_freq: u64 = 0;
    let mut big_core_id: usize = 0;

    for cpu_id in 0..32 {
        let path = format!("/sys/devices/system/cpu/cpu{}/cpufreq/cpuinfo_max_freq", cpu_id);
        match fs::read_to_string(&path) {
            Ok(content) => {
                if let Ok(freq) = content.trim().parse::<u64>() {
                    if freq >= max_freq {
                        max_freq = freq;
                        big_core_id = cpu_id;
                    }
                }
            }
            Err(_) => break,
        }
    }

    unsafe {
        let mut set: libc::cpu_set_t = std::mem::zeroed();
        libc::CPU_ZERO(&mut set);
        libc::CPU_SET(big_core_id, &mut set);
        libc::sched_setaffinity(0, std::mem::size_of::<libc::cpu_set_t>(), &set);
    }
}

#[no_mangle]
pub extern "system" fn Java_net_kdt_pojavlaunch_utils_JREUtils_setupBridgeSurfaceAWT(
    _env: JNIEnv,
    _class: JClass,
    surface: jlong,
) {
    SHARED_AWT_SURFACE.store(surface, Ordering::SeqCst);
}

#[no_mangle]
pub extern "system" fn Java_android_view_Surface_nativeGetBridgeSurfaceAWT(
    _env: JNIEnv,
    _class: JClass,
) -> jlong {
    SHARED_AWT_SURFACE.load(Ordering::SeqCst)
}

#[no_mangle]
pub extern "system" fn Java_android_os_OpenJDKNativeRegister_nativeRegisterNatives(
    mut env: JNIEnv,
    _class: JClass,
    register_symbol: JString,
) -> jint {
    let Ok(sym_str) = env.get_string(&register_symbol) else { return -1; };
    let Ok(c_sym) = CString::new(sym_str.to_str().unwrap_or_default()) else { return -1; };
    unsafe {
        let sym = libc::dlsym(libc::RTLD_DEFAULT, c_sym.as_ptr());
        if sym.is_null() {
            return -1;
        }
        let func: unsafe extern "C" fn(*mut jni::sys::JNIEnv) -> jint = std::mem::transmute(sym);
        func(env.get_native_interface())
    }
}

#[no_mangle]
pub extern "system" fn Java_net_kdt_pojavlaunch_utils_JREUtils_chdir(
    mut env: JNIEnv,
    _class: JClass,
    name_str: JString,
) -> jint {
    let Ok(s) = env.get_string(&name_str) else { return -1; };
    let Ok(cs) = CString::new(s.to_str().unwrap_or_default()) else { return -1; };
    unsafe { libc::chdir(cs.as_ptr()) }
}

#[no_mangle]
pub extern "system" fn Java_net_kdt_pojavlaunch_utils_JREUtils_dlopen(
    mut env: JNIEnv,
    _class: JClass,
    name: JString,
) -> jboolean {
    let Ok(s) = env.get_string(&name) else { return JNI_FALSE; };
    let Ok(cs) = CString::new(s.to_str().unwrap_or_default()) else { return JNI_FALSE; };
    let handle = unsafe { libc::dlopen(cs.as_ptr(), libc::RTLD_LAZY | libc::RTLD_GLOBAL) };
    if handle.is_null() { JNI_FALSE } else { JNI_TRUE }
}

#[no_mangle]
pub extern "system" fn Java_net_kdt_pojavlaunch_utils_JREUtils_setLdLibraryPath(
    mut env: JNIEnv,
    _class: JClass,
    ld_library_path: JString,
) {
    let Ok(s) = env.get_string(&ld_library_path) else { return; };
    let Ok(cs) = CString::new(s.to_str().unwrap_or_default()) else { return; };
    let libdl = CString::new("libdl.so").unwrap();
    let sym_name = CString::new("android_update_LD_LIBRARY_PATH").unwrap();
    unsafe {
        let h = libc::dlopen(libdl.as_ptr(), libc::RTLD_LAZY);
        let sym = libc::dlsym(h, sym_name.as_ptr());
        if !sym.is_null() {
            let f: unsafe extern "C" fn(*const libc::c_char) = std::mem::transmute(sym);
            f(cs.as_ptr());
        }
    }
}

#[no_mangle]
pub extern "system" fn Java_net_kdt_pojavlaunch_utils_JREUtils_executeBinary(
    _env: JNIEnv,
    _class: JClass,
    _cmd_args: JObjectArray,
) -> jint {
    0
}

#[no_mangle]
pub extern "system" fn Java_net_kdt_pojavlaunch_utils_JREUtils_executeForkedBinary(
    _env: JNIEnv,
    _class: JClass,
    _cmd_args: JObjectArray,
) -> jint {
    0
}

#[no_mangle]
pub extern "system" fn Java_net_kdt_pojavlaunch_utils_JREUtils_setupExitMethod(
    _env: JNIEnv,
    _class: JClass,
    _context: JObject,
) {}

#[no_mangle]
pub extern "system" fn Java_net_kdt_pojavlaunch_Logger_begin(
    _env: JNIEnv,
    _class: JClass,
    _log_file_path: JString,
) {}

#[no_mangle]
pub extern "system" fn Java_net_kdt_pojavlaunch_Logger_appendToLog(
    _env: JNIEnv,
    _class: JClass,
    _text: JString,
) {}

#[no_mangle]
pub extern "system" fn Java_net_kdt_pojavlaunch_Logger_setLogListener(
    _env: JNIEnv,
    _class: JClass,
    _listener: JObject,
) {}
