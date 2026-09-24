
import { callCommand } from "../utils/ipc";
import type { EngineInfo } from "../types";
import { t, getLang, setLang } from "../i18n";

export function mountSettings(root: HTMLElement): () => void {
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
    <div class="group">${t("set.uiGroup")}</div>
    <div class="form">
      <label>${t("set.language")}</label>
      <div class="ctl">
        <select id="set-lang">
          <option value="zh">${t("set.langZh")}</option>
          <option value="en">${t("set.langEn")}</option>
        </select>
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
  const langSel = root.querySelector("#set-lang") as HTMLSelectElement;
  langSel.value = getLang();
  langSel.addEventListener("change", (e) => {
    setLang((e.target as HTMLSelectElement).value as "zh" | "en");
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
      ver: info.yt_dlp_version,
      ok2: info.ffmpeg ? t("common.ok") : t("common.missing"),
    });
  });
  return () => {};
}
