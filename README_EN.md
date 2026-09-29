# Web Video Catcher (网页视频捕手)

English | [简体中文](./README.md)

A desktop tool for grabbing and downloading web videos. It ships with a built-in capture browser: open a video page and it automatically sniffs out the media direct links (MP4 / M3U8 / segments, etc.), downloads them on demand, and supports merging separate audio and video streams.

Works with mainstream video sites such as Douyin, Kuaishou and Bilibili, as well as ordinary video pages of all kinds.

Built on [Tauri 2](https://v2.tauri.app/) — Rust backend, TypeScript frontend, small binary, no heavy Electron runtime.

## Features

- **Capture browser**: a built-in WebView2 window loads the target URL and captures media streams (manifests / segments) in the background, no popup needed.
- **Automatic sniffing**: hooks `video.src` assignment and the prototype `setter`, and collects `performance resource timing` entries — covering dynamically rendered pages such as Bilibili, Douyin, TikTok and Kuaishou.
- **Media classification**: identifies MP4, M3U8 and other types by extension / path pattern, deduplicates, then pushes them to the list.
- **One-click download**: native HTTP download (based on `reqwest`, using system schannel / ring — no external dependencies, readable errors), using the page origin as Referer to bypass CDN hotlink protection.
- **Audio / video merging**: merge separate audio and video streams into a single file.
- **Size probing**: read-only background requests probe media length to refresh download card info.
- **HTTPS interception**: built-in self-signed CA (pure Rust `rcgen`) for HTTPS traffic on the capture channel.

## Usage

### Development

```bash
npm install
npm run tauri dev      # start dev mode (hot reload)
```

### Build

```bash
npm install
npm run tauri build    # produce the Windows installer (nsis)
```

### Basic workflow

1. Open the app and go to the "Sniffer" page.
2. Enter a video page URL in the capture browser and visit it.
3. Media direct links on the page appear in the list automatically — click "Download".
4. Check progress, merged results and file sizes on the "Downloads" page.
5. Adjust UA and capture behavior options on the "Settings" page.

## Tech Stack

- **Framework**: Tauri 2
- **Backend**: Rust (capture proxy, download, certificates, merging)
- **Frontend**: TypeScript + Vite
- **Capabilities**: WebView2 (Windows), system TLS

## Project Layout

```
.
├── index.html              # frontend entry
├── package.json            # frontend deps & scripts
├── vite.config.ts          # Vite config
├── tsconfig.json           # TS config
├── src/                    # frontend source
│   ├── main.ts
│   ├── styles.css
│   ├── components/         # sniffer / downloads / settings views
│   ├── types/              # type definitions
│   └── utils/              # IPC helpers
├── src-tauri/              # Rust backend
│   ├── Cargo.toml
│   ├── tauri.conf.json
│   ├── capabilities/       # Tauri permission config
│   ├── icons/              # app icons
│   └── src/                # capture / download / certificate / merge modules
└── zfb.png                 # donation QR code (Alipay)
```

## License & Disclaimer

This project is provided free under a **Non-Commercial License**. Full terms in [LICENSE](./LICENSE).

- **Free & open source**: download, study and use for personal non-commercial purposes; redistribute the original copy with the license and copyright notice intact.
- **No commercial use**: without prior written authorization, do not sell, paid-integrate, offer paid download/transcode/hosting, monetize via ads/traffic, or redistribute for profit. For commercial collaboration, contact via the repo homepage.
- **Disclaimer**: provided "as is" with no warranty and no liability for consequences of use; downloading copyrighted content requires authorization and compliance with local law — copyright or compliance disputes are the user's sole responsibility.

Copyright © 2026 strayly. All rights reserved.

---

## Support the Author

If this tool helps you, feel free to buy me a coffee ☕

![Alipay donation](zfb.png)
