use jni::objects::JClass;
use jni::sys::jstring;
use jni::JNIEnv;
use once_cell::sync::Lazy;
use pojav_core::soc::{classify_soc, SocProfile};
use std::ffi::{c_char, CStr, CString};
use std::fs;
use std::sync::Mutex;

#[cfg(target_os = "android")]
extern "C" {
    fn __system_property_get(name: *const c_char, value: *mut c_char) -> libc::c_int;
}

#[cfg(target_os = "android")]
fn get_android_prop(key: &str) -> String {
    let Ok(c_key) = CString::new(key) else {
        return String::new();
    };
    let mut buf = [0u8; 128];
    let len = unsafe { __system_property_get(c_key.as_ptr(), buf.as_mut_ptr() as *mut c_char) };
    if len > 0 {
        unsafe { CStr::from_ptr(buf.as_ptr() as *const c_char) }
            .to_string_lossy()
            .trim()
            .to_string()
    } else {
        String::new()
    }
}

#[cfg(not(target_os = "android"))]
fn get_android_prop(_key: &str) -> String {
    String::new()
}

pub fn get_android_api_level() -> i32 {
    get_android_prop("ro.build.version.sdk")
        .parse::<i32>()
        .unwrap_or(21)
}

fn read_cpuinfo_hardware() -> String {
    if let Ok(txt) = fs::read_to_string("/proc/cpuinfo") {
        for line in txt.lines() {
            if line.starts_with("Hardware") || line.starts_with("model name") {
                if let Some((_, val)) = line.split_once(':') {
                    return val.trim().to_string();
                }
            }
        }
    }
    String::new()
}

pub static DETECTED_SOC: Lazy<Mutex<SocProfile>> = Lazy::new(|| {
    let mfg = get_android_prop("ro.soc.manufacturer");
    let model = {
        let m = get_android_prop("ro.soc.model");
        if m.is_empty() {
            get_android_prop("ro.board.platform")
        } else {
            m
        }
    };
    let hw = {
        let h = get_android_prop("ro.hardware");
        if h.is_empty() {
            read_cpuinfo_hardware()
        } else {
            h
        }
    };
    Mutex::new(classify_soc(&mfg, &model, &hw))
});

/// Áp dụng biến môi trường & cấu hình tối ưu theo SoC (Dimensity Mali vs Snapdragon Adreno)
pub fn apply_soc_runtime_optimizations() -> SocProfile {
    let profile = DETECTED_SOC.lock().map(|p| p.clone()).unwrap_or_else(|_| classify_soc("", "", ""));

    let renderer_name = unsafe {
        let rk = CString::new("POJAV_RENDERER").unwrap();
        let rp = libc::getenv(rk.as_ptr());
        if rp.is_null() {
            String::new()
        } else {
            CStr::from_ptr(rp).to_string_lossy().into_owned()
        }
    };
    let is_gl4es = renderer_name.is_empty()
        || renderer_name == "opengles2"
        || renderer_name == "opengles2_5"
        || renderer_name == "opengles3"
        || renderer_name == "opengles3_ng_gl4es";

    for (k, v) in &profile.env_overrides {
        if !is_gl4es && k.starts_with("LIBGL_") {
            continue;
        }
        if let (Ok(ck), Ok(cv)) = (CString::new(k.as_str()), CString::new(v.as_str())) {
            unsafe {
                // Không ghi đè nếu người dùng đã tự cấu hình trong custom_env.txt
                libc::setenv(ck.as_ptr(), cv.as_ptr(), 0);
            }
        }
    }

    // Trên Dimensity / Mali / Exynos, gỡ POJAV_LOAD_TURNIP để tránh crash driver Vulkan Adreno
    if !profile.enable_turnip {
        if let Ok(turnip_key) = CString::new("POJAV_LOAD_TURNIP") {
            unsafe {
                libc::unsetenv(turnip_key.as_ptr());
            }
        }
    }

    profile
}

/// Ghim luồng dựng hình vào toàn bộ cụm nhân Big & Prime (xung nhịp >= 80% nhân mạnh nhất)
/// Giúp Dimensity 7000/8000/9000 & Snapdragon đa nhân tải chunk mượt mà hơn
#[no_mangle]
pub extern "C" fn bigcore_set_cluster_affinity() {
    let threshold_pct = DETECTED_SOC
        .lock()
        .map(|p| p.big_core_freq_threshold_pct as u64)
        .unwrap_or(80);

    let mut freqs: Vec<(usize, u64)> = Vec::new();
    let mut max_freq: u64 = 0;

    for cpu_id in 0..32 {
        let path = format!("/sys/devices/system/cpu/cpu{}/cpufreq/cpuinfo_max_freq", cpu_id);
        match fs::read_to_string(&path) {
            Ok(content) => {
                if let Ok(freq) = content.trim().parse::<u64>() {
                    if freq > max_freq {
                        max_freq = freq;
                    }
                    freqs.push((cpu_id, freq));
                }
            }
            Err(_) => break,
        }
    }

    if freqs.is_empty() || max_freq == 0 {
        return;
    }

    let min_target_freq = (max_freq * threshold_pct) / 100;
    unsafe {
        let mut set: libc::cpu_set_t = std::mem::zeroed();
        libc::CPU_ZERO(&mut set);
        let mut pinned_count = 0;
        for (cpu_id, freq) in &freqs {
            if *freq >= min_target_freq {
                libc::CPU_SET(*cpu_id, &mut set);
                pinned_count += 1;
            }
        }
        if pinned_count > 0 {
            libc::sched_setaffinity(0, std::mem::size_of::<libc::cpu_set_t>(), &set);
        }
    }
}

#[no_mangle]
pub extern "system" fn Java_net_kdt_pojavlaunch_rust_RustCoreBridge_nativeDetectSocProfileJson(
    env: JNIEnv,
    _class: JClass,
) -> jstring {
    let json = DETECTED_SOC
        .lock()
        .map(|p| serde_json::to_string(&*p).unwrap_or_else(|_| "{}".into()))
        .unwrap_or_else(|_| "{}".into());
    env.new_string(json)
        .map(|s| s.into_raw())
        .unwrap_or(std::ptr::null_mut())
}

#[no_mangle]
pub extern "system" fn Java_net_kdt_pojavlaunch_rust_RustCoreBridge_nativeApplySocOptimizations(
    env: JNIEnv,
    _class: JClass,
) -> jstring {
    let profile = apply_soc_runtime_optimizations();
    bigcore_set_cluster_affinity();
    let json = serde_json::to_string(&profile).unwrap_or_else(|_| "{}".into());
    env.new_string(json)
        .map(|s| s.into_raw())
        .unwrap_or(std::ptr::null_mut())
}
