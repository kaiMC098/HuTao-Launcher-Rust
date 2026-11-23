# PojavLauncher (Rust Native Architecture)

Dự án này đã chuyển đổi toàn bộ tầng native C/JNI (`src/main/jni`) và lõi nghiệp vụ sang **Rust Cargo Workspace** (`rust/`), sẵn sàng biên dịch tự động ra file `.apk` trên GitHub Actions.

## Cấu trúc mô-đun Rust (`rust/crates/`)
- `pojav_core`: Xác thực Microsoft OAuth & Offline UUID, quản lý Profile, tham số dòng lệnh JVM, tìm kiếm Modrinth và ánh xạ phím Android -> GLFW.
- `pojav_ui`: Máy trạng thái giao diện (UI State Machine & Screen Router).
- `pojav_native` (`libpojavexec.so`): Cầu nối JNI, `JLI_Launch` khởi chạy OpenJDK, bộ đệm vòng sự kiện đầu vào 8000 phần tử, cầu nối EGL/Vulkan và ghim nhân CPU xung nhịp cao (Big-Core Affinity).
- `pojav_awt` (`libpojavexec_awt.so`, `libawt_xawt.so`): Cầu nối AWT/Caciocavallo cho trình cài đặt mod (Forge/OptiFine).
- `pojav_hooks` (`liblinkerhook.so`, `libexithook.so`): Nạp driver Vulkan Turnip tùy chỉnh và hook tiến trình.

## Hướng dẫn xuất file `.apk` trên GitHub
1. Giải nén file `PojavLauncher-Rust.zip`.
2. Tạo một repository mới trên GitHub và tải toàn bộ thư mục bên trong lên.
3. Mở tab **Actions** trên GitHub -> chọn workflow **Build PojavLauncher Rust APK**.
4. Sau khi build hoàn tất, tải tệp `PojavLauncher-Rust-debug.apk` tại mục **Artifacts**.
