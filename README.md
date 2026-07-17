<div align="center">
  <img src="assets/icon.png" width="128" alt="RTML Logo" />
  <h1>RTML — Rusted TUI Minecraft Launcher</h1>
  <p>v1.0.0 — 一个基于 rmcl 和 BonNext 开发的 Rust TUI Minecraft 启动器</p>
</div>

## 简介

RTML 是一个使用 Rust 编写的终端用户界面（TUI）Minecraft 启动器，基于 [rmcl](https://github.com/objz/rmcl) 项目开发，并移植了 [BonNext](https://github.com/anomalyco/BonNextMinecraftLauncher-Rust) 的额外功能。

特色功能：
- **Modrinth 集成** — 搜索、筛选、一键下载模组
- **整合包导入** — 支持 Modrinth (.mrpack) 和 CurseForge 格式
- **BMCLAPI 镜像加速** — 国内用户友好
- **桌面快捷方式** — Linux .desktop / Windows VBS
- **多加载器** — Vanilla / Fabric / Forge / NeoForge / Quilt
- **陶瓦联机** — 基于 [Terracotta](https://github.com/burningtnt/Terracotta) 的 P2P 局域网联机，兼容 HMCL/PCL2 CE

## 从源码构建

```bash
git clone https://github.com/MEKCCK/RTML.git
cd RTML
cargo build --release
./target/release/rtml
```

**依赖**: Rust 1.85+ (edition 2024)、Java（运行 Minecraft）

## 打包

| 平台 | 脚本 | 用法 |
|------|------|------|
| Arch Linux | `scripts/PKGBUILD` | 本地打包：`makepkg` |
| Windows | `scripts/build.bat` | 双击或终端运行 |

## 快捷键

| 按键 | 功能 |
|------|------|
| `1-4` | 切换面板 |
| `t` | 局域网联机 (Terracotta) |
| `m` | 打开模组下载 (Modrinth) |
| `?` / `h` | 帮助 |
| `q` | 退出 |
| `Esc` | 返回 / 关闭弹窗 |

实例面板: `Enter` 启动 / `a` 新建 / `t` 联机 / `i` 导入整合包 / `d` 删除 / `r` 重命名 / `o` 打开目录  
内容面板: `Space` 切换启用 / `d` 删除 / `o` 打开目录  
模组下载: `/` 搜索 / `←→` 切换分类 / `Enter` 下载 / `PgUp/PgDn` 翻页

## 许可证

本项目基于 GPL-3.0 协议开源。

基于以下上游项目修改：
- [rmcl](https://github.com/objz/rmcl) — 原始 Rust TUI 启动器框架
- [BonNext](https://github.com/anomalyco/BonNextMinecraftLauncher-Rust) — Modrinth 集成、下载系统、整合包导入等代码移植
- [HMCL](https://github.com/HMCL-dev/HMCL) — 陶瓦联机架构参考 (TerracottaManager、状态机、守护进程生命周期)

See [LICENSE](./LICENSE) for full text.
