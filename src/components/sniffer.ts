
import { listen } from "@tauri-apps/api/event";
import { callCommand } from "../utils/ipc";
import type { MediaItem, DownloadTask } from "../types";
import { t, pickBi, errText } from "../i18n";

function kindLabel(k: string): string {
  return t("kind." + k.toLowerCase()) || k.toLowerCase();
}

// Rust 侧的画质兜底标签按当前语言显示(未知画质则不显示)。
function qualLabel(q: string): string {
  if (!q || /^(未知|Unknown)$/i.test(q)) return "";
  if (q === "杜比全景声") return t("q.dolbyAtmos");
  if (q === "杜比视界") return t("q.dolbyVision");
  return q;
}

// Rust 侧的标题兜底值按当前语言显示。
function titleText(s: string): string {
  return /^(未知标题|Unknown title)$/.test(s) ? t("media.unknownTitle") : s;
}

function fmtSize(bytes: number): string {
  if (!bytes || bytes <= 0) return "";
  if (bytes >= 1073741824) return (bytes / 1073741824).toFixed(2) + " GB";
  if (bytes >= 1048576) return (bytes / 1048576).toFixed(1) + " MB";
  return Math.max(1, Math.round(bytes / 1024)) + " KB";
}

export function mountSniffer(root: HTMLElement, refresh: () => void): (m: MediaItem[], tasks: DownloadTask[]) => void {
  const selected = new Set<string>();
  let currentMedia: MediaItem[] = [];
  let capLines: string[] = [];
  let listEl: HTMLElement, countEl: HTMLElement, mergeHint: HTMLElement;

  const pushCap = (msg: string | { zh: string; en: string }) => {
    capLines.push(pickBi(msg));
    if (capLines.length > 4) capLines = capLines.slice(-4);
    (root.querySelector("#cap-bar") as HTMLElement).style.display = "flex";
    (root.querySelector("#cap-status") as HTMLElement).textContent = capLines.join("  ·  ");
  };

  // 解析事件(全局只注册一次): Rust 的 resolve_page 把页面 URL + 清晰度列表推回来
  interface RFormat { format_id: string; quality: string; ext: string; kind: string; size: number }
  listen<{ url: string; title: string; formats: RFormat[] }>("resolve://formats", (e) => {
    const p = e.payload;
    const panel = root.querySelector("#resolve-panel") as HTMLElement | null;
    if (!panel) return;
    (root.querySelector("#rp-title") as HTMLElement).textContent = p.title;
    const list = root.querySelector("#rp-list") as HTMLElement;
    list.innerHTML = "";
    for (const f of p.formats) {
      const row = document.createElement("div");
      row.className = "rp-row";
      const label =
        f.format_id === "best"
          ? t("sniffer.fmtBest")
          : `${f.quality} · ${f.ext} · ${f.kind === "Audio" ? t("kind.audio") : t("kind.other")}${f.size > 0 ? " · " + (f.size / 1048576).toFixed(1) + "MB" : ""}`;
      row.innerHTML = `<span class="rp-fmt">${escapeHtml(label)}</span><button class="btn primary small rp-dl">${t("sniffer.fmtDownload")}</button>`;
      row.querySelector(".rp-dl")!.addEventListener("click", async () => {
        const btn = row.querySelector<HTMLButtonElement>(".rp-dl")!;
        try {
          const sel =
            f.format_id === "best"
              ? "best"

              : f.kind === "Audio"
                ? f.format_id

                : `${f.format_id}+bestaudio`;
          await callCommand("download", { url: p.url, opts: { format: sel, quality: "best", out_dir: "" } });
          btn.textContent = "✓ " + t("sniffer.fmtAdded");
          btn.disabled = true;
        } catch (err) {
          btn.disabled = false;
          alert(errText(String(err)));
        }
      });
      list.appendChild(row);
    }
  });
  listen<{ url: string; error: string }>("resolve://error", (e) => {
    const panel = root.querySelector("#resolve-panel") as HTMLElement | null;
    if (!panel) return;
    (root.querySelector("#rp-title") as HTMLElement).textContent = t("sniffer.resolveFailed");
    (root.querySelector("#rp-list") as HTMLElement).innerHTML = `<div class="info" style="color:#e5484d">${escapeHtml(errText(e.payload.error))}</div>`;
  });

  // 打开解析面板并请求后端解析(yt-dlp)
  const doResolve = async (url: string) => {
    if (!url) {
      alert(t("sniffer.pleasePaste"));
      return;
    }
    const panel = root.querySelector("#resolve-panel") as HTMLElement | null;
    if (!panel) return;
    (root.querySelector("#rp-title") as HTMLElement).textContent = t("sniffer.resolving");
    (root.querySelector("#rp-list") as HTMLElement).innerHTML = "";
    panel.style.display = "flex";
    try {
      await callCommand("resolve_page", { url });
    } catch (e) {
      (root.querySelector("#rp-list") as HTMLElement).innerHTML = `<div class="info" style="color:#e5484d">${escapeHtml(errText(String(e)))}</div>`;
    }
  };

  function shell() {
    root.innerHTML = `
    <div class="statusbar">
      <span class="ad-link" id="ad-link" title="https://www.licstack.com/">🎯 ${t("sniffer.adText")}</span>
    </div>
    <div class="manual">
      <input id="manual-url" type="text" placeholder="${t("sniffer.urlPlaceholder")}" />
      <button class="btn primary cap" id="btn-capture">🧲 ${t("sniffer.btnCapture")}</button>
      <button class="btn ghost" id="btn-resolve">🔧 ${t("sniffer.btnResolve")}</button>
      <label class="chk" title="${t("sniffer.popupTitle")}">
        <input type="checkbox" id="chk-popup" /> ${t("sniffer.chkPopup")}
      </label>
    </div>
    <div class="resolve-panel" id="resolve-panel" style="display:none">
      <div class="rp-head"><span id="rp-title"></span><span class="rp-close" id="btn-resolve-close">✕</span></div>
      <div class="rp-list" id="rp-list"></div>
    </div>
    <div class="capbar" id="cap-bar" style="display:none">
      <span class="cap-text" id="cap-status">${t("sniffer.capOpening")}</span>
      <span class="cap-acts">
        <button class="btn ghost small" id="btn-cap-raw" title="${t("sniffer.capRawTitle")}">${t("sniffer.capRawBtn")}</button>
        <button class="btn ghost small" id="btn-cap-close">${t("sniffer.capClose")}</button>
      </span>
    </div>
    <div class="list-head">
      <h3 class="sec">${t("sniffer.mediaTitle")} <span class="mcount" id="m-count"></span></h3>
      <div class="batchbar">
        <label class="chk"><input type="checkbox" id="m-selall" /> ${t("sniffer.selAll")}</label>
        <button class="btn primary small" id="m-merge" title="${t("sniffer.mergeTitle")}">🔀 ${t("sniffer.merge")}</button>
        <button class="btn ghost small" id="m-del-sel">${t("sniffer.delSel")}</button>
        <button class="btn ghost small" id="m-clear">${t("sniffer.clearAll")}</button>
      </div>
    </div>
    <div class="mergehint" id="merge-hint" style="display:none">
      🎬 ${t("sniffer.mergeHint")}
    </div>
    <div id="media-list"></div>
  `;
    listEl = root.querySelector("#media-list") as HTMLElement;
    countEl = root.querySelector("#m-count") as HTMLElement;
    mergeHint = root.querySelector("#merge-hint") as HTMLElement;

    root.querySelector("#ad-link")!.addEventListener("click", async () => {
      try {
        await callCommand("open_url", { url: "https://www.licstack.com/" });
      } catch {
      }
    });

    const resolvePanel = root.querySelector("#resolve-panel") as HTMLElement;
    (root.querySelector("#btn-resolve-close") as HTMLElement).addEventListener("click", () => {
      resolvePanel.style.display = "none";
    });
    root.querySelector("#btn-resolve")!.addEventListener("click", async () => {
      const url = (root.querySelector("#manual-url") as HTMLInputElement).value.trim();
      await doResolve(url);
    });

    root.querySelector("#btn-capture")!.addEventListener("click", () => openCapture(true));
    root.querySelector("#btn-cap-raw")!.addEventListener("click", () => openCapture(false));
    root.querySelector("#btn-cap-close")!.addEventListener("click", async () => {
      try {
        await callCommand("close_capture");
        pushCap(t("sniffer.closeOk"));
      } catch (e) {
        pushCap(t("sniffer.closeFailed") + String(e));
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

    root.querySelector("#m-merge")!.addEventListener("click", async () => {
      const picked = currentMedia.filter((m) => selected.has(m.id));
      if (picked.length !== 2) {
        alert(t("sniffer.mergeNeed2", { n: picked.length }));
        return;
      }
      const [a, b] = picked;
      if ((a.kind === "Audio") === (b.kind === "Audio")) {
        alert(
          a.kind === "Audio"
            ? t("sniffer.mergeBothAudio")
            : t("sniffer.mergeBothVideo"),
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
        alert(t("sniffer.delSelEmpty"));
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
      if (!confirm(t("sniffer.clearConfirm"))) return;
      try {
        await callCommand("clear_media");
        selected.clear();
        refresh();
      } catch (e) {
        alert(String(e));
      }
    });

    listEl.querySelectorAll<HTMLInputElement>("input.msel").forEach((b) => {
      b.checked = selected.has(b.dataset.mid!);
      b.addEventListener("change", () => {
        if (b.checked) selected.add(b.dataset.mid!);
        else selected.delete(b.dataset.mid!);
        syncSelAll();
      });
    });
    bindListActions();
    syncSelAll();
  }

  async function openCapture(inject: boolean) {
    const url = (root.querySelector("#manual-url") as HTMLInputElement).value.trim();
    const popup = (root.querySelector("#chk-popup") as HTMLInputElement).checked;
    if (!url) {
      (root.querySelector("#cap-bar") as HTMLElement).style.display = "flex";
      (root.querySelector("#cap-status") as HTMLElement).textContent = t("sniffer.pleasePaste");
      return;
    }
    capLines = [];
    const mode = popup
      ? inject
        ? t("sniffer.capOpening")
        : t("sniffer.capOpeningRaw")
      : t("sniffer.capOpeningBg");
    pushCap(mode);
    try {
      const msg = await callCommand<string>("open_capture", { url, inject, silent: !popup });
      if (msg) pushCap(msg);
    } catch (e) {
      pushCap(t("sniffer.openFailed") + String(e));
    }
  }

  function bindListActions() {
    listEl.querySelectorAll<HTMLButtonElement>("[data-dl]").forEach((b) =>
      b.addEventListener("click", () => {
        const u = b.dataset.dl || "";
        // YouTube 的 googlevideo 直链带签名保护, 直接下载必然 403, 自动转走「解析」
        if (/googlevideo\.com|youtube\.com\/videoplayback|youtube\.com\/api\/manifest/i.test(u)) {
          const page = (root.querySelector("#manual-url") as HTMLInputElement)?.value.trim() || "";
          if (page) {
            void doResolve(page);
            return;
          }
        }
        void callCommand("download", { url: u, opts: { format: "best", quality: "best", out_dir: "" } });
      }),
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
  }

  const unlistenCap = listen<any>("capture://status", (e) => pushCap(e.payload));
  void unlistenCap;

  window.addEventListener("i18n-change", () => {
    capLines = [];
    shell();
  });

  shell();

  return (media: MediaItem[], tasks: DownloadTask[]) => {
    currentMedia = media;
    const hasVideo = media.some((m) => m.kind !== "Audio");
    const hasAudio = media.some((m) => m.kind === "Audio");
    mergeHint.style.display = hasVideo && hasAudio ? "flex" : "none";
    countEl.textContent = media.length ? `(${media.length})` : "";
    listEl.innerHTML = media.length
      ? media.map((m) => cardHtml(m, latestTask(m.url, tasks), selected.has(m.id))).join("")
      : `<div class="empty">${t("sniffer.empty")}</div>`;

    listEl.querySelectorAll<HTMLInputElement>("input.msel").forEach((b) => {
      b.checked = selected.has(b.dataset.mid!);
      b.addEventListener("change", () => {
        if (b.checked) selected.add(b.dataset.mid!);
        else selected.delete(b.dataset.mid!);
      });
    });

    const live = new Set(media.map((m) => m.id));
    Array.from(selected).forEach((id) => {
      if (!live.has(id)) selected.delete(id);
    });
    (root.querySelector("#m-selall") as HTMLInputElement).checked =
      listEl.querySelectorAll<HTMLInputElement>("input.msel").length > 0 &&
      Array.from(listEl.querySelectorAll<HTMLInputElement>("input.msel")).every((b) => b.checked);
    bindListActions();
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
  const badge = kindLabel(m.kind);
  const cls =
    m.kind === "M3u8"
      ? "badge m3u8"
      : m.kind === "Ts"
        ? "badge ts"
        : m.kind === "Audio"
          ? "badge audio"
          : "badge";
  const quality = qualLabel(m.quality || "");
  const size = fmtSize(m.size_bytes) || t("media.sizeUnknown");
  const sub = `<span class="${cls}">${badge}</span> ${[quality, size, t("media.from", { source: m.source })]
    .map(escapeHtml)
    .join(" · ")}`;
  const sel = `<label class="msel"><input type="checkbox" class="msel" data-mid="${escapeAttr(m.id)}" ${checked ? "checked" : ""} /></label>`;

  if (!task) {
    return `<div class="card">
      ${sel}
      <div class="thumb">▶</div>
      <div class="meta"><div class="name">${escapeHtml(titleText(m.title))}</div><div class="sub">${sub}</div></div>
      <div class="acts">
        <button class="btn primary" data-dl="${escapeAttr(m.url)}">${t("sniffer.download")}</button>
        <button class="btn ghost" data-copy="${escapeAttr(m.url)}">${t("sniffer.copy")}</button>
      </div>
    </div>`;
  }

  const pct = Math.round(task.progress * 100);
  const copyBtn = `<button class="btn ghost" data-copy="${escapeAttr(m.url)}">${t("sniffer.copy")}</button>`;

  if (task.status === "Queued" || task.status === "Downloading" || task.status === "Merging") {
    const label =
      task.status === "Queued"
        ? t("dl.queued")
        : task.status === "Merging"
          ? t("dl.merging")
          : t("dl.downloading", { pct });
    return `<div class="card">
      ${sel}
      <div class="thumb">▶</div>
      <div class="meta">
        <div class="name">${escapeHtml(titleText(m.title))}</div><div class="sub">${sub}</div>
        <div class="minibar"><i style="width:${task.status === "Queued" ? 0 : pct}%"></i></div>
      </div>
      <div class="acts">
        <span class="dlstate">${label}</span>
        <button class="btn ghost" data-cancel="${task.id}">${t("dl.cancel")}</button>
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
      <div class="meta"><div class="name">${escapeHtml(titleText(task.title || m.title))}</div><div class="sub">${sub}</div></div>
      <div class="acts">
        <span class="dlstate ok">${t("dl.done")}</span>
        ${fp ? `<button class="btn primary" data-play="${escapeAttr(fp)}">${t("dl.play")}</button>` : ""}
        <button class="btn ghost" data-reveal="${escapeAttr(revealTarget)}">${t("dl.folder")}</button>
        ${copyBtn}
      </div>
    </div>`;
  }

  const errLine = task.error
    ? `<div class="errline" title="${escapeAttr(errText(task.error))}">${escapeHtml(errText(task.error))}</div>`
    : "";
  const stText = task.status === "Failed" ? t("dl.failed") : t("dl.cancelled");
  return `<div class="card haserr">
    ${sel}
    <div class="thumb">▶</div>
    <div class="meta"><div class="name">${escapeHtml(titleText(m.title))}</div><div class="sub">${sub}</div>${errLine}</div>
    <div class="acts">
      <span class="dlstate err">${stText}</span>
      <button class="btn primary" data-dl="${escapeAttr(m.url)}">${t("dl.retry")}</button>
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
