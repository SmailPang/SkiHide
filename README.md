# SkiHide

<p align="center">
  <img src="https://raw.githubusercontent.com/SmailPang/SkiHide/refs/heads/main/icon.ico" alt="SkiHide Logo" width="180">
</p>

[![Ask DeepWiki](https://deepwiki.com/badge.svg)](https://deepwiki.com/SmailPang/SkiHide)

**SkiHide** 是一款专为 Windows 设计的桌面窗口管理工具，  
通过快捷键或鼠标侧键，一键隐藏 / 恢复指定窗口，  
适合 **双屏用户、游戏玩家、演示场景、~摸鱼~（划掉）高效办公** 等使用场景。

> 当前版本基于 **Tauri 2 + Vue 3 + Rust** 全面重构，体积更小、启动更快、资源占用更低。

---

## ✨ 功能特性

- 🪟 **一键隐藏 / 恢复窗口**
  - 支持指定窗口或「当前前台窗口」
  - 再次触发即可恢复

- ⌨️ **全局快捷键**
  - 自定义组合键，系统级监听
  - 不影响当前应用的输入

- 🖱️ **鼠标侧键支持**
  - 支持 X1 / X2 侧键触发
  - 通过底层鼠标钩子全局生效，无需键盘操作

- 🔇 **隐藏窗口时自动静音（可选）**
  - 隐藏时关闭系统声音
  - 恢复时自动还原原状态

- ⏸️ **隐藏前自动暂停（可选）**
  - 隐藏前向目标窗口发送指定快捷键（如媒体暂停键）
  - 适合视频、音乐、游戏场景

- 🚀 **开机自启 / 静默启动 / 启动时自动监听**
  - 启动后自动隐藏到托盘
  - 可在启动时直接进入监听状态

- 🧠 **智能窗口过滤**
  - 自动过滤系统窗口 / 无效窗口
  - 仅显示可操作的正常应用窗口

- 🧰 **附加工具箱**
  - 内存清理与状态监控
  - 系统缓存 / 临时文件 / 缩略图 / 应用缓存 / 回收站 清理

- 🌐 **多语言支持**
  - 简体中文 / 繁體中文 / English / 日本語
  - 首次启动自动跟随系统语言

- 🎨 **主题与字号自定义**
  - 浅色 / 深色 / 跟随系统
  - 4 档字号自由切换

- 🔄 **自动更新**
  - 支持 Mirror 酱 / SkiHide 官方源
  - 支持 GitHub / CNB 多下载源回退
  - 下载后 SHA256 校验，自动替换并重启

- 📦 **轻量免安装**
  - 单文件 EXE，无需运行环境

- 🔐 **隐私合规**
  - 首次启动强制提示隐私政策与免责说明
  - 不同意将无法使用软件

---

## 🖥️ 使用场景示例

- 游戏中一键隐藏攻略窗口
- 演示 / 投屏时快速隐藏私人内容
- 双屏办公临时收起副屏窗口
- 录屏 / 直播前快速清理界面

---

## 🚀 使用方法

1. 下载最新版本的 `SkiHide.exe`
2. 双击运行（首次启动需同意隐私政策）
3. 在窗口列表中选择要隐藏的窗口，或保留默认的「当前前台窗口」
4. 设置快捷键，或勾选鼠标侧键监听
5. 点击「开始监听」

---

## ⚙️ 设置说明

- **设置快捷键**
  - 点击输入框后直接按组合键即可录制

- **使用鼠标侧键**
  - 勾选后可用鼠标 X1 / X2 键触发

- **隐藏前暂停**
  - 在「设置」中开启并录制要发送的暂停热键

- **隐藏后关闭声音**
  - 在「设置」中开启

- **开机自启 / 静默启动**
  - 通过注册表 `Run` 项实现，可选择启动后是否直接隐藏到托盘

- **启动时自动监听**
  - 程序启动后自动进入监听状态，无需手动点击

---

## 🔒 隐私政策与免责说明

SkiHide 在首次启动时会提示并要求用户阅读并同意 [隐私政策与免责说明](https://skihide.xyz/guide/privacy) 。

📄 在线查看地址：
👉 https://skihide.xyz/guide/privacy

---

## 🐞 问题反馈

如果你在使用过程中遇到问题、Bug 或有功能建议，  
请通过 GitHub Issues 进行反馈。

---

## 🛠️ 开发与构建（开发者）

### 技术栈

- **前端**：Vue 3 + TypeScript + Vite 5 + vue-i18n
- **后端**：Tauri 2 + Rust 2021（`windows` crate 调用 Win32 API）
- **包管理**：pnpm

### 环境要求

- Node.js 18+
- pnpm 10+
- Rust（stable，含 `cargo` 与 `rustup`）
- Windows 10 / 11（含 WebView2 运行时）
- Visual Studio Build Tools（含 MSVC 工具链）

### 安装依赖

```bash
pnpm install
```

### 本地开发

```bash
pnpm tauri dev
```

> 该命令会先启动 Vite 开发服务（`http://localhost:1420`），再以开发模式拉起 Tauri 主进程。

### 构建发行版

```bash
pnpm tauri build
```

构建产物位于：

```
src-tauri/target/release/
src-tauri/target/release/bundle/
```

### 清理产物

```bash
pnpm clean
```

会清理 `dist/`、`src-tauri/target/` 等本地构建缓存。

### 项目结构

```
SkiHide/
├── src/                    # 前端 (Vue 3)
│   ├── App.vue             # 主界面
│   ├── i18n/               # 多语言文案
│   └── types/              # 共享类型定义
├── src-tauri/              # 后端 (Rust + Tauri)
│   ├── src/
│   │   ├── main.rs         # 入口 & Tauri commands
│   │   ├── window_ops.rs   # 窗口枚举 / 隐藏 / 显示
│   │   ├── mouse_hook.rs   # 鼠标侧键全局钩子
│   │   ├── audio_ops.rs    # 系统静音控制
│   │   ├── config.rs       # 注册表读写
│   │   ├── update_ops.rs   # 自动更新
│   │   └── ...
│   └── tauri.conf.json
└── package.json
```

---

## 📄 开源许可

本项目基于 **MIT License** 开源。

---

## ❤️ 致谢

感谢所有测试与反馈的用户，  
SkiHide 会持续迭代与改进。

如果这个项目对你有帮助，欢迎点一个 ⭐️！
