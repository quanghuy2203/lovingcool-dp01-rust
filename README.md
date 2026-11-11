# LovingCool DP-01 Studio — Rust port (v0.2.1 prototype)

> Windows 10/11, x64. **Bản source chưa được kiểm tra biên dịch/chạy trên DP-01 thật**. Không điều khiển LED/RGB; chỉ có điều khiển màn LCD USB đã nhận diện, và phần hiển thị thông số PC.

## Thiết kế

Refactor từ `LovingCool1.rar` (C# WinForms, `MainForm.cs`, `HardwareSensors.cs`, `IndependentSceneWallpaper.cs`, `IndependentWebWallpaper.cs`, `FrameDiskCache.cs`). Mã nguồn Rust được viết mới thay cho WinForms. **Không chuyển** `MainForm.LedStudio.cs` và `Fix_RGB_Duplicates.*`.

- GUI native `eframe/egui` — không dùng Electron, không nhúng WebView2 vào UI Rust.
- Theo dõi CPU usage, CPU name, RAM, disk, tốc độ mạng, uptime qua `sysinfo` mỗi 2 giây. Nhiệt độ CPU/GPU và GPU load là tùy chọn qua web server `LibreHardwareMonitor` `http://127.0.0.1:8085/data.json`; cần cài và bật web server của LHM. Không cam kết mọi máy đều có cảm biến nhiệt độ.
- JPG / PNG, GIF từng frame, preview dashboard và overlay thông số PC lên hình LCD.
- Wallpaper Engine **Scene / Web qua chương trình chính chủ**; dùng tham số `-control openWallpaper -file ... -playInWindow ...`. Đầu ra LCD dựa trên chụp cửa sổ qua Win32 GDI. Cửa sổ render GPU/WebGL có thể không chụp được (đen hoặc đứng hình). Đây **chưa phải** render Scene/Web độc lập.
- USB/COM **chỉ** nhận diện VID/PID `33C3:7791` và `33C3:7788`, xác thực thông tin thiết bị trước khi gửi ảnh. Không đảm bảo DP-01 của bạn thuộc loại này.
- Công việc cảm biến/ảnh ở thread riêng, hàng chờ ảnh giới hạn 2 thông báo, FPS 1–15, preview 2 giây khi Dashboard đứng yên; không giữ toàn bộ frame GIF trong RAM. GIF disposal "Previous" chưa hoàn chỉnh; một số GIF có thể sai hình.
- Settings lưu tại `%LOCALAPPDATA%\LovingCoolRust\settings.json`.
- GitHub Releases: nút cập nhật trong ứng dụng gọi PowerShell khi người dùng yêu cầu; không chạy dịch vụ kiểm tra cập nhật suốt thời gian idle. Release ZIP có checksum SHA-256; kiểm tra checksum bảo vệ lỗi tải dữ liệu, **không phải chữ ký mật mã chống việc tài khoản GitHub bị chiếm**.

### Yêu cầu / Build

Cài [Rust](https://rustup.rs/) và **Visual Studio C++ Build Tools / Desktop development with C++** trên Windows.

```powershell
cargo build --release
.\target\release\lovingcool-dp01-studio.exe
```

Để thử updater khi build thủ công, copy `update.ps1` vào cùng thư mục với `.exe`. Bản phát hành tự đóng gói cả hai.

### Đưa lên GitHub của bạn

Cần [Git for Windows](https://git-scm.com/), [GitHub CLI](https://cli.github.com/) và đăng nhập:

```powershell
gh auth login
.\publish-github.ps1 -Repo lovingcool-dp01-rust
```

Script **tạo repository mới** trong tài khoản `gh` đang đăng nhập và push `main` (mặc định public; dùng `-Private` nếu muốn private). Nếu tên repo đã tồn tại, script sẽ lỗi, không ghi đè. Trong app, trường `Repository` mặc định là `quanghuy2203/lovingcool-dp01-rust`, có thể sửa.

### Phát hành và cập nhật tự động

- Sau khi mã nguồn được đẩy lên nhánh `main`, GitHub Actions tự kiểm tra, build Windows, đóng gói EXE và tạo Release khi `version` trong `Cargo.toml` là phiên bản mới chưa có tag.
- Không cần chạy lệnh `git tag` hay `git push origin <tag>` thủ công để phát hành. Tăng `Cargo.toml` version cho mỗi bản mới.
- Trong ứng dụng, nhấn **Kiểm tra, tải và cài bản mới**. Updater sẽ tải Release mới nhất, xác minh SHA-256, giải nén trước, đóng app, thay EXE rồi khởi động lại. Cài đặt/giải nén phải nằm tại thư mục mà người dùng có quyền ghi.
- Không cần Git hay GitHub CLI trên máy sử dụng ứng dụng. Lần đầu cần tải bản Release, những lần sau cập nhật từ giao diện.
- Nút cập nhật chỉ hoạt động với gói Release có ZIP và checksum; SHA-256 xác minh tính toàn vẹn, không thay thế chữ ký phần mềm.

### Giới hạn quan trọng

1. Chưa có `.exe` build/kiểm thử từ môi trường này; GitHub CI sẽ cho biết lỗi compile nếu có.
2. Driver LCD chưa xác minh cụ thể cho Loving Cool DP-01.
3. Tính năng Scene/Web chất lượng cao cần Windows Graphics Capture (WGC)/GPU capture thay GDI; lệnh mở wallpaper là phần đã có, chưa có đường capture WGC ổn định.
4. Mục tiêu tối ưu RAM chưa được benchmark bằng Task Manager; Wallpaper Engine chạy trong tiến trình riêng nên tổng RAM cần tính cả tiến trình đó.
5. Script tự cập nhật là prototype; nên phát hành bằng repository bạn sở hữu và cân nhắc ký EXE/installer trước khi sử dụng rộng rãi.
