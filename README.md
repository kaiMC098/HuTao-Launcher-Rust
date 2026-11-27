<p align="center">
  <img src="assets/hutao_banner.png" alt="HuTao Launcher Banner" width="100%" />
</p>

<h1 align="center">🔥 HuTao Launcher (Rust Edition)</h1>

<p align="center">
  <strong>Trải nghiệm Minecraft: Java Edition mượt mà ngay trên Android với giao diện phong cách Hu Tao (Genshin Impact) rực lửa và lõi xử lý viết lại bằng Rust siêu nhẹ, tối ưu hiệu năng đa nhân cho chip MediaTek Dimensity &amp; Snapdragon.</strong>
</p>

---

## ✨ Giới thiệu

Khám phá thế giới *Minecraft: Java Edition* trên điện thoại theo một diện mạo hoàn toàn mới với **HuTao Launcher**! Dự án được tái cấu trúc toàn bộ tầng native sang ngôn ngữ **Rust** cho tốc độ phản hồi nhanh và ổn định, tích hợp cơ chế tự động nhận diện và tối ưu cụm nhân hiệu năng cao cho dòng chip **MediaTek Dimensity** cùng nhiều vi xử lý Android khác, hỗ trợ đầy đủ modpack (**Forge**, **Fabric**, **Quilt**, **OptiFine**) trong giao diện Đỏ Hỏa – Vàng Kim đặc trưng của Đường chủ Vãng Sinh Đường.

### Tính năng nổi bật
* **Lõi Native 100% Rust (`rust/crates/*`)**: Thay thế mã C/JNI truyền thống bằng Rust Cargo Workspace (`pojav_core`, `pojav_ui`, `pojav_native`, `pojav_awt`, `pojav_hooks`).
* **Tối ưu tự động theo SoC (`soc_optimizer.rs`)**:
  * **MediaTek Dimensity & Helio (GPU Mali / Immortalis)**: Tự động cấu hình OpenGL ES / GL4ES & LTW, tắt xung đột driver Turnip và ghim luồng dựng hình vào toàn bộ cụm nhân **Big & Prime** giúp tải chunk mượt mà.
  * **Qualcomm Snapdragon (GPU Adreno)**: Hỗ trợ tự động nạp driver Vulkan Turnip cho Zink và tối ưu bộ nhớ đệm shader.
* **Giao diện Wangsheng Pyro Edition**: Tông màu Đỏ Hỏa (`#D93829`) – Vàng Đèn Lồng (`#F5B041`) trên nền nhung tối (`#120A0C`) cùng bộ avatar & banner nhân vật Hu Tao.
* **Tự động build APK trên GitHub Actions**: Đẩy code lên GitHub và tải trực tiếp tệp `HuTao-Launcher-debug.apk` tại tab **Actions → Artifacts**.

---

## 🛠️ Cấu trúc mã nguồn

```text
HuTao-Launcher-Rust/
├── .github/workflows/build-apk.yml   # Tự động biên dịch Rust 4 ABI & đóng gói APK
├── assets/hutao_banner.png           # Banner giới thiệu HuTao Launcher
├── app_pojavlauncher/                # Giao diện Android, tài nguyên Hu Tao & cầu nối JNI
└── rust/crates/
    ├── pojav_core/                   # Xác thực, quản lý Profile, Modrinth & phân loại SoC
    ├── pojav_ui/                     # Máy trạng thái giao diện & bảng màu Hu Tao
    ├── pojav_native/                 # Khởi chạy OpenJDK, Input RingBuffer & cầu nối EGL/OSMesa
    ├── pojav_awt/                    # Cầu nối Caciocavallo cho trình cài mod
    └── pojav_hooks/                  # Nạp driver Vulkan & hook tiến trình
```

## 📜 Giấy phép & Ghi công

Phát hành theo giấy phép **GNU GPLv3 / LGPLv3** (xem tệp `LICENSE`). Xây dựng dựa trên nền tảng mã nguồn mở của PojavLauncher, Boardwalk, OpenJDK, LWJGL3, GL4ES và Mesa 3D.
