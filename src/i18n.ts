
// 多语言内核: zh-CN / en-US 双语, 当前语言存 localStorage, 首启按系统语言。
// 文本以 key 索引, t(key, vars) 做 {name} 插值。

export type Lang = "zh" | "en";

type Dict = Record<string, string>;

const ZH: Dict = {
  "app.title": "网页视频捕手",
  "nav.sniffer": "嗅探",
  "nav.downloads": "下载管理",
  "nav.settings": "设置",

  "sniffer.proxyOff": "代理 已关闭",
  "sniffer.proxyOn": "代理 已开启 · {addr}",
  "sniffer.capCount": "抓包 {n} 路",
  "sniffer.captureToggle": "抓包",
  "sniffer.urlPlaceholder": "粘贴视频页 URL, 点「捕获」用内置浏览器打开(抖音等需签名的站点)",
  "sniffer.btnCapture": "捕获",
  "sniffer.chkPopup": "弹窗",
  "sniffer.popupTitle": "勾选则弹出可见窗口(便于手动播放/调试), 默认不弹窗后台捕获",
  "sniffer.capRawBtn": "原始窗口",
  "sniffer.capRawTitle": "无注入打开, 排查白屏",
  "sniffer.capClose": "关闭捕获",
  "sniffer.capOpening": "正在打开捕获窗口…",
  "sniffer.capOpeningRaw": "正在打开原始窗口(无注入)…",
  "sniffer.capOpeningBg": "正在后台启动捕获(不弹窗)…",
  "sniffer.mediaTitle": "检测到的媒体",
  "sniffer.selAll": "全选",
  "sniffer.merge": "合并",
  "sniffer.mergeTitle": "勾选 1 条视频 + 1 条音频, 合成为带声音的 mp4",
  "sniffer.delSel": "删除选中",
  "sniffer.clearAll": "清空全部",
  "sniffer.mergeHint": "这是音视频分离(DASH)站点: 单独下载视频没有声音。勾选 1 条视频 + 1 条音频, 点「合并」即可合成带声音的 mp4",
  "sniffer.empty": "暂无。点「捕获」用内置浏览器播放网页视频即可自动抓直链。",
  "sniffer.pleasePaste": "请先粘贴视频页地址, 再点「捕获」。",
  "sniffer.openFailed": "打开失败: ",
  "sniffer.closeOk": "捕获窗口已关闭。",
  "sniffer.closeFailed": "关闭失败: ",
  "sniffer.mergeNeed2": "合并需要勾选恰好 2 项(1 条视频 + 1 条音频), 当前选中 {n} 项。",
  "sniffer.mergeBothAudio": "两条都是音频, 没有画面可放 —— 请改选 1 条视频 + 1 条音频。",
  "sniffer.mergeBothVideo": "两条都是视频, 合并后依然没有声音 —— 请改选 1 条视频 + 1 条音频。",
  "sniffer.delSelEmpty": "请先勾选要删除的媒体项。",
  "sniffer.clearConfirm": "清空全部嗅探到的媒体？(只是列表, 不影响已下载文件, 可重新捕获)",
  "sniffer.copy": "复制",
  "sniffer.download": "下载",

  "kind.mp4": "mp4",
  "kind.m3u8": "m3u8",
  "kind.ts": "ts",
  "kind.dash": "dash",
  "kind.audio": "音频",
  "kind.other": "video",
  "kind.unknown": "video",

  "media.from": "来自 {source}",
  "media.sizeUnknown": "大小未知",

  "dl.queued": "排队中",
  "dl.merging": "合并中",
  "dl.downloading": "下载中 {pct}%",
  "dl.statusDownloading": "下载中",
  "dl.paused": "已暂停",
  "dl.done": "已完成",
  "dl.failed": "下载失败",
  "dl.cancelled": "已取消",
  "dl.cancel": "取消",
  "dl.play": "播放",
  "dl.folder": "文件夹",
  "dl.retry": "重新下载",
  "dl.title": "下载任务",
  "dl.delSel": "删除选中",
  "dl.clearDone": "清空已完成",
  "dl.delSelEmpty": "请先勾选要删除的任务。",
  "dl.noDone": "没有已完成的任务。",
  "dl.empty": "暂无下载任务。在「嗅探」里点「下载」即可。",
  "dl.delete": "删除",
  "dl.deleteTitle": "仅从列表移除, 不删本地文件",

  "set.basic": "基础",
  "set.proxyPort": "代理端口",
  "set.proxyPortNote": "默认 8888",
  "set.downloadDir": "下载目录",
  "set.downloadDirPlaceholder": "留空 = 程序目录下 downloads",
  "set.cookieBrowser": "Cookie 浏览器",
  "set.cookieNone": "不使用(部分站点需登录)",
  "set.cookieNote": "供 yt-dlp 解析页面用; 捕获到的直链会自动复用浏览器已带的 Cookie",
  "set.engineGroup": "引擎",
  "set.ytDlpFfmpeg": "yt-dlp / ffmpeg",
  "set.testBtn": "检测",
  "set.engineNotTested": "未检测",
  "set.engineInfo": "yt-dlp: {ok} ({ver})  ffmpeg: {ok2}",
  "set.noteEngine": "捕获到的视频直链由程序内置下载器直连取流(带浏览器同款请求头), 不需要 yt-dlp; yt-dlp 只用于解析网页地址、下载 m3u8 分片并合并。",

  "cap.navigating": "导航: {u}",
  "cap.loading": "{stage}: {url}",
  "cap.titleChanged": "页面标题: {t}",
  "cap.noMedia12s": "⏳ 12 秒内未检测到媒体直链: 请勾选「弹窗」让捕获窗口可见, 并在窗口内点击播放(TikTok 等站点需可见窗口才发取流请求)",
  "cap.xhsLogin": "⏳ 小红书视频需登录后才能播放: 请勾选「弹窗」, 在弹出的窗口里登录小红书, 登录成功后再点「捕获」重试",
  "cap.done": "捕获完成! 共 {n} 条",

  "common.ok": "OK",
  "common.missing": "缺失",
  "set.notInstalled": "未安装",
  "media.unknownTitle": "未知标题",
  "q.dolbyAtmos": "杜比全景声",
  "q.dolbyVision": "杜比视界",
};

const EN: Dict = {
  "app.title": "Web Video Catcher",
  "nav.sniffer": "Sniffer",
  "nav.downloads": "Downloads",
  "nav.settings": "Settings",

  "sniffer.proxyOff": "Proxy Off",
  "sniffer.proxyOn": "Proxy On · {addr}",
  "sniffer.capCount": "Capturing {n}",
  "sniffer.captureToggle": "Capture",
  "sniffer.urlPlaceholder": "Paste video page URL, click Capture to open it in the built-in browser (sites like Douyin need signatures)",
  "sniffer.btnCapture": "Capture",
  "sniffer.chkPopup": "Popup",
  "sniffer.popupTitle": "Show a visible window (for manual playback/debug); hidden background capture by default",
  "sniffer.capRawBtn": "Raw window",
  "sniffer.capRawTitle": "Open without injection, for blank-screen troubleshooting",
  "sniffer.capClose": "Close capture",
  "sniffer.capOpening": "Opening capture window…",
  "sniffer.capOpeningRaw": "Opening raw window (no injection)…",
  "sniffer.capOpeningBg": "Starting background capture (hidden)…",
  "sniffer.mediaTitle": "Detected Media",
  "sniffer.selAll": "Select all",
  "sniffer.merge": "Merge",
  "sniffer.mergeTitle": "Select 1 video + 1 audio to merge into an mp4 with sound",
  "sniffer.delSel": "Delete selected",
  "sniffer.clearAll": "Clear all",
  "sniffer.mergeHint": "This is a split A/V (DASH) site: downloading video alone has no sound. Select 1 video + 1 audio and click Merge to combine into an mp4 with sound",
  "sniffer.empty": "Empty. Click Capture and play a web video in the built-in browser to auto-grab direct links.",
  "sniffer.pleasePaste": "Paste the video page URL first, then click Capture.",
  "sniffer.openFailed": "Open failed: ",
  "sniffer.closeOk": "Capture window closed.",
  "sniffer.closeFailed": "Close failed: ",
  "sniffer.mergeNeed2": "Merge needs exactly 2 items (1 video + 1 audio); you selected {n}.",
  "sniffer.mergeBothAudio": "Both are audio, no video to embed — pick 1 video + 1 audio.",
  "sniffer.mergeBothVideo": "Both are video; merged result still has no sound — pick 1 video + 1 audio.",
  "sniffer.delSelEmpty": "Select media items to delete first.",
  "sniffer.clearConfirm": "Clear all sniffed media? (List only; downloaded files are untouched; re-capture anytime)",
  "sniffer.copy": "Copy",
  "sniffer.download": "Download",

  "kind.mp4": "mp4",
  "kind.m3u8": "m3u8",
  "kind.ts": "ts",
  "kind.dash": "dash",
  "kind.audio": "audio",
  "kind.other": "video",
  "kind.unknown": "video",

  "media.from": "from {source}",
  "media.sizeUnknown": "size unknown",

  "dl.queued": "Queued",
  "dl.merging": "Merging",
  "dl.downloading": "Downloading {pct}%",
  "dl.statusDownloading": "Downloading",
  "dl.paused": "Paused",
  "dl.done": "Done",
  "dl.failed": "Failed",
  "dl.cancelled": "Cancelled",
  "dl.cancel": "Cancel",
  "dl.play": "Play",
  "dl.folder": "Folder",
  "dl.retry": "Redownload",
  "dl.title": "Download Tasks",
  "dl.delSel": "Delete selected",
  "dl.clearDone": "Clear completed",
  "dl.delSelEmpty": "Select tasks to delete first.",
  "dl.noDone": "No completed tasks.",
  "dl.empty": "No download tasks. Click Download in Sniffer to start.",
  "dl.delete": "Delete",
  "dl.deleteTitle": "Remove from list only; local file kept",

  "set.basic": "Basic",
  "set.proxyPort": "Proxy port",
  "set.proxyPortNote": "Default 8888",
  "set.downloadDir": "Download dir",
  "set.downloadDirPlaceholder": "Blank = downloads under app dir",
  "set.cookieBrowser": "Cookie browser",
  "set.cookieNone": "None (some sites need login)",
  "set.cookieNote": "Used by yt-dlp to parse pages; captured direct links auto-reuse the browser's cookies",
  "set.engineGroup": "Engine",
  "set.ytDlpFfmpeg": "yt-dlp / ffmpeg",
  "set.testBtn": "Check",
  "set.engineNotTested": "Not checked",
  "set.engineInfo": "yt-dlp: {ok} ({ver})  ffmpeg: {ok2}",
  "set.noteEngine": "Captured direct links are fetched by the built-in downloader (with browser-like headers); yt-dlp is only for parsing pages and merging m3u8.",

  "cap.navigating": "Navigating: {u}",
  "cap.loading": "{stage}: {url}",
  "cap.titleChanged": "Page title: {t}",
  "cap.noMedia12s": "⏳ No media direct link detected in 12s: tick Popup to show the capture window, and click play inside it (sites like TikTok only emit streams in a visible window)",
  "cap.xhsLogin": "⏳ Xiaohongshu videos require login: tick Popup, log in to Xiaohongshu in the window, then click Capture again",
  "cap.done": "Capture done! {n} items",

  "common.ok": "OK",
  "common.missing": "Missing",
  "set.notInstalled": "not installed",
  "media.unknownTitle": "Unknown title",
  "q.dolbyAtmos": "Dolby Atmos",
  "q.dolbyVision": "Dolby Vision",
};

const DICTS: Record<Lang, Dict> = { zh: ZH, en: EN };

const STORE_KEY = "wvc.lang";

let lang: Lang = pickInitial();

function pickInitial(): Lang {
  try {
    const saved = localStorage.getItem(STORE_KEY);
    if (saved === "zh" || saved === "en") return saved;
  } catch {}
  const nav = (navigator.language || "zh").toLowerCase();
  return nav.startsWith("zh") ? "zh" : "en";
}

export function getLang(): Lang {
  return lang;
}

export function setLang(l: Lang): void {
  lang = l;
  try {
    localStorage.setItem(STORE_KEY, l);
  } catch {}
  document.documentElement.lang = l === "zh" ? "zh-CN" : "en";
  window.dispatchEvent(new CustomEvent("i18n-change"));
}

export function t(key: string, vars?: Record<string, string | number>): string {
  let s = DICTS[lang][key] ?? ZH[key] ?? key;
  if (vars) {
    for (const k in vars) {
      s = s.replace(new RegExp("\\{" + k + "\\}", "g"), String(vars[k]));
    }
  }
  return s;
}

// 用于捕获状态条: Rust 推来的可能是 {zh,en} 对象, 前端本地调用则传已翻译字符串。
export function pickBi(msg: string | { zh: string; en: string }): string {
  if (typeof msg === "string") return msg;
  return lang === "zh" ? msg.zh : msg.en;
}

// 用于错误文本: Rust 侧错误字符串可能是 '{"zh":..,"en":..}' JSON, 解析后按语言选显; 解析失败原样返回。
export function errText(s: string): string {
  if (!s) return "";
  try {
    const v = JSON.parse(s);
    if (v && typeof v === "object" && typeof v.zh === "string" && typeof v.en === "string") {
      return lang === "zh" ? v.zh : v.en;
    }
  } catch {}
  return s;
}

export function initI18n(): void {
  document.documentElement.lang = lang === "zh" ? "zh-CN" : "en";
  document.title = t("app.title");
}

export const LANG_LABELS: Record<Lang, string> = { zh: "中文", en: "English" };
