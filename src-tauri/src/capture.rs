





use crate::schema::*;
use crate::sniffer::{classify, host_of};
use crate::state::AppState;
use tauri::{AppHandle, Emitter, Listener, Manager, WebviewUrl};
use tauri::webview::WebviewWindowBuilder;



const INJECT: &str = r#"(function(){
  // 回传被拒时(典型场景: 站点不在 capabilities/*.json 的 remote.urls 里, ACL 会拒掉 emit)
  // 用这个前缀把原因写进 document.title, Rust 端识别后转成状态条提示, 不当作页面标题
  // —— 否则会污染下载文件名。
  var WARN_PREFIX = '\u26D4[capture] ';
  var seen = {};
  function warnOnce(msg) {
    if (window.__capWarned) return;
    window.__capWarned = true;
    try { console.warn('[capture] 媒体直链回传失败:', msg); } catch(e){}
    // 页面脚本随后会把自己标题改回来, 所以多刷几次, 保证用户看得见
    var n = 0;
    var tick = setInterval(function(){
      try { document.title = WARN_PREFIX + msg; } catch(e){}
      if (++n >= 10) clearInterval(tick);
    }, 1200);
  }
  function report(url, ct, method) {
    if (!url || url.startsWith('data:') || url.startsWith('blob:')) return;
    if (seen[url]) return;
    // 明确非媒体的响应类型先挡一道, 少发无用的 IPC(Rust 端还会复核)
    if (ct && badRe.test(ct)) return;
    seen[url] = 1;
    window.__capAny = 1;
    try {
      var ev = window.__TAURI__ && window.__TAURI__.event;
      if (!ev || !ev.emit) { warnOnce('页面里没有 Tauri 注入对象'); return; }
      // emit 是异步的: ACL 拒绝只体现在 Promise 上, 同步 try/catch 根本抓不到 ——
      // 之前所有"捕获失败"都是一个字都不报的, 原因就在这里。
      var p = ev.emit('capture://media', { url: url, content_type: ct || '', method: (method || 'GET').toUpperCase(), host: location.host });
      if (p && p.catch) p.catch(function(e){ warnOnce(String((e && e.message) || e)); });
    } catch(e){ warnOnce(String((e && e.message) || e)); }
  }
  var mediaRe = /video\/|audio\/|application\/(vnd\.apple\.mpegurl|x-mpegurl|dash\+xml)/i;
  var extRe = /\.(mp4|m4v|m4s|m3u8|mpd|ts|webm|mkv|flv|mp3|m4a|aac)(\?|#|$)/i;
  // 无扩展名的媒体直链路径特征: TikTok/抖音系取流地址形如 /video/tos/... , B 站是 /upgcxcode/...
  var pathRe = /\/(video\/tos|aweme\/v1\/play|upgcxcode)\//i;
  // 小红书取流地址: 域名 xhscdn.com、路径 /stream/...、content-type 常是 application/octet-stream 且无 .mp4 扩展,
  // 上面的 mediaRe/extRe/pathRe 全都不命中, 单独加一条(图片/JS 走的是 /platform/ /formula-static/, 不会误伤)。
  var xhsRe = /xhscdn\.com\/stream\//i;
  var badRe = /^(text|image|font)\/|application\/(json|javascript|xml|xhtml|pdf|zip|wasm)/i;
  try {
    var of = window.fetch;
    window.fetch = function(input, init){
      var url = typeof input === 'string' ? input : (input && input.url) || '';
      var method = (init && init.method) || (input && input.method) || 'GET';
      return of.apply(this, arguments).then(function(r){
        var ct = ''; try { ct = r.headers && r.headers.get ? r.headers.get('content-type') : ''; } catch(e){}
        if (url && (mediaRe.test(ct) || extRe.test(url) || pathRe.test(url) || xhsRe.test(url))) report(url, ct, method);
        return r;
      });
    };
  } catch(e){}
  try {
    var oo = XMLHttpRequest.prototype.open;
    XMLHttpRequest.prototype.open = function(m, url){ this.__capUrl = typeof url === 'string' ? url : ''; this.__capM = m; return oo.apply(this, arguments); };
    var os = XMLHttpRequest.prototype.send;
    XMLHttpRequest.prototype.send = function(){
      var self = this;
      this.addEventListener('load', function(){
        var ct = ''; try { ct = self.getResponseHeader ? self.getResponseHeader('content-type') : ''; } catch(e){}
        if (self.__capUrl && (mediaRe.test(ct) || extRe.test(self.__capUrl) || pathRe.test(self.__capUrl) || xhsRe.test(self.__capUrl))) report(self.__capUrl, ct, self.__capM);
      });
      return os.apply(this, arguments);
    };
  } catch(e){}
  // SPA 普遍用 JS 直接给 video.src 赋值(这是 DOM 属性, 不会出现在 HTML 标签上),
  // MutationObserver 只能看到 attribute 变化, 抓不到这种 —— 必须劫持原型上的 setter。
  // 快手/小红书等动态渲染的播放页多半走这条路, 漏掉它就会一条都抓不到。
  try {
    var proto = window.HTMLMediaElement && window.HTMLMediaElement.prototype;
    var desc = proto && Object.getOwnPropertyDescriptor(proto, 'src');
    if (desc && desc.set) {
      Object.defineProperty(proto, 'src', {
        configurable: true,
        enumerable: desc.enumerable,
        get: desc.get,
        set: function(v){ try { report(String(v || ''), ''); } catch(e){} return desc.set.call(this, v); }
      });
    }
  } catch(e){}
  try {
    function scan(el) {
      if (el && el.tagName === 'VIDEO' && el.src) report(el.src, '');
      if (el && el.querySelectorAll) {
        el.querySelectorAll('video[src], source[src]').forEach(function(v){ report(v.src || '', ''); });
      }
    }
    var mo = new MutationObserver(function(muts){
      muts.forEach(function(m){ m.addedNodes.forEach(scan); });
    });
    mo.observe(document.documentElement, { childList: true, subtree: true });
    window.addEventListener('load', function(){ scan(document.body); });
  } catch(e){}
  // 兜底: 播放器在 Worker/ServiceWorker 里拉分片时, 那些全局各自有自己的 fetch,
  // window 上的 hook 覆盖不到(独立 worker 全局); 但 performance resource timing 能列出
  // 页面上已经发出的全部请求 —— 按媒体特征过一遍, 把漏掉的捡回来。
  try {
    function scanPerf() {
      var list = performance.getEntriesByType ? performance.getEntriesByType('resource') : [];
      for (var i = 0; i < list.length; i++) {
        var u = (list[i] && list[i].name) || '';
        if (u && (extRe.test(u) || pathRe.test(u) || xhsRe.test(u))) report(u, '');
      }
    }
    setInterval(scanPerf, 1500);
    window.addEventListener('load', function(){ setTimeout(scanPerf, 800); });
  } catch(e){}

  // 自动触发播放: TikTok/B站等动态站点, 隐藏窗口会被节流、播放器不主动发取流请求。
  // 周期尝试把 video 静音后 play(), 并在加载后点一次常见播放按钮, 让播放器真正开始缓冲 —— 直链才会出现。
  // 对普通站点无副作用(没有 video 就不做事)。
  try {
    var capPlayClicked = false;
    function tryPlay() {
      var vs = document.querySelectorAll('video');
      for (var i = 0; i < vs.length; i++) {
        var v = vs[i];
        try {
          if (v.muted === false) v.muted = true;
          var pr = v.play && v.play();
          if (pr && pr.catch) pr.catch(function(){});
        } catch(e){}
      }
      if (!capPlayClicked) {
        capPlayClicked = true;
        var sels = ['[class*="play" i]', '[aria-label*="play" i]', '[data-e2e*="play"]'];
        for (var s = 0; s < sels.length; s++) {
          try {
            var btns = document.querySelectorAll(sels[s]);
            for (var b = 0; b < btns.length; b++) {
              if (btns[b].offsetParent !== null) { try { btns[b].click(); } catch(e){} }
            }
          } catch(e){}
        }
      }
    }
    setInterval(tryPlay, 2000);
    window.addEventListener('load', function(){ tryPlay(); setTimeout(tryPlay, 1500); });
  } catch(e){}

  // 12 秒仍未出现直链时给一次提示(仅一次): 多半是隐藏窗口被节流, 需勾选「弹窗」并手动播放
  try {
    setTimeout(function(){
      if (!window.__capAny) {
        try {
          var ev = window.__TAURI__ && window.__TAURI__.event;
          if (ev && ev.emit) {
            var tip = '⏳ 12 秒内未检测到媒体直链: 请勾选「弹窗」让捕获窗口可见, 并在窗口内点击播放(TikTok 等站点需可见窗口才发取流请求)';
            if (location.host.indexOf('xiaohongshu.com') >= 0) {
              tip = '⏳ 小红书视频需登录后才能播放: 请勾选「弹窗」, 在弹出的窗口里登录小红书, 登录成功后再点「捕获」重试';
            }
            ev.emit('capture://status', tip);
          }
        } catch(e){}
      }
    }, 12000);
  } catch(e){}
})();"#;



const WARN_PREFIX: &str = "\u{26D4}[capture] ";


static ACL_WARNED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);



pub const DESKTOP_UA: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/126.0.0.0 Safari/537.36";



static PROBE_BUDGET: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
const PROBE_BUDGET_MAX: usize = 60;


fn claim_probe_budget() -> bool {
    PROBE_BUDGET.fetch_add(1, std::sync::atomic::Ordering::Relaxed) < PROBE_BUDGET_MAX
}





pub fn open(app: &AppHandle, state: &AppState, url: String, inject: bool, visible: bool) -> Result<String, String> {
    let parsed: tauri::Url = url
        .parse()
        .map_err(|_| format!("URL 无效: {url}（需要 http(s):// 开头的完整地址）"))?;
    
    state.set_capture_page(url.clone());
    
    PROBE_BUDGET.store(0, std::sync::atomic::Ordering::Relaxed);
    ACL_WARNED.store(false, std::sync::atomic::Ordering::Relaxed);
    
    if let Some(w) = app.get_webview_window("capture") {
        let _ = w.close();
        std::thread::sleep(std::time::Duration::from_millis(300));
    }

    
    let make_builder = || {
        let nav_app = app.clone();
        let load_app = app.clone();
        let title_app = app.clone();
        let title_state = AppState::clone(state);
        let mut b = WebviewWindowBuilder::new(app, "capture", WebviewUrl::External(parsed.clone()))
            .title("捕获浏览器 - 网页视频捕手")
            .inner_size(1100.0, 720.0)
            .visible(visible)
            .devtools(visible)
            .user_agent(DESKTOP_UA)
            .on_navigation(move |u| {
                let msg = format!("导航: {u}");
                println!("[capture] {msg}");
                let _ = nav_app.emit("capture://status", msg);
                true
            })
            .on_page_load(move |_w, payload| {
                let stage = match payload.event() {
                    tauri::webview::PageLoadEvent::Started => "开始加载",
                    tauri::webview::PageLoadEvent::Finished => "加载完成",
                };
                let msg = format!("{stage}: {}", payload.url());
                println!("[capture] {msg}");
                let _ = load_app.emit("capture://status", msg);
            })
            .on_document_title_changed(move |_w, t| {
                
                
                if let Some(reason) = t.strip_prefix(WARN_PREFIX) {
                    if !ACL_WARNED.swap(true, std::sync::atomic::Ordering::Relaxed) {
                        let msg = format!(
                            "⚠️ 抓不到媒体: 回传通道被拒({reason}) —— 该站点未在 capabilities/capture.json 的 remote.urls 中授权"
                        );
                        println!("[capture] {msg}");
                        let _ = title_app.emit("capture://status", msg);
                    }
                    return;
                }
                let msg = format!("标题: {t}");
                println!("[capture] {msg}");
                
                title_state.set_capture_title(t.clone());
                let _ = title_app.emit("capture://status", msg);
            });
        
        
        
        
        if inject {
            b = b.initialization_script(INJECT);
        }
        b
    };

    let mut last_err = String::new();
    for attempt in 0..3 {
        match make_builder().build() {
            Ok(_) => {
                let hint = if !visible {
                    "后台捕获已启动(不弹窗)。页面加载后自动捕获媒体直链, 进度见上方状态条"
                } else if inject {
                    "捕获窗口已打开(含注入)。若白屏, 按 F12 看控制台, 或用「原始窗口」重试"
                } else {
                    "原始窗口已打开(无注入), 仅用于排查白屏原因"
                };
                return Ok(hint.into());
            }
            Err(e) => {
                last_err = e.to_string();
                println!("[capture] 第 {} 次创建失败: {last_err}", attempt + 1);
                std::thread::sleep(std::time::Duration::from_millis(400));
            }
        }
    }
    Err(format!("创建捕获窗口失败: {last_err}"))
}


pub fn register(app: &AppHandle, state: AppState) {
    let handle = app.clone();
    app.listen("capture://media", move |event| {
        let payload = match serde_json::from_str::<serde_json::Value>(event.payload()) {
            Ok(v) => v,
            Err(_) => return,
        };
        let url = payload
            .get("url")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        if url.trim().is_empty() {
            return;
        }
        let ct = payload
            .get("content_type")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        
        let method = payload
            .get("method")
            .and_then(|v| v.as_str())
            .unwrap_or("GET")
            .to_ascii_uppercase();
        if method != "GET" {
            return;
        }
        let (kind, quality) = match classify(&url, &ct) {
            Some(v) => v,
            
            None => return,
        };
        let host = host_of(&url);
        
        let page_title = state.capture_title();
        let title = if page_title.trim().is_empty() { host.clone() } else { page_title };
        let item = MediaItem {
            id: format!("m_{}", now_ms()),
            url: url.clone(),
            title,
            source: host,
            kind,
            quality,
            size_bytes: 0,
            status: "new".into(),
        };
        let referer = state.capture_page_url().or_else(|| state.capture_page_origin());

        // 信息流站点(TikTok/抖音/快手)会自动连播, 推荐视频的直链也会被抓进来;
        // 用户意图是"下载当前页面这条" —— 非音频媒体每次捕获最多保留 2 条,
        // 但媒体 URL 里带页面视频 ID(/video/<id>)的始终放行(那是正片本尊)。
        const FEED_HOSTS: &[&str] = &["tiktok.com", "douyin.com", "iesdouyin.com", "kuaishou.com"];
        let page_url = state.capture_page_url().unwrap_or_default();
        let is_feed = FEED_HOSTS.iter().any(|h| host_of(&page_url).contains(h));
        if is_feed && !matches!(item.kind, MediaKind::Audio) {
            let id_hit = page_url
                .split("/video/")
                .nth(1)
                .or_else(|| page_url.split("/note/").nth(1))
                .map(|rest| {
                    let id: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
                    !id.is_empty() && url.contains(&id)
                })
                .unwrap_or(false);
            if !id_hit && !state.feed_allow() {
                return;
            }
        }

        if state.push_media(item.clone()) {
            if let Some(r) = &referer {
                state.set_referer(&url, r.clone());
            }
            let _ = handle.emit("sniffer://new", item.clone());
            
            
            if !matches!(item.kind, MediaKind::M3u8 | MediaKind::Dash) && claim_probe_budget() {
                let st = state.clone();
                let h = handle.clone();
                let probe_url = url.clone();
                let mid = item.id.clone();
                tauri::async_runtime::spawn(async move {
                    let cookie = crate::download::webview_cookie_header(&h, &probe_url);
                    if let Some(size) = crate::download::probe_size(
                        &probe_url,
                        referer.as_deref(),
                        cookie.as_deref(),
                    )
                    .await
                    {
                        if let Some(updated) = st.set_media_size(&mid, size) {
                            let _ = h.emit("sniffer://update", updated);
                        }
                    }
                });
            }
        }
    });
}


fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}
