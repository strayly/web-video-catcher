
import { listen } from "@tauri-apps/api/event";
import { callCommand } from "./utils/ipc";
import type { MediaItem, DownloadTask } from "./types";
import { mountSniffer } from "./components/sniffer";
import { mountDownloads } from "./components/downloads";
import { mountSettings } from "./components/settings";

const app = document.getElementById("app")!;
app.innerHTML = `
  <div class="win">
    <div class="body">
      <div class="side">
        <div class="brand">📥 网页视频捕手</div>
        <div class="nav">
          <button class="active" data-v="sniffer"><span class="ic">🔎</span> 嗅探</button>
          <button data-v="downloads"><span class="ic">📋</span> 下载管理</button>
          <button data-v="settings"><span class="ic">⚙️</span> 设置</button>
        </div>
      </div>
      <div class="main">
        <section class="view active" id="view-sniffer"></section>
        <section class="view" id="view-downloads"></section>
        <section class="view" id="view-settings"></section>
      </div>
    </div>
  </div>
`;

const snifferRoot = document.getElementById("view-sniffer")!;
const downloadsRoot = document.getElementById("view-downloads")!;
const settingsRoot = document.getElementById("view-settings")!;

async function refreshMedia() {
  try {
    const m = await callCommand<MediaItem[]>("list_media");
    state.media = m;
    if (activeView() === "sniffer") updateSniffer(state.media, taskList());
  } catch {}
}
async function refreshTasks() {
  try {
    const t = await callCommand<DownloadTask[]>("list_tasks");
    state.tasks.clear();
    t.forEach((x) => state.tasks.set(x.id, x));
    if (activeView() === "downloads") updateDownloads(taskList());
    else if (activeView() === "sniffer") updateSniffer(state.media, taskList());
  } catch {}
}

const updateSniffer = mountSniffer(snifferRoot, refreshMedia);
const updateDownloads = mountDownloads(downloadsRoot, refreshTasks);
const updateSettings = mountSettings(settingsRoot);

const state = { media: [] as MediaItem[], tasks: new Map<string, DownloadTask>() };

function activeView(): string {
  return (document.querySelector(".nav button.active") as HTMLButtonElement).dataset.v!;
}

function taskList(): DownloadTask[] {
  return Array.from(state.tasks.values());
}

function renderActive() {
  const v = activeView();
  if (v === "sniffer") updateSniffer(state.media, taskList());
  else if (v === "downloads") updateDownloads(taskList());
  else updateSettings();
}

document.querySelectorAll<HTMLButtonElement>(".nav button").forEach((b) => {
  b.addEventListener("click", () => {
    document.querySelectorAll(".nav button").forEach((x) => x.classList.remove("active"));
    document.querySelectorAll(".view").forEach((x) => x.classList.remove("active"));
    b.classList.add("active");
    document.getElementById("view-" + b.dataset.v)!.classList.add("active");
    renderActive();
  });
});

listen<MediaItem>("sniffer://new", (e) => {
  if (!state.media.find((m) => m.url === e.payload.url)) {
    state.media.unshift(e.payload);
    if (activeView() === "sniffer") updateSniffer(state.media, taskList());
  }
});


listen<MediaItem>("sniffer://update", (e) => {
  const i = state.media.findIndex((m) => m.id === e.payload.id);
  if (i >= 0) state.media[i] = e.payload;
  else state.media.unshift(e.payload);
  if (activeView() === "sniffer") {
    const listEl = document.getElementById("media-list");
    const top = listEl ? listEl.scrollTop : 0;
    updateSniffer(state.media, taskList());
    if (listEl) listEl.scrollTop = top;
  }
});
listen<DownloadTask>("download://task", (e) => {
  state.tasks.set(e.payload.id, e.payload);
  
  if (activeView() === "downloads") updateDownloads(taskList());
  else if (activeView() === "sniffer") updateSniffer(state.media, taskList());
});

callCommand<MediaItem[]>("list_media").then((m) => {
  state.media = m;
  if (activeView() === "sniffer") updateSniffer(state.media, taskList());
});
callCommand<DownloadTask[]>("list_tasks").then((t) => {
  t.forEach((x) => state.tasks.set(x.id, x));
  if (activeView() === "downloads") updateDownloads(taskList());
  else if (activeView() === "sniffer") updateSniffer(state.media, taskList());
});
