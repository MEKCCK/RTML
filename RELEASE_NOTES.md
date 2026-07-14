# v1.0.0 — 陶瓦联机正式发布

## 新功能
- **陶瓦联机 (Terracotta)** — 基于 Terracotta 的 P2P 局域网联机功能，兼容 HMCL / PCL2 CE
  - 创建房间（自动检测端口，无需手动选择）
  - 加入房间（支持邀请码）
  - 成员列表实时刷新
  - 离开房间 / 断开连接
- **Modrinth 集成** — 搜索、筛选、一键下载模组
- **整合包导入** — 支持 Modrinth (.mrpack) 和 CurseForge 格式
- **BMCLAPI 镜像加速** — 国内用户友好
- **桌面快捷方式** — Linux .desktop / Windows VBS
- **多加载器支持** — Vanilla / Fabric / Forge / NeoForge / Quilt

## 优化
- 联机流程对齐 HMCL：自动检测 LAN 端口，无需用户选择端口
- 离开房间时保持守护进程运行（可按 `d` 回到等待状态）
- 关闭弹窗（`Esc`）仅隐藏界面，不终止守护进程
- 改进首屏加载性能

## 修复
- 修复邀请码解析错误（移除严格的 hex 格式校验）
- 修复 Host 状态解析错误（显式 serde rename）
- 修复多线程死锁（分离 ONLINE_MANAGER 和 ONLINE_STATE 锁）

## 快捷键
| 按键 | 功能 |
|------|------|
| `t` | 打开联机弹窗 |
| `d` | 断开联机（回到等待状态） |
| `Esc` | 关闭弹窗 |
| `?` / `h` | 帮助 |

## 构建产物
- Linux 二进制: `target/release/rtml` (12M)
- Windows exe: `scripts/rtml.exe` (11M)
- Arch Linux PKG: `scripts/rtml-1.0.0-2-x86_64.pkg.tar.zst` (7.9M)

## 许可证
本项目基于 GPL-3.0 发布。
衍生自 [rmcl](https://github.com/objz/rmcl)，参考了 [BonNext](https://github.com/anomalyco/BonNextMinecraftLauncher-Rust) 的部分实现。
陶瓦联机功能参考 [HMCL](https://github.com/HMCL-dev/HMCL) 设计。
