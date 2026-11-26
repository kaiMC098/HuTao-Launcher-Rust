pub mod auth;
pub mod controls;
pub mod models;
pub mod modloaders;
pub mod profiles;
pub mod soc;
pub mod versions;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_offline_uuid_generation() {
        let acc = auth::create_offline_account("Steve").unwrap();
        assert_eq!(acc.username, "Steve");
        assert!(!acc.is_microsoft);
        assert_eq!(acc.uuid.len(), 36);
    }

    #[test]
    fn test_android_to_glfw_keymap() {
        assert_eq!(controls::android_key_to_glfw(51), 87);
        assert_eq!(controls::android_key_to_glfw(62), 32);
    }
}
