
import { callCommand } from "../utils/ipc";
import type { DownloadTask, TaskStatus } from "../types";
import { t } from "../i18n";

function stLabel(s: TaskStatus): string {
  switch (s) {
    case "Queued":
      return t("dl.queued");
    case "Downloading":
      return t("dl.statusDownloading");
    case "Merging":
      return t("dl.merging");
    case "Done":
      return t("dl.done");
    case "Failed":
      return t("dl.failed");
    case "Paused":
      return t("dl.paused");
    case "Cancelled":
      return t("dl.cancelled");
  }
}

export function mountDownloads(root: HTMLElement, refresh: () => void): (t: DownloadTask[]) => void {
  const selected = new Set<string>();
  let latest: DownloadTask[] = [];
  let listEl: HTMLElement;

  function shell() {
    root.innerHTML = `
    <div class="list-head">
      <h3 class="sec">${t("dl.title")}</h3>
      <div class="batchbar">
        <label class="chk"><input type="checkbox" id="t-selall" /> ${t("sniffer.selAll")}</label>
        <button class="btn ghost small" id="t-del-sel">${t("sniffer.delSel")}</button>
        <button class="btn ghost small" id="t-del-done">${t("dl.clearDone")}</button>
      </div>
    </div>
    <div id="task-list"></div>`;
    listEl = root.querySelector("#task-list") as HTMLElement;

    const syncSelAll = () => {
      const boxes = listEl.querySelectorAll<HTMLInputElement>("input.tsel");
      const all = boxes.length > 0 && Array.from(boxes).every((b) => b.checked);
      (root.querySelector("#t-selall") as HTMLInputElement).checked = all;
    };

    root.querySelector("#t-selall")!.addEventListener("change", (e) => {
      const checked = (e.target as HTMLInputElement).checked;
      listEl.querySelectorAll<HTMLInputElement>("input.tsel").forEach((b) => {
        b.checked = checked;
        const id = b.dataset.tid!;
        if (checked) selected.add(id);
        else selected.delete(id);
      });
    });

    root.querySelector("#t-del-sel")!.addEventListener("click", async () => {
      const ids = Array.from(selected);
      if (ids.length === 0) {
        alert(t("dl.delSelEmpty"));
        return;
      }
      try {
        await callCommand("delete_tasks", { ids });
        selected.clear();
        refresh();
      } catch (e) {
        alert(String(e));
      }
    });

    root.querySelector("#t-del-done")!.addEventListener("click", async () => {
      const ids = latest.filter((x) => x.status === "Done").map((x) => x.id);
      if (ids.length === 0) {
        alert(t("dl.noDone"));
        return;
      }
      try {
        await callCommand("delete_tasks", { ids });
        ids.forEach((id) => selected.delete(id));
        refresh();
      } catch (e) {
        alert(String(e));
      }
    });

    listEl.querySelectorAll<HTMLInputElement>("input.tsel").forEach((b) => {
      b.checked = selected.has(b.dataset.tid!);
      b.addEventListener("change", () => {
        if (b.checked) selected.add(b.dataset.tid!);
        else selected.delete(b.dataset.tid!);
        syncSelAll();
      });
    });
    syncSelAll();
    renderList(syncSelAll);
  }

  function renderList(syncSelAll: () => void) {
    const sorted = [...latest].sort((a, b) =>
      a.status === b.status ? 0 : a.status === "Downloading" || a.status === "Merging" ? -1 : 1,
    );
    listEl.innerHTML = sorted.length
      ? sorted.map((x) => taskHtml(x, selected.has(x.id))).join("")
      : `<div class="empty">${t("dl.empty")}</div>`;

    const live = new Set(latest.map((x) => x.id));
    Array.from(selected).forEach((id) => {
      if (!live.has(id)) selected.delete(id);
    });
    listEl.querySelectorAll<HTMLInputElement>("input.tsel").forEach((b) => {
      b.checked = selected.has(b.dataset.tid!);
      b.addEventListener("change", () => {
        if (b.checked) selected.add(b.dataset.tid!);
        else selected.delete(b.dataset.tid!);
        syncSelAll();
      });
    });
    syncSelAll();
    bindListActions();
  }

  function bindListActions() {
    listEl.querySelectorAll<HTMLButtonElement>("[data-cancel]").forEach((b) =>
      b.addEventListener("click", () => callCommand("cancel_task", { id: b.dataset.cancel })),
    );
    listEl.querySelectorAll<HTMLButtonElement>("[data-retry]").forEach((b) =>
      b.addEventListener("click", () => callCommand("retry_task", { id: b.dataset.retry })),
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
    listEl.querySelectorAll<HTMLButtonElement>("[data-del]").forEach((b) =>
      b.addEventListener("click", async () => {
        try {
          await callCommand("delete_task", { id: b.dataset.del });
          selected.delete(b.dataset.del!);
          refresh();
        } catch (e) {
          alert(String(e));
        }
      }),
    );
  }

  window.addEventListener("i18n-change", shell);
  shell();

  return (tasks: DownloadTask[]) => {
    latest = tasks;
    renderList(() => {
      const boxes = listEl.querySelectorAll<HTMLInputElement>("input.tsel");
      const all = boxes.length > 0 && Array.from(boxes).every((b) => b.checked);
      (root.querySelector("#t-selall") as HTMLInputElement).checked = all;
    });
  };
}

function taskHtml(task: DownloadTask, checked: boolean): string {
  const pct = Math.round(task.progress * 100);
  const done = task.status === "Done";
  const failed = task.status === "Failed" || task.status === "Cancelled";
  const barCls = done ? "bar done" : "bar";
  const stCls = failed ? "st err" : done ? "st done" : "st";
  const speed = task.speed_bps > 0 ? (task.speed_bps / 1024).toFixed(0) + " KB/s" : "";
  const size =
    task.total > 0
      ? `${(task.downloaded / 1048576).toFixed(1)} / ${(task.total / 1048576).toFixed(1)} MB`
      : task.downloaded > 0
        ? `${(task.downloaded / 1048576).toFixed(1)} MB`
        : "";
  const sel = `<label class="tsel"><input type="checkbox" class="tsel" data-tid="${escapeHtml(task.id)}" ${checked ? "checked" : ""} /></label>`;

  let actions = "";
  if (done) {
    const fp = task.file_path || "";
    const revealTarget = fp || task.out_path;
    actions =
      (fp ? `<button class="btn ghost" data-play="${escapeHtml(fp)}">${t("dl.play")}</button>` : "") +
      `<button class="btn ghost" data-reveal="${escapeHtml(revealTarget)}">${t("dl.folder")}</button>`;
  } else if (failed) {
    actions = `<button class="btn ghost" data-retry="${task.id}">${t("dl.retry")}</button>`;
  } else {
    actions = `<button class="btn ghost" data-cancel="${task.id}">${t("dl.cancel")}</button>`;
  }
  actions += `<button class="btn ghost" data-del="${task.id}" title="${t("dl.deleteTitle")}">${t("dl.delete")}</button>`;
  const err = task.error ? `<div class="info" style="color:#e5484d">${escapeHtml(task.error)}</div>` : "";
  return `<div class="dl">
    ${sel}
    <div class="row"><div class="name">${escapeHtml(task.title)}</div><div class="${stCls}">${stLabel(task.status)} ${speed}</div></div>
    <div class="${barCls}"><i style="width:${pct}%"></i></div>
    <div class="info"><span>${pct}% · ${size}</span><span class="acts-inline">${actions}</span></div>
    ${err}
  </div>`;
}

function escapeHtml(s: string): string {
  return s.replace(/[&<>"]/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" })[c] || c);
}
