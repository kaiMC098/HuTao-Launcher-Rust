use crate::models::ControlLayout;

pub fn android_key_to_glfw(android_keycode: i32) -> i32 {
    match android_keycode {
        4 => 256,
        19 => 265,
        20 => 264,
        21 => 263,
        22 => 262,
        29..=54 => 65 + (android_keycode - 29),
        7..=16 => 48 + (android_keycode - 7),
        59 | 60 => 340,
        61 => 258,
        62 => 32,
        66 => 257,
        67 => 259,
        111 => 256,
        113 | 114 => 341,
        131..=142 => 290 + (android_keycode - 131),
        _ => 0,
    }
}

pub fn sanitize_control_layout(raw_json: &str) -> Result<String, serde_json::Error> {
    let mut layout: ControlLayout = serde_json::from_str(raw_json)?;
    if layout.scaled_at <= 0.0 {
        layout.scaled_at = 100.0;
    }
    for btn in &mut layout.m_control_data_list {
        btn.opacity = btn.opacity.clamp(0.05, 1.0);
        btn.width = btn.width.max(10.0);
        btn.height = btn.height.max(10.0);
    }
    serde_json::to_string_pretty(&layout)
}
