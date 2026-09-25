
import { callCommand } from "../utils/ipc";
import type { EngineInfo } from "../types";
import { t } from "../i18n";

export function mountSettings(root: HTMLElement): () => void {
  async function render() {
    root.innerHTML = `
    <h3 class="sec">${t("set.basic")}</h3>
    <div class="form">
      <label>${t("set.downloadDir")}</label>
      <div class="ctl"><input id="set-dir" type="text" placeholder="${t("set.downloadDirPlaceholder")}" /></div>
      <label>${t("set.cookieBrowser")}</label>
      <div class="ctl">
        <select id="set-cookie">
          <option value="">${t("set.cookieNone")}</option>
          <option value="chrome">Chrome</option>
          <option value="edge">Edge</option>
          <option value="firefox">Firefox</option>
        </select>
        <span class="note">${t("set.cookieNote")}</span>
      </div>
    </div>
    <div class="group">${t("set.engineGroup")}</div>
    <div class="form">
      <label>${t("set.ytDlpFfmpeg")}</label>
      <div class="ctl">
        <button class="btn ghost" id="btn-test">${t("set.testBtn")}</button>
        <button class="btn ghost" id="btn-upd-ytdlp">${t("set.updateYtDlp")}</button>
        <span id="engine-info" class="note">${t("set.engineNotTested")}</span>
        <span id="ytdlp-info" class="note"></span>
      </div>
      <div class="note" style="margin-top:6px">
        ${t("set.noteEngine")}
      </div>
    </div>
    <div class="group">${t("set.aboutGroup")}</div>
    <div class="form">
      <label>${t("set.version")}</label>
      <div class="ctl">
        <button class="btn ghost" id="btn-update">${t("set.checkUpdate")}</button>
        <span id="update-info" class="note"></span>
      </div>
    </div>
  `;
    const infoEl = root.querySelector("#update-info") as HTMLElement;
    infoEl.textContent = `v${await callCommand<{ current: string }>("check_update").then((u) => u.current).catch(() => "?")}`;
    root.querySelector("#btn-update")!.addEventListener("click", async () => {
      const el = root.querySelector("#update-info") as HTMLElement;
      el.textContent = t("set.checking");
      try {
        const u = await callCommand<{ latest: string; has_update: boolean; url: string; current: string }>("check_update");
        if (u.has_update) {
          el.innerHTML = "";
          const a = document.createElement("a");
          a.href = "#";
          a.textContent = t("update.found", { ver: u.latest.replace(/^v/, "") });
          a.style.color = "var(--green, #30a46c)";
          a.addEventListener("click", async (ev) => {
            ev.preventDefault();
            try {
              await callCommand("open_url", { url: u.url });
            } catch {
            }
          });
          el.appendChild(a);
        } else {
          el.textContent = t("update.latest");
        }
      } catch {
        el.textContent = t("update.checkFailed");
      }
    });
    root.querySelector("#set-cookie")!.addEventListener("change", async (e) => {
      const v = (e.target as HTMLSelectElement).value;
      await callCommand("set_cookie_browser", { browser: v || null });
    });
    root.querySelector("#btn-test")!.addEventListener("click", async () => {
      const info = await callCommand<EngineInfo>("test_engine");
      const el = root.querySelector("#engine-info") as HTMLElement;
      el.textContent = t("set.engineInfo", {
        ok: info.yt_dlp ? t("common.ok") : t("common.missing"),
        ver: info.yt_dlp ? info.yt_dlp_version : t("set.notInstalled"),
        ok2: info.ffmpeg ? t("common.ok") : t("common.missing"),
      });
    });
    root.querySelector("#btn-upd-ytdlp")!.addEventListener("click", async () => {
      const el = root.querySelector("#ytdlp-info") as HTMLElement;
      el.textContent = t("set.checking");
      try {
        const msg = await callCommand<string>("update_ytdlp");
        el.textContent = errText(msg);
      } catch (e) {
        el.textContent = errText(String(e));
      }
    });
  }
  window.addEventListener("i18n-change", render);
  render();
  return () => {};
}
