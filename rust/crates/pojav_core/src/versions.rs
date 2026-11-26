use crate::models::{LauncherSettings, MinecraftAccount, MinecraftProfile};

#[derive(Debug, Clone)]
pub struct LaunchContext {
    pub game_dir: String,
    pub assets_dir: String,
    pub libraries_classpath: String,
    pub main_class: String,
    pub asset_index: String,
    pub version_id: String,
    pub width: u32,
    pub height: u32,
    pub account: MinecraftAccount,
    pub profile: MinecraftProfile,
    pub settings: LauncherSettings,
}

pub fn build_jvm_launch_args(ctx: &LaunchContext) -> Vec<String> {
    let mut args = vec![
        "java".to_string(),
        format!("-Xms{}M", ctx.settings.ram_allocation_mb.min(512)),
        format!("-Xmx{}M", ctx.settings.ram_allocation_mb),
        format!("-Duser.home={}", ctx.game_dir),
        "-Dorg.lwjgl.opengl.libname=libgl4es_114.so".to_string(),
        format!("-Dglfwstub.windowWidth={}", ctx.width),
        format!("-Dglfwstub.windowHeight={}", ctx.height),
        "-Djava.awt.headless=false".to_string(),
        "-Dcacio.managed.screensize=true".to_string(),
        "-Dcacio.font.fontmanager=sun.awt.X11FontManager".to_string(),
        "-Dcacio.font.fontscaler=sun.font.FreetypeFontScaler".to_string(),
    ];

    for extra in ctx.settings.custom_jvm_args.split_whitespace() {
        args.push(extra.to_string());
    }
    if let Some(ref prof_args) = ctx.profile.java_args {
        for extra in prof_args.split_whitespace() {
            args.push(extra.to_string());
        }
    }

    args.push("-cp".to_string());
    args.push(ctx.libraries_classpath.clone());
    args.push(ctx.main_class.clone());

    args.extend([
        "--username".to_string(),
        ctx.account.username.clone(),
        "--version".to_string(),
        ctx.version_id.clone(),
        "--gameDir".to_string(),
        ctx.game_dir.clone(),
        "--assetsDir".to_string(),
        ctx.assets_dir.clone(),
        "--assetIndex".to_string(),
        ctx.asset_index.clone(),
        "--uuid".to_string(),
        ctx.account.uuid.clone(),
        "--accessToken".to_string(),
        ctx.account.access_token.clone(),
        "--userType".to_string(),
        if ctx.account.is_microsoft { "msa" } else { "legacy" }.to_string(),
        "--versionType".to_string(),
        "HuTaoLauncher-Rust".to_string(),
        "--width".to_string(),
        ctx.width.to_string(),
        "--height".to_string(),
        ctx.height.to_string(),
    ]);

    args
}
