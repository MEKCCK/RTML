## v0.1.0 — 初始版本

基于 [rmcl](https://github.com/objz/rmcl) 项目开发，并移植了 [BonNext](https://github.com/anomalyco/BonNextMinecraftLauncher-Rust) 的 Modrinth 集成与下载系统。

### 功能

- **多实例管理** — 创建、删除、重命名、搜索实例，每个实例独立隔离
- **多加载器支持** — Vanilla / Fabric / Forge / NeoForge / Quilt
- **Modrinth 模组下载** — 搜索、分类筛选、一键下载
- **整合包导入** — 支持 Modrinth (.mrpack) 和 CurseForge 格式
- **内容管理** — 模组、资源包、光影包、存档、截图管理
- **Microsoft 账户登录**
- **BMCLAPI 镜像加速** — 国内用户友好
- **桌面快捷方式** — Linux .desktop / Windows VBS
- **实时游戏日志查看** — 支持搜索过滤

### 下载

| 文件 | 说明 |
|------|------|
| `rtml-0.1.0-x86_64-windows.exe` | Windows 可执行文件（`dist/windows/build.ps1` 构建） |
| `rtml-0.1.0-2-x86_64.pkg.tar.zst` | Arch Linux 包（`dist/arch/PKGBUILD` 构建） |

### 构建方式

```bash
git clone https://github.com/MEKCCK/RTML.git
cd RTML
cargo build --release
./target/release/rtml
```

**依赖**: Rust 1.85+ (edition 2024), Java 17+

### 快捷键

| 按键 | 功能 |
|------|------|
| `1-4` | 切换面板 |
| `m` | 打开 Modrinth 模组下载 |
| `?` / `h` | 帮助 |
| `q` | 退出 |

详情参见 [README.md](./README.md)。

### 许可证

GPL-3.0-or-later
