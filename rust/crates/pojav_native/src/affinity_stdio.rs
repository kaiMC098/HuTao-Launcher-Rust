use jni::objects::{JClass, JObject, JObjectArray, JString};
use jni::sys::{jboolean, jint, jlong, JNI_FALSE, JNI_TRUE};
use jni::JNIEnv;
use once_cell::sync::Lazy;
use std::ffi::{CStr, CString};
use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::os::unix::io::FromRawFd;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::{Mutex, Once};

static SHARED_AWT_SURFACE: AtomicI64 = AtomicI64::new(0);
static LOG_FILE: Lazy<Mutex<Option<File>>> = Lazy::new(|| Mutex::new(None));
static STDIO_REDIRECT_ONCE: Once = Once::new();

pub fn log_native(msg: &str) {
    if let Ok(mut guard) = LOG_FILE.lock() {
        if let Some(ref mut f) = *guard {
            let _ = writeln!(f, "{}", msg);
            let _ = f.flush();
        }
    }
}

pub fn start_stdio_redirect() {
    STDIO_REDIRECT_ONCE.call_once(|| unsafe {
        let mut fds: [libc::c_int; 2] = [0; 2];
        if libc::pipe(fds.as_mut_ptr()) != 0 {
            log_native("[Rust stdio] Failed to create pipe for stdout/stderr redirection");
            return;
        }
        let read_fd = fds[0];
        let write_fd = fds[1];
        libc::dup2(write_fd, libc::STDOUT_FILENO);
        libc::dup2(write_fd, libc::STDERR_FILENO);
        libc::close(write_fd);

        let _ = std::thread::Builder::new()
            .name("hutao-stdio-logger".to_string())
            .spawn(move || {
                let reader_file = File::from_raw_fd(read_fd);
                let mut reader = BufReader::new(reader_file);
                let mut line = String::new();
                while let Ok(n) = reader.read_line(&mut line) {
                    if n == 0 {
                        break;
                    }
                    let trimmed = line.trim_end_matches(&['\r', '\n'][..]);
                    if !trimmed.is_empty() {
                        log_native(trimmed);
                    }
                    line.clear();
                }
            });
    });
}

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
    let c_sym = {
        let Ok(sym_str) = env.get_string(&register_symbol) else { return -1; };
        let Ok(cs) = CString::new(sym_str.to_str().unwrap_or_default()) else { return -1; };
        cs
    };
    unsafe {
        let sym = libc::dlsym(libc::RTLD_DEFAULT, c_sym.as_ptr());
        if sym.is_null() {
            return -1;
        }
        let func: unsafe extern "C" fn(*mut jni::sys::JNIEnv) -> jint = std::mem::transmute(sym);
        func(env.get_raw())
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
    let name_str = s.to_str().unwrap_or_default().to_string();
    let Ok(cs) = CString::new(name_str.as_str()) else { return JNI_FALSE; };
    let handle = unsafe { libc::dlopen(cs.as_ptr(), libc::RTLD_LAZY | libc::RTLD_GLOBAL) };
    if handle.is_null() {
        if name_str.contains("libjli")
            || name_str.contains("libjvm")
            || name_str.contains("libgl4es")
            || name_str.contains("libOSMesa")
            || name_str.contains("libltw")
            || name_str.contains("libopenal")
        {
            let err_str = unsafe {
                let e = libc::dlerror();
                if e.is_null() {
                    "unknown dlerror".to_string()
                } else {
                    CStr::from_ptr(e).to_string_lossy().into_owned()
                }
            };
            log_native(&format!("[Rust dlopen] Failed to load {}: {}", name_str, err_str));
        }
        JNI_FALSE
    } else {
        JNI_TRUE
    }
}

#[no_mangle]
pub extern "system" fn Java_net_kdt_pojavlaunch_utils_JREUtils_setLdLibraryPath(
    mut env: JNIEnv,
    _class: JClass,
    ld_library_path: JString,
) {
    let Ok(s) = env.get_string(&ld_library_path) else { return; };
    let path_str = s.to_str().unwrap_or_default().to_string();
    let Ok(cs) = CString::new(path_str.as_str()) else { return; };
    let env_key = CString::new("LD_LIBRARY_PATH").unwrap();
    let sym_name = CString::new("android_update_LD_LIBRARY_PATH").unwrap();

    unsafe {
        libc::setenv(env_key.as_ptr(), cs.as_ptr(), 1);

        let mut sym = libc::dlsym(libc::RTLD_DEFAULT, sym_name.as_ptr());
        if sym.is_null() {
            for lib_candidate in ["libdl_android.so", "libnativeloader.so", "libdl.so"] {
                if let Ok(c_lib) = CString::new(lib_candidate) {
                    let h = libc::dlopen(c_lib.as_ptr(), libc::RTLD_LAZY);
                    if !h.is_null() {
                        sym = libc::dlsym(h, sym_name.as_ptr());
                        if !sym.is_null() {
                            break;
                        }
                    }
                }
            }
        }
        if !sym.is_null() {
            let f: unsafe extern "C" fn(*const libc::c_char) = std::mem::transmute(sym);
            f(cs.as_ptr());
        } else {
            log_native("[Rust linker] android_update_LD_LIBRARY_PATH unavailable; using absolute dlopen paths");
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
    mut env: JNIEnv,
    _class: JClass,
    log_file_path: JString,
) {
    let Ok(s) = env.get_string(&log_file_path) else { return; };
    let path_str = s.to_str().unwrap_or_default().to_string();
    if let Ok(file) = OpenOptions::new().create(true).write(true).truncate(true).open(&path_str) {
        if let Ok(mut guard) = LOG_FILE.lock() {
            *guard = Some(file);
        }
    }
}

#[no_mangle]
pub extern "system" fn Java_net_kdt_pojavlaunch_Logger_appendToLog(
    mut env: JNIEnv,
    _class: JClass,
    text: JString,
) {
    let Ok(s) = env.get_string(&text) else { return; };
    let line = s.to_str().unwrap_or_default().to_string();
    log_native(&line);
}

#[no_mangle]
pub extern "system" fn Java_net_kdt_pojavlaunch_Logger_setLogListener(
    _env: JNIEnv,
    _class: JClass,
    _listener: JObject,
) {}
