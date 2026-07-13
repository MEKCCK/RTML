本项目 RustedTuiMcLauncher (RTML) 基于 rmcl 项目开发

一个用 Rust 编写的终端用户界面 (TUI) Minecraft 启动器，支持 BMCLAPI 镜像加速。

## 功能特性

- **多实例管理** - 创建、删除、重命名游戏实例，每个实例独立隔离
- **多加载器支持** - Vanilla、Fabric、Forge、NeoForge、Quilt
- **模组下载** - 集成 Modrinth API，支持搜索、分类筛选、一键下载
- **内容管理** - 管理模组、资源包、光影包、存档、截图
- **账户系统** - Microsoft 账户登录
- **镜像加速** - 支持 BMCLAPI 镜像加速下载
- **日志查看** - 实时查看游戏日志，支持搜索过滤

## 安装

### 从源码构建

```bash
# 克隆仓库
git clone <repo-url>
cd RustedTuiMcLauncher

# 构建 Release 版本
cargo build --release

# 二进制文件位于
./target/release/rtml
```

### 依赖要求

- Rust 1.85+ (edition 2024)
- Java (用于运行 Minecraft)

## 使用方法

```bash
./target/release/rtml
```

## 快捷键

### 全局快捷键

| 按键 | 功能 |
|------|------|
| `1-4` | 切换面板 (实例/内容/账户/设置) |
| `b` | 打开模组下载 (Modrinth) |
| `?` / `h` | 显示/隐藏帮助 |
| `q` | 退出程序 |
| `Esc` | 返回 / 关闭弹窗 |

### 实例面板

| 按键 | 功能 |
|------|------|
| `Enter` / `Space` | 启动游戏 |
| `a` | 新建实例 |
| `i` | 导入整合包 |
| `d` | 删除实例 |
| `r` | 重命名实例 |
| `o` | 打开实例目录 |
| `/` | 搜索实例 |
| `↑↓` / `jk` | 上下导航 |

### 内容面板

| 按键 | 功能 |
|------|------|
| `←→` / `hl` | 切换标签 (模组/资源包/光影/截图/存档/日志) |
| `Space` | 切换模组启用/禁用 |
| `d` | 删除选中内容 |
| `o` | 打开目录 |
| `/` | 搜索 |

### 模组下载

| 按键 | 功能 |
|------|------|
| `/` | 搜索模组 |
| `←→` / `hl` | 切换分类 |
| `Enter` | 查看版本 / 下载模组 |
| `PageUp/Down` | 翻页 |
| `jk` | 上下导航 |

### 日志覆盖层

| 按键 | 功能 |
|------|------|
| `O` / `Esc` | 关闭日志 |
| `g` / `G` | 跳转到顶部/底部 |
| `/` | 搜索日志 |

## 项目结构

```
src/
├── auth/           # 账户认证
├── cli/            # 命令行接口
├── config/         # 配置管理
├── instance/       # 实例管理
│   ├── content/    # 内容管理 (模组/资源包等)
│   ├── launch/     # 游戏启动
│   └── loader/     # 加载器安装
├── net/            # 网络层
│   ├── modrinth.rs # Modrinth API
│   ├── mojang.rs   # Mojang API
│   ├── fabric.rs   # Fabric API
│   ├── forge.rs    # Forge API
│   ├── neoforge.rs # NeoForge API
│   └── quilt.rs    # Quilt API
└── tui/            # TUI 界面
    ├── widgets/    # UI 组件
    └── popups/     # 弹窗
```

## 配置

配置文件位于 `~/.config/rtml/config.toml`，可配置：

- 实例存储路径
- 元数据存储路径
- 下载源 (Mojang / BMCLAPI)
- Java 路径
- 内存设置
- 主题颜色

## 许可证

GPL-3.0 License
