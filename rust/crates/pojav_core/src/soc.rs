use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SocVendor {
    MediaTekDimensity,
    MediaTekHelio,
    QualcommSnapdragon,
    SamsungExynos,
    GoogleTensor,
    Unisoc,
    GenericArm,
    X86,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum GpuFamily {
    ArmMaliImmortalis,
    QualcommAdreno,
    SamsungXclipse,
    PowerVr,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SocProfile {
    pub manufacturer: String,
    pub model: String,
    pub hardware: String,
    pub vendor: SocVendor,
    pub gpu_family: GpuFamily,
    pub recommended_renderer: String,
    pub enable_turnip: bool,
    pub big_core_freq_threshold_pct: u32,
    pub env_overrides: Vec<(String, String)>,
}

/// Phân tích chuỗi nhận diện phần cứng Android và tạo cấu hình tối ưu riêng cho từng dòng chip
pub fn classify_soc(manufacturer: &str, model: &str, hardware: &str) -> SocProfile {
    let combined = format!("{} {} {}", manufacturer, model, hardware).to_lowercase();

    let (vendor, gpu_family) = if combined.contains("x86") || combined.contains("goldfish") || combined.contains("ranchu") {
        (SocVendor::X86, GpuFamily::Unknown)
    } else if combined.contains("dimensity")
        || combined.contains("mt68")
        || combined.contains("mt69")
        || combined.contains("mt6789")
    {
        (SocVendor::MediaTekDimensity, GpuFamily::ArmMaliImmortalis)
    } else if combined.contains("mediatek") || combined.contains("helio") || combined.contains("mt67") {
        (SocVendor::MediaTekHelio, GpuFamily::ArmMaliImmortalis)
    } else if combined.contains("qualcomm")
        || combined.contains("snapdragon")
        || combined.contains("qcom")
        || combined.contains("sm")
        || combined.contains("sdm")
        || combined.contains("adreno")
    {
        (SocVendor::QualcommSnapdragon, GpuFamily::QualcommAdreno)
    } else if combined.contains("exynos") || combined.contains("s5e") || combined.contains("universal") {
        let gpu = if combined.contains("s5e99") || combined.contains("s5e88") {
            GpuFamily::SamsungXclipse
        } else {
            GpuFamily::ArmMaliImmortalis
        };
        (SocVendor::SamsungExynos, gpu)
    } else if combined.contains("tensor") || combined.contains("gs101") || combined.contains("gs201") || combined.contains("zuma") {
        (SocVendor::GoogleTensor, GpuFamily::ArmMaliImmortalis)
    } else if combined.contains("unisoc") || combined.contains("spreadtrum") || combined.contains("ums") {
        (SocVendor::Unisoc, GpuFamily::ArmMaliImmortalis)
    } else {
        (SocVendor::GenericArm, GpuFamily::Unknown)
    };

    let mut env_overrides = vec![
        ("LIBGL_NOERROR".to_string(), "1".to_string()),
        ("MESA_GLSL_CACHE_DISABLE".to_string(), "false".to_string()),
    ];

    let (recommended_renderer, enable_turnip, big_core_freq_threshold_pct) = match vendor {
        SocVendor::MediaTekDimensity => {
            // Tối ưu chuyên biệt cho GPU Mali / Immortalis & cụm nhân Prime/Big trên Dimensity
            env_overrides.push(("POJAV_DIMENSITY_OPT".to_string(), "1".to_string()));
            env_overrides.push(("LIBGL_NPOT".to_string(), "1".to_string()));
            env_overrides.push(("LIBGL_FASTEXACT".to_string(), "1".to_string()));
            env_overrides.push(("LIBGL_SHRINK".to_string(), "2".to_string()));
            ("opengles2".to_string(), false, 80)
        }
        SocVendor::MediaTekHelio => {
            env_overrides.push(("LIBGL_NPOT".to_string(), "1".to_string()));
            env_overrides.push(("LIBGL_SHRINK".to_string(), "2".to_string()));
            ("opengles2".to_string(), false, 85)
        }
        SocVendor::QualcommSnapdragon => {
            // Tối ưu Adreno + hỗ trợ Turnip Vulkan driver
            env_overrides.push(("POJAV_SNAPDRAGON_OPT".to_string(), "1".to_string()));
            ("opengles2".to_string(), true, 80)
        }
        SocVendor::SamsungExynos | SocVendor::GoogleTensor | SocVendor::Unisoc => {
            env_overrides.push(("LIBGL_NPOT".to_string(), "1".to_string()));
            ("opengles2".to_string(), false, 80)
        }
        SocVendor::GenericArm | SocVendor::X86 => ("opengles2".to_string(), false, 85),
    };

    SocProfile {
        manufacturer: manufacturer.to_string(),
        model: model.to_string(),
        hardware: hardware.to_string(),
        vendor,
        gpu_family,
        recommended_renderer,
        enable_turnip,
        big_core_freq_threshold_pct,
        env_overrides,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dimensity_detection() {
        let p = classify_soc("Mediatek", "MT6897", "mt6897");
        assert_eq!(p.vendor, SocVendor::MediaTekDimensity);
        assert_eq!(p.gpu_family, GpuFamily::ArmMaliImmortalis);
        assert!(!p.enable_turnip);
        assert_eq!(p.recommended_renderer, "opengles2");
    }

    #[test]
    fn test_snapdragon_detection() {
        let p = classify_soc("Qualcomm", "SM8650", "qcom");
        assert_eq!(p.vendor, SocVendor::QualcommSnapdragon);
        assert_eq!(p.gpu_family, GpuFamily::QualcommAdreno);
        assert!(p.enable_turnip);
    }
}
