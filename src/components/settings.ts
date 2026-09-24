
import { callCommand } from "../utils/ipc";
import type { EngineInfo } from "../types";
import { t } from "../i18n";

export function mountSettings(root: HTMLElement): () => void {
  function render() {
    root.innerHTML = `
    <h3 class="sec">${t("set.basic")}</h3>
    <div class="form">
      <label>${t("set.proxyPort")}</label>
      <div class="ctl"><input type="text" value="8888" readonly /><span class="note">${t("set.proxyPortNote")}</span></div>
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
        <span id="engine-info" class="note">${t("set.engineNotTested")}</span>
      </div>
      <div class="note" style="margin-top:6px">
        ${t("set.noteEngine")}
      </div>
    </div>
  `;
    root.querySelector("#set-cookie")!.addEventListener("change", async (e) => {
      const v = (e.target as HTMLSelectElement).value;
      await callCommand("set_cookie_browser", { browser: v || null });
    });
    root.querySelector("#btn-test")!.addEventListener("click", async () => {
      const info = await callCommand<EngineInfo>("test_engine");
      const el = root.querySelector("#engine-info") as HTMLElement;
      el.textContent = t("set.engineInfo", {
        ok: info.yt_dlp ? t("common.ok") : t("common.missing"),
        ver: info.yt_dlp_version,
        ok2: info.ffmpeg ? t("common.ok") : t("common.missing"),
      });
    });
  }
  window.addEventListener("i18n-change", render);
  render();
  return () => {};
}
