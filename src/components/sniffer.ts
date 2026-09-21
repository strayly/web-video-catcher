


import { listen } from "@tauri-apps/api/event";
import { callCommand } from "../utils/ipc";
import type { MediaItem, DownloadTask } from "../types";

const KIND_LABEL: Record<string, string> = {
  Mp4: "mp4",
  M3u8: "m3u8",
  Ts: "ts",
  Dash: "dash",
  Audio: "音频",
  Other: "video",
};


function fmtSize(bytes: number): string {
  if (!bytes || bytes <= 0) return "";
  if (bytes >= 1073741824) return (bytes / 1073741824).toFixed(2) + " GB";
  if (bytes >= 1048576) return (bytes / 1048576).toFixed(1) + " MB";
  return Math.max(1, Math.round(bytes / 1024)) + " KB";
}

export function mountSniffer(root: HTMLElement, refresh: () => void): (m: MediaItem[], tasks: DownloadTask[]) => void {
  root.innerHTML = `
    <div class="statusbar">
      <span class="pill" id="pill-proxy"><span class="led off"></span> 代理 已关闭</span>
      <span class="pill warn"><span class="led"></span> 抓包 0 路</span>
      <span class="switch"><span>抓包</span><span class="sw off" id="sw-proxy"></span></span>
    </div>
    <div class="manual">
      <input id="manual-url" type="text" placeholder="粘贴视频页 URL, 点「捕获」用内置浏览器打开(抖音等需签名的站点)" />
      <button class="btn primary cap" id="btn-capture">🧲 捕获</button>
      <label class="chk" title="勾选则弹出可见窗口(便于手动播放/调试), 默认不弹窗后台捕获">
        <input type="checkbox" id="chk-popup" /> 弹窗
      </label>
    </div>
    <div class="capbar" id="cap-bar" style="display:none">
      <span class="cap-text" id="cap-status">正在打开捕获窗口…</span>
      <span class="cap-acts">
        <button class="btn ghost small" id="btn-cap-raw" title="无注入打开, 排查白屏">原始窗口</button>
        <button class="btn ghost small" id="btn-cap-close">关闭捕获</button>
      </span>
    </div>
    <div class="list-head">
      <h3 class="sec">检测到的媒体 <span class="mcount" id="m-count"></span></h3>
      <div class="batchbar">
        <label class="chk"><input type="checkbox" id="m-selall" /> 全选</label>
        <button class="btn primary small" id="m-merge" title="勾选 1 条视频 + 1 条音频, 合成为带声音的 mp4">🔀 合并</button>
        <button class="btn ghost small" id="m-del-sel">删除选中</button>
        <button class="btn ghost small" id="m-clear">清空全部</button>
      </div>
    </div>
    <div class="mergehint" id="merge-hint" style="display:none">
      🎬 这是音视频分离(DASH)站点: 单独下载视频<b>没有声音</b>。勾选 <b>1 条视频 + 1 条音频</b>, 点「🔀 合并」即可合成带声音的 mp4
    </div>
    <div id="media-list"></div>
  `;
  const listEl = root.querySelector("#media-list") as HTMLElement;
  const countEl = root.querySelector("#m-count") as HTMLElement;
  const sw = root.querySelector("#sw-proxy") as HTMLElement;
  const pill = root.querySelector("#pill-proxy") as HTMLElement;

  
  const selected = new Set<string>();
  
  let currentMedia: MediaItem[] = [];

  
  const capBar = root.querySelector("#cap-bar") as HTMLElement;
  const capStatus = root.querySelector("#cap-status") as HTMLElement;
  let capLines: string[] = [];
  const pushCap = (msg: string) => {
    capLines.push(msg);
    if (capLines.length > 4) capLines = capLines.slice(-4);
    capBar.style.display = "flex";
    capStatus.textContent = capLines.join("  ·  ");
  };
  const unlistenCap = listen<string>("capture://status", (e) => pushCap(e.payload));

  const openCapture = async (inject: boolean) => {
    const url = (root.querySelector("#manual-url") as HTMLInputElement).value.trim();
    const popup = (root.querySelector("#chk-popup") as HTMLInputElement).checked;
    if (!url) {
      capBar.style.display = "flex";
      capStatus.textContent = "请先粘贴视频页地址, 再点「捕获」。";
      return;
    }
    capLines = [];
    const mode = popup ? (inject ? "正在打开捕获窗口…" : "正在打开原始窗口(无注入)…") : "正在后台启动捕获(不弹窗)…";
    pushCap(mode);
    try {
      const msg = await callCommand<string>("open_capture", { url, inject, silent: !popup });
      pushCap(String(msg));
    } catch (e) {
      pushCap("打开失败: " + String(e));
    }
  };
  root.querySelector("#btn-capture")!.addEventListener("click", () => openCapture(true));
  root.querySelector("#btn-cap-raw")!.addEventListener("click", () => openCapture(false));
  root.querySelector("#btn-cap-close")!.addEventListener("click", async () => {
    try {
      await callCommand("close_capture");
      pushCap("捕获窗口已关闭。");
    } catch (e) {
      pushCap("关闭失败: " + String(e));
    }
  });

  
  const syncSelAll = () => {
    const boxes = listEl.querySelectorAll<HTMLInputElement>("input.msel");
    const all = boxes.length > 0 && Array.from(boxes).every((b) => b.checked);
    (root.querySelector("#m-selall") as HTMLInputElement).checked = all;
  };
  root.querySelector("#m-selall")!.addEventListener("change", (e) => {
    const checked = (e.target as HTMLInputElement).checked;
    listEl.querySelectorAll<HTMLInputElement>("input.msel").forEach((b) => {
      b.checked = checked;
      const id = b.dataset.mid!;
      if (checked) selected.add(id);
      else selected.delete(id);
    });
  });
  
  const mergeHint = root.querySelector("#merge-hint") as HTMLElement;
  root.querySelector("#m-merge")!.addEventListener("click", async () => {
    const picked = currentMedia.filter((m) => selected.has(m.id));
    if (picked.length !== 2) {
      alert(`合并需要勾选恰好 2 项(1 条视频 + 1 条音频), 当前选中 ${picked.length} 项。`);
      return;
    }
    const [a, b] = picked;
    
    if ((a.kind === "Audio") === (b.kind === "Audio")) {
      alert(
        a.kind === "Audio"
          ? "两条都是音频, 没有画面可放 —— 请改选 1 条视频 + 1 条音频。"
          : "两条都是视频, 合并后依然没有声音 —— 请改选 1 条视频 + 1 条音频。",
      );
      return;
    }
    try {
      await callCommand("download_pair", {
        a: a.url,
        b: b.url,
        opts: { format: "best", quality: "best", out_dir: "" },
      });
      selected.clear();
    } catch (e) {
      alert(String(e));
    }
  });

  root.querySelector("#m-del-sel")!.addEventListener("click", async () => {
    const ids = Array.from(selected);
    if (ids.length === 0) {
      alert("请先勾选要删除的媒体项。");
      return;
    }
    try {
      await callCommand("remove_media", { ids });
      selected.clear();
      refresh();
    } catch (e) {
      alert(String(e));
    }
  });
  root.querySelector("#m-clear")!.addEventListener("click", async () => {
    if (!confirm("清空全部嗅探到的媒体？(只是列表, 不影响已下载文件, 可重新捕获)")) return;
    try {
      await callCommand("clear_media");
      selected.clear();
      refresh();
    } catch (e) {
      alert(String(e));
    }
  });

  
  async function refreshProxy() {
    try {
      const s = await callCommand<{ running: boolean; addr: string }>("proxy_status");
      pill.innerHTML = s.running
        ? `<span class="led"></span> 代理 已开启 · ${s.addr}`
        : `<span class="led off"></span> 代理 已关闭`;
      sw.classList.toggle("off", !s.running);
    } catch {
      
    }
  }
  refreshProxy();

  sw.addEventListener("click", async () => {
    try {
      const s = await callCommand<{ running: boolean }>("proxy_status");
      if (s.running) await callCommand("stop_proxy");
      else await callCommand("start_proxy");
      refreshProxy();
    } catch {

    }
  });

  
  return (media: MediaItem[], tasks: DownloadTask[]) => {
    currentMedia = media;
    
    const hasVideo = media.some((m) => m.kind !== "Audio");
    const hasAudio = media.some((m) => m.kind === "Audio");
    mergeHint.style.display = hasVideo && hasAudio ? "flex" : "none";
    countEl.textContent = media.length ? `(${media.length})` : "";
    listEl.innerHTML = media.length
      ? media.map((m) => cardHtml(m, latestTask(m.url, tasks), selected.has(m.id))).join("")
      : '<div class="empty">暂无。点「捕获」用内置浏览器播放网页视频即可自动抓直链。</div>';
    
    listEl.querySelectorAll<HTMLInputElement>("input.msel").forEach((b) => {
      b.checked = selected.has(b.dataset.mid!);
      b.addEventListener("change", () => {
        if (b.checked) selected.add(b.dataset.mid!);
        else selected.delete(b.dataset.mid!);
        syncSelAll();
      });
    });
    
    const live = new Set(media.map((m) => m.id));
    Array.from(selected).forEach((id) => {
      if (!live.has(id)) selected.delete(id);
    });
    syncSelAll();
    listEl.querySelectorAll<HTMLButtonElement>("[data-dl]").forEach((b) =>
      b.addEventListener("click", () =>
        callCommand("download", { url: b.dataset.dl, opts: { format: "best", quality: "best", out_dir: "" } }),
      ),
    );
    listEl.querySelectorAll<HTMLButtonElement>("[data-copy]").forEach((b) =>
      b.addEventListener("click", () => navigator.clipboard.writeText(b.dataset.copy || "")),
    );
    listEl.querySelectorAll<HTMLButtonElement>("[data-cancel]").forEach((b) =>
      b.addEventListener("click", () => callCommand("cancel_task", { id: b.dataset.cancel })),
    );
    listEl.querySelectorAll<HTMLButtonElement>("[data-play]").forEach((b) =>
      b.addEventListener("click", async () => {
        try {
          await callCommand("open_path", { path: b.dataset.play });
        } catch (e) {
          alert(String(e));
        }
      }),
    );
    listEl.querySelectorAll<HTMLButtonElement>("[data-reveal]").forEach((b) =>
      b.addEventListener("click", async () => {
        try {
          await callCommand("reveal_path", { path: b.dataset.reveal });
        } catch (e) {
          alert(String(e));
        }
      }),
    );
  };
}


function latestTask(url: string, tasks: DownloadTask[]): DownloadTask | null {
  let best: DownloadTask | null = null;
  let bestTs = -1;
  for (const t of tasks) {
    
    if (t.url !== url && t.pair_url !== url) continue;
    const ts = Number(t.id.replace("task_", "")) || 0;
    if (ts >= bestTs) {
      best = t;
      bestTs = ts;
    }
  }
  return best;
}

function cardHtml(m: MediaItem, task: DownloadTask | null, checked: boolean): string {
  const badge = KIND_LABEL[m.kind] || "video";
  const cls =
    m.kind === "M3u8"
      ? "badge m3u8"
      : m.kind === "Ts"
        ? "badge ts"
        : m.kind === "Audio"
          ? "badge audio"
          : "badge";
  
  const quality = m.quality && m.quality !== "未知" ? m.quality : "";
  const size = fmtSize(m.size_bytes) || "大小未知";
  const sub = `<span class="${cls}">${badge}</span> ${[quality, size, `来自 ${m.source}`]
    .map(escapeHtml)
    .join(" · ")}`;
  const sel = `<label class="msel"><input type="checkbox" class="msel" data-mid="${escapeAttr(m.id)}" ${checked ? "checked" : ""} /></label>`;

  
  if (!task) {
    return `<div class="card">
      ${sel}
      <div class="thumb">▶</div>
      <div class="meta"><div class="name">${escapeHtml(m.title)}</div><div class="sub">${sub}</div></div>
      <div class="acts">
        <button class="btn primary" data-dl="${escapeAttr(m.url)}">下载</button>
        <button class="btn ghost" data-copy="${escapeAttr(m.url)}">复制</button>
      </div>
    </div>`;
  }

  const pct = Math.round(task.progress * 100);
  const copyBtn = `<button class="btn ghost" data-copy="${escapeAttr(m.url)}">复制</button>`;

  
  if (task.status === "Queued" || task.status === "Downloading" || task.status === "Merging") {
    const label =
      task.status === "Queued" ? "排队中" : task.status === "Merging" ? "合并中" : `下载中 ${pct}%`;
    return `<div class="card">
      ${sel}
      <div class="thumb">▶</div>
      <div class="meta">
        <div class="name">${escapeHtml(m.title)}</div><div class="sub">${sub}</div>
        <div class="minibar"><i style="width:${task.status === "Queued" ? 0 : pct}%"></i></div>
      </div>
      <div class="acts">
        <span class="dlstate">${label}</span>
        <button class="btn ghost" data-cancel="${task.id}">取消</button>
        ${copyBtn}
      </div>
    </div>`;
  }

  
  if (task.status === "Done") {
    const fp = task.file_path || "";
    const revealTarget = fp || task.out_path;
    return `<div class="card">
      ${sel}
      <div class="thumb done">✓</div>
      <div class="meta"><div class="name">${escapeHtml(task.title || m.title)}</div><div class="sub">${sub}</div></div>
      <div class="acts">
        <span class="dlstate ok">已完成</span>
        ${fp ? `<button class="btn primary" data-play="${escapeAttr(fp)}">播放</button>` : ""}
        <button class="btn ghost" data-reveal="${escapeAttr(revealTarget)}">文件夹</button>
        ${copyBtn}
      </div>
    </div>`;
  }

  
  const errLine = task.error
    ? `<div class="errline" title="${escapeAttr(task.error)}">${escapeHtml(task.error)}</div>`
    : "";
  const stText = task.status === "Failed" ? "下载失败" : "已取消";
  return `<div class="card haserr">
    ${sel}
    <div class="thumb">▶</div>
    <div class="meta"><div class="name">${escapeHtml(m.title)}</div><div class="sub">${sub}</div>${errLine}</div>
    <div class="acts">
      <span class="dlstate err">${stText}</span>
      <button class="btn primary" data-dl="${escapeAttr(m.url)}">重新下载</button>
      ${copyBtn}
    </div>
  </div>`;
}

function escapeHtml(s: string): string {
  return s.replace(/[&<>]/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;" })[c] || c);
}
function escapeAttr(s: string): string {
  return s.replace(/"/g, "&quot;");
}
