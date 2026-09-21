
import { callCommand } from "../utils/ipc";
import type { EngineInfo } from "../types";

export function mountSettings(root: HTMLElement): () => void {
  root.innerHTML = `
    <h3 class="sec">基础</h3>
    <div class="form">
      <label>代理端口</label>
      <div class="ctl"><input type="text" value="8888" readonly /><span class="note">默认 8888</span></div>
      <label>下载目录</label>
      <div class="ctl"><input id="set-dir" type="text" placeholder="留空 = 程序目录下 downloads" /></div>
      <label>Cookie 浏览器</label>
      <div class="ctl">
        <select id="set-cookie">
          <option value="">不使用(部分站点需登录)</option>
          <option value="chrome">Chrome</option>
          <option value="edge">Edge</option>
          <option value="firefox">Firefox</option>
        </select>
        <span class="note">供 yt-dlp 解析页面用; 捕获到的直链会自动复用浏览器已带的 Cookie</span>
      </div>
      <div class="group">引擎</div>
      <label>yt-dlp / ffmpeg</label>
      <div class="ctl">
        <button class="btn ghost" id="btn-test">检测</button>
        <span id="engine-info" class="note">未检测</span>
      </div>
      <div class="note" style="margin-top:6px">
        捕获到的视频直链由程序内置下载器直连取流(带浏览器同款请求头), <b>不需要 yt-dlp</b>;
        yt-dlp 只用于解析网页地址、下载 m3u8 分片并合并。
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
    el.textContent = `yt-dlp: ${info.yt_dlp ? "OK" : "缺失"} (${info.yt_dlp_version})  ffmpeg: ${info.ffmpeg ? "OK" : "缺失"}`;
  });
  return () => {};
}
