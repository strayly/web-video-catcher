# 网页视频捕手 (Web Video Catcher)

[English](./README_EN.md) | 简体中文

一款桌面端「网页视频抓取与下载」工具。内置一个捕获浏览器，访问视频页面后自动嗅探出页面里的媒体直链（MP4 / M3U8 / 分片等），按需下载，并支持音视频合并。

支持抖音、快手、B站等主流视频网站，以及各类普通视频页面。

基于 [Tauri 2](https://v2.tauri.app/) 构建，后端 Rust、前端 TypeScript，体积小、无 Electron 重运行时。

## 功能特性

- **捕获浏览器**：内置 WebView2 窗口加载目标网址，不弹窗也能在后台捕获媒体流（manifest / 分片）。
- **自动嗅探**：劫持页面的 `video.src` 赋值与原型 `setter`，并回收 `performance resource timing`，覆盖 B 站、抖音、TikTok、快手等动态渲染页面的媒体直链。
- **媒体分类**：按扩展名 / 路径特征识别 MP4、M3U8 等类型，去重后推送到列表。
- **一键下载**：原生 HTTP 下载（基于 `reqwest`，走系统 schannel / ring，无外部依赖、报错可读），用页面 Origin 作 Referer 绕过 CDN 防盗链。
- **音视频合并**：支持把分离的音视频流合并为单个文件。
- **大小探测**：后台只读请求探测媒体长度，刷新下载卡片信息。
- **HTTPS 解密**：内置自签 CA（纯 Rust `rcgen` 生成），用于捕获通道的 HTTPS 流量。

## 使用方法

### 开发调试

```bash
npm install
npm run tauri dev      # 启动开发模式（带热更新）
```

### 打包发布

```bash
npm install
npm run tauri build    # 产出 Windows 安装包（nsis）
```

### 基本操作

1. 打开软件，进入「嗅探」页。
2. 在捕获浏览器里输入视频页面网址并访问。
3. 页面里的媒体直链会自动出现在列表中，点击「下载」即可。
4. 在「下载管理」页查看进度、合并结果与文件大小。
5. 在「设置」页调整 UA、捕获行为等选项。

## 技术栈

- **框架**：Tauri 2
- **后端**：Rust（捕获代理、下载、证书、合并）
- **前端**：TypeScript + Vite
- **能力**：WebView2（Windows）、系统 TLS

## 目录结构

```
.
├── index.html              # 前端入口
├── package.json            # 前端依赖与脚本
├── vite.config.ts          # Vite 配置
├── tsconfig.json           # TS 配置
├── src/                    # 前端源码
│   ├── main.ts
│   ├── styles.css
│   ├── components/         # 嗅探 / 下载 / 设置 视图
│   ├── types/              # 类型定义
│   └── utils/              # IPC 封装
├── src-tauri/              # Rust 后端
│   ├── Cargo.toml
│   ├── tauri.conf.json
│   ├── capabilities/       # Tauri 权限配置
│   ├── icons/              # 应用图标
│   └── src/                # 捕获 / 下载 / 证书 / 合并 等模块
└── zfb.png                 # 支付宝打赏码
```

## 协议

本项目基于 [MIT License](./LICENSE) 开源。

Copyright © 2026 strayly

---

## 支持作者

如果这个工具对你有帮助，欢迎扫码打赏 ☕

![支付宝打赏](zfb.png)
