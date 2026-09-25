




use std::io::{Read, Write};
#[cfg(windows)]
use std::os::windows::process::CommandExt;
use std::process::Stdio;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager, Url};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command as TokioCommand;
use crate::capture::DESKTOP_UA;
use crate::schema::*;
use crate::sniffer::host_of;
use crate::state::AppState;

#[derive(serde::Serialize, Clone)]
struct ResolvedFormat {
    format_id: String,
    quality: String,
    ext: String,
    kind: String,
    size: i64,
}

#[derive(serde::Serialize)]
struct ResolvedPage {
    url: String,
    title: String,
    formats: Vec<ResolvedFormat>,
}


/// 生成双语提示文本: 序列化为 {"zh":..,"en":..} JSON 字符串, 由前端解析选显。
pub fn bi(zh: &str, en: &str) -> String {
    serde_json::json!({ "zh": zh, "en": en }).to_string()
}

/// 拆出双语字段: 若 s 是 {"zh":..,"en":..} JSON 则返回 (zh, en), 否则原样返回。
pub fn bi_part(s: &str) -> (String, String) {
    serde_json::from_str::<serde_json::Value>(s)
        .ok()
        .and_then(|v| {
            let zh = v["zh"].as_str()?.to_string();
            let en = v["en"].as_str()?.to_string();
            Some((zh, en))
        })
        .unwrap_or_else(|| (s.to_string(), s.to_string()))
}



pub fn enqueue(app: AppHandle, state: AppState, url: String, format: String, out_dir: String) -> String {
    
    for t in state.tasks().values() {
        if t.url == url
            && matches!(t.status, TaskStatus::Queued | TaskStatus::Downloading | TaskStatus::Merging)
        {
            return t.id.clone();
        }
    }
    for t in state.tasks().values() {
        if t.url == url
            && t.status == TaskStatus::Done
            && !t.file_path.is_empty()
            && std::path::Path::new(&t.file_path).exists()
        {
            emit_task(&app, &state, &t.id);
            return t.id.clone();
        }
    }
    let id = format!("task_{}", now_ms());
    let dir = if out_dir.trim().is_empty() {
        default_out_dir()
    } else {
        out_dir.trim().to_string()
    };
    let _ = std::fs::create_dir_all(&dir);
    
    let cap_title = state.capture_title();
    let title = if cap_title.trim().is_empty() { short_title(&url) } else { cap_title };
    let task = DownloadTask {
        id: id.clone(),
        url: url.clone(),
        title,
        out_path: dir.clone(),
        status: TaskStatus::Queued,
        progress: 0.0,
        speed_bps: 0.0,
        downloaded: 0,
        total: 0,
        error: "".into(),
        format: Some(format.clone()),
        file_path: "".into(),
        pair_url: "".into(),
    };
    state.upsert_task(task);
    emit_task(&app, &state, &id);
    state.mark_cancel(&id, false);
    let cookie = state.cookie_browser();

    let referer = state.referer_of(&url);

    let spawned_id = id.clone();
    tauri::async_runtime::spawn(async move {
        let (cookie_header, cookies_txt) = tauri::async_runtime::spawn_blocking({
            let app = app.clone();
            let url = url.clone();
            move || {
                let header = webview_cookie_header(&app, &url);
                let txt = webview_cookies_txt(&app, &url);
                (header, txt)
            }
        })
        .await
        .unwrap_or((None, None));
        run_task(
            app,
            state,
            spawned_id,
            url,
            format,
            dir,
            referer,
            cookie,
            cookie_header,
            cookies_txt,
        )
        .await;
    });
    id
}



async fn run_task(
    app: AppHandle,
    state: AppState,
    id: String,
    url: String,
    format: String,
    dir: String,
    referer: Option<String>,
    cookie: Option<String>,
    cookie_header: Option<String>,
    cookies_txt: Option<String>,
) {
    if !is_direct_media(&url) {
        run_yt_dlp(app, state, id, url, format, dir, referer, cookie, cookie_header, cookies_txt).await;
        return;
    }
    run_direct(
        app.clone(),
        state.clone(),
        id.clone(),
        url.clone(),
        dir.clone(),
        referer.clone(),
        cookie_header.clone(),
    )
    .await;

    if state.task(&id).is_none() || state.is_cancelled(&id) {
        return;
    }
    if state.status_of(&id) != TaskStatus::Failed {
        return;
    }
    let first = state.task(&id).map(|t| t.error).unwrap_or_default();

    state.set_error(&id, String::new());
    state.set_status(&id, TaskStatus::Queued);
    emit_task(&app, &state, &id);
    run_yt_dlp(app.clone(), state.clone(), id.clone(), url, format, dir, referer, cookie, cookie_header, cookies_txt).await;
    if state.status_of(&id) == TaskStatus::Failed {
        let second = state.task(&id).map(|t| t.error).unwrap_or_default();
        let (s_zh, s_en) = bi_part(&second);
        let (f_zh, f_en) = bi_part(&first);
        state.set_error(
            &id,
            bi(
                &format!("{s_zh}（内置下载器也失败: {f_zh}）"),
                &format!("{s_en} (built-in downloader also failed: {f_en})"),
            ),
        );
        emit_task(&app, &state, &id);
    }
}





fn is_youtube(url: &str) -> bool {
    let low = url.trim().to_ascii_lowercase();
    let host = low.split("://").nth(1).unwrap_or("").split(['/', '?', '#']).next().unwrap_or("");
    host == "youtube.com"
        || host.ends_with(".youtube.com")
        || host == "youtu.be"
        || host.ends_with(".youtu.be")
        || host == "youtube-nocookie.com"
        || host.ends_with(".youtube-nocookie.com")
}

fn is_direct_media(url: &str) -> bool {
    let low = url.trim().to_ascii_lowercase();
    if !(low.starts_with("http://") || low.starts_with("https://")) {
        return false;
    }
    let path = low.split(['?', '#']).next().unwrap_or(&low);
    
    const EXTS: [&str; 10] = [
        ".mp4", ".m4v", ".flv", ".f4v", ".mov", ".mkv", ".webm", ".ts", ".m4s", ".mp3",
    ];
    if EXTS.iter().any(|e| path.ends_with(*e)) {
        return true;
    }
    
    
    
    const CDNS: [&str; 19] = [
        "douyinvod.com",
        "bytecdntp.com",
        "douyinpic.com",
        "zjcdn.com",
        "ixigua.com",
        "ixiguavideo.com",
        "bilivideo.com",
        "hdslb.com",
        
        "kwimgs.com",
        "kwaicdn.com",
        "yximgs.com",
        "gifshow.com",
        
        
        "tiktokcdn.com",
        "tiktokcdn-us.com",
        "tiktokcdn-eu.com",
        "tiktok.com",
        "tiktokv.com",
        "byteoversea.com",
        "xhscdn.com",
    ];
    let host = low.split("://").nth(1).unwrap_or("").split('/').next().unwrap_or("");
    CDNS.iter().any(|d| host == *d || host.ends_with(&format!(".{d}")))
}





fn with_browser_headers(
    req: reqwest::RequestBuilder,
    referer: Option<&str>,
    cookie: Option<&str>,
) -> reqwest::RequestBuilder {
    let mut req = req.header(reqwest::header::ACCEPT, "*/*");
    if let Some(r) = referer {
        req = req.header(reqwest::header::REFERER, r);
        if let Some(o) = origin_of_str(r) {
            req = req.header(reqwest::header::ORIGIN, o);
        }
    }
    if let Some(c) = cookie {
        req = req.header(reqwest::header::COOKIE, c);
    }
    req
}



pub fn webview_cookie_header(app: &AppHandle, url: &str) -> Option<String> {
    let parsed: Url = url.parse().ok()?;
    let mut pairs: Vec<String> = Vec::new();
    for (_label, w) in app.webview_windows() {
        let cookies = match w.cookies_for_url(parsed.clone()) {
            Ok(c) => c,
            Err(_) => continue,
        };
        for c in cookies {
            let (n, v) = (c.name().to_string(), c.value().to_string());
            if n.is_empty() {
                continue;
            }
            let key = format!("{n}=");
            if !pairs.iter().any(|p| p.starts_with(&key)) {
                pairs.push(format!("{n}={v}"));
            }
        }
    }
    if pairs.is_empty() {
        None
    } else {
        Some(pairs.join("; "))
    }
}

pub fn webview_cookies_txt(app: &AppHandle, url: &str) -> Option<String> {
    let parsed: Url = url.parse().ok()?;
    let host = parsed.host_str()?.trim_start_matches('.').to_string();
    if host.is_empty() {
        return None;
    }
    let domain = format!(".{host}");
    let mut seen: Vec<String> = Vec::new();
    let mut out = String::from("# Netscape HTTP Cookie File\n");
    for (_label, w) in app.webview_windows() {
        let cookies = match w.cookies_for_url(parsed.clone()) {
            Ok(c) => c,
            Err(_) => continue,
        };
        for c in cookies {
            let (n, v) = (c.name().to_string(), c.value().to_string());
            if n.is_empty() || seen.iter().any(|s| *s == n) {
                continue;
            }
            seen.push(n.clone());
            out.push_str(&format!("{domain}\tTRUE\t/\tTRUE\t2145916800\t{n}\t{v}\n"));
        }
    }
    if seen.is_empty() {
        None
    } else {
        Some(out)
    }
}

fn write_cookies_file(content: &str) -> Option<std::path::PathBuf> {
    let p = std::env::temp_dir().join("wvc_cookies.txt");
    std::fs::write(&p, content).ok()?;
    Some(p)
}




fn size_from_response(resp: &reqwest::Response) -> Option<i64> {
    if let Some(total) = resp
        .headers()
        .get(reqwest::header::CONTENT_RANGE)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.rsplit('/').next())
        .and_then(|v| v.trim().parse::<i64>().ok())
        .filter(|n| *n > 0)
    {
        return Some(total);
    }
    resp.content_length().map(|n| n as i64).filter(|n| *n > 0)
}







pub async fn probe_size(url: &str, referer: Option<&str>, cookie: Option<&str>) -> Option<i64> {
    let client = reqwest::Client::builder()
        .user_agent(DESKTOP_UA)
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(15))
        .build()
        .ok()?;

    let head = with_browser_headers(client.head(url), referer, cookie);
    if let Ok(resp) = head.send().await {
        if resp.status().is_success() {
            if let Some(n) = size_from_response(&resp) {
                return Some(n);
            }
        }
    }

    let probe = with_browser_headers(
        client.get(url).header(reqwest::header::RANGE, "bytes=0-0"),
        referer,
        cookie,
    );
    size_from_response(&probe.send().await.ok()?)
}


fn build_media_client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .user_agent(DESKTOP_UA)
        .redirect(reqwest::redirect::Policy::limited(10))
        .connect_timeout(Duration::from_secs(15))
        .read_timeout(Duration::from_secs(30))
        .build()
        .map_err(|e| bi(&format!("初始化下载器失败: {e}"), &format!("Failed to initialize downloader: {e}")))
}







async fn fetch_to_file<F>(
    client: &reqwest::Client,
    url: &str,
    referer: Option<&str>,
    cookie: Option<&str>,
    dest: &std::path::Path,
    cancelled: impl Fn() -> bool,
    mut on_progress: F,
) -> Result<i64, String>
where
    F: FnMut(i64, i64),
{
    let req = with_browser_headers(client.get(url), referer, cookie);
    let mut resp = req
        .send()
        .await
        .map_err(|e| {
            bi(
                &format!("请求失败: {e} —— 链接可能已过期, 请重新「捕获」。"),
                &format!("Request failed: {e} — the link may have expired, please re-capture."),
            )
        })?;

    if cancelled() {
        let _ = std::fs::remove_file(dest);
        return Err("__cancelled__".into());
    }

    let status = resp.status();
    if !status.is_success() {
        let (h_zh, h_en) = match status.as_u16() {
            401 | 403 | 412 => (
                " —— CDN 拒绝访问(防盗链或链接已过期), 请重新「捕获」后再下载。",
                " — CDN refused access (hotlink protection or expired link). Re-capture before downloading.",
            ),
            404 | 410 => (
                " —— 链接已失效, 请重新「捕获」。",
                " — Link expired. Please re-capture.",
            ),
            _ => ("", ""),
        };
        return Err(bi(
            &format!("HTTP {}{h_zh}", status.as_u16()),
            &format!("HTTP {}{h_en}", status.as_u16()),
        ));
    }

    let ct = resp
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_ascii_lowercase();
    
    if ct.starts_with("text/") || ct.contains("json") || ct.contains("html") {
        return Err(bi(
            &format!("服务器返回的是网页/文本({ct})而非视频, 多为防盗链拦截。请重新「捕获」后用可见窗口播放一次再下载。"),
            &format!("Server returned a web page/text ({ct}) instead of video, usually hotlink protection. Re-capture, then play once in the visible window before downloading."),
        ));
    }

    let total = resp.content_length().unwrap_or(0) as i64;
    let mut file = std::fs::File::create(dest)
        .map_err(|e| bi(&format!("创建文件失败: {e}"), &format!("Failed to create file: {e}")))?;
    let mut downloaded: i64 = 0;
    let mut last_emit = Instant::now();
    loop {
        if cancelled() {
            drop(file);
            let _ = std::fs::remove_file(dest);
            return Err("__cancelled__".into());
        }
        let chunk = match resp.chunk().await {
            Ok(Some(c)) => c,
            Ok(None) => break,
            Err(e) => {
                drop(file);
                let _ = std::fs::remove_file(dest);
                return Err(bi(
                    &format!("下载中断: {e} —— 可点「重试」; 若仍失败请重新「捕获」。"),
                    &format!("Download interrupted: {e} — you can retry; if it still fails, re-capture."),
                ));
            }
        };
        if let Err(e) = file.write_all(&chunk) {
            drop(file);
            let _ = std::fs::remove_file(dest);
            return Err(bi(&format!("写入文件失败: {e}"), &format!("Failed to write file: {e}")));
        }
        downloaded += chunk.len() as i64;
        
        if last_emit.elapsed() >= Duration::from_millis(200) {
            last_emit = Instant::now();
            on_progress(downloaded, total);
        }
    }
    if let Err(e) = file.flush() {
        drop(file);
        let _ = std::fs::remove_file(dest);
        return Err(bi(&format!("写入文件失败: {e}"), &format!("Failed to write file: {e}")));
    }
    drop(file);
    
    on_progress(downloaded, total);
    Ok(downloaded)
}





pub fn enqueue_pair(
    app: AppHandle,
    state: AppState,
    a_url: String,
    b_url: String,
    out_dir: String,
) -> String {
    for t in state.tasks().values() {
        if t.url == a_url
            && t.status == TaskStatus::Done
            && !t.file_path.is_empty()
            && std::path::Path::new(&t.file_path).exists()
        {
            emit_task(&app, &state, &t.id);
            return t.id.clone();
        }
    }
    let id = format!("task_{}", now_ms());
    let dir = if out_dir.trim().is_empty() {
        default_out_dir()
    } else {
        out_dir.trim().to_string()
    };
    let _ = std::fs::create_dir_all(&dir);
    let cap_title = state.capture_title();
    let title = if cap_title.trim().is_empty() {
        short_title(&a_url)
    } else {
        cap_title
    };
    let task = DownloadTask {
        id: id.clone(),
        url: a_url.clone(),
        title,
        out_path: dir.clone(),
        status: TaskStatus::Queued,
        progress: 0.0,
        speed_bps: 0.0,
        downloaded: 0,
        total: 0,
        error: "".into(),
        format: Some("merge".into()),
        file_path: "".into(),
        pair_url: b_url.clone(),
    };
    state.upsert_task(task);
    emit_task(&app, &state, &id);
    state.mark_cancel(&id, false);
    let ra = state.referer_of(&a_url);
    let rb = state.referer_of(&b_url);
    let spawned_id = id.clone();
    tauri::async_runtime::spawn(async move {
        let ca = tauri::async_runtime::spawn_blocking({
            let app = app.clone();
            let a_url = a_url.clone();
            move || webview_cookie_header(&app, &a_url)
        })
        .await
        .unwrap_or(None);
        let cb = tauri::async_runtime::spawn_blocking({
            let app = app.clone();
            let b_url = b_url.clone();
            move || webview_cookie_header(&app, &b_url)
        })
        .await
        .unwrap_or(None);
        run_pair(app, state, spawned_id, a_url, b_url, dir, ra, rb, ca, cb).await;
    });
    id
}


async fn run_pair(
    app: AppHandle,
    state: AppState,
    id: String,
    a_url: String,
    b_url: String,
    dir: String,
    ra: Option<String>,
    rb: Option<String>,
    ca: Option<String>,
    cb: Option<String>,
) {
    use std::sync::atomic::{AtomicI64, Ordering};
    cleanup_parts(&dir, &id);
    let client = match build_media_client() {
        Ok(c) => c,
        Err(e) => return fail_direct(&app, &state, &id, e),
    };

    let pa = std::path::PathBuf::from(&dir).join(format!("{id}__A.bin"));
    let pb = std::path::PathBuf::from(&dir).join(format!("{id}__B.bin"));

    
    let prog = std::sync::Arc::new([
        AtomicI64::new(0),
        AtomicI64::new(0),
        AtomicI64::new(0),
        AtomicI64::new(0),
    ]);
    let t0 = Instant::now();
    let make_cb = |slot: usize| {
        let p = prog.clone();
        let st = state.clone();
        let ap = app.clone();
        let tid = id.clone();
        move |d: i64, t: i64| {
            p[slot * 2].store(d, Ordering::Relaxed);
            if t > 0 {
                p[slot * 2 + 1].store(t, Ordering::Relaxed);
            }
            let down = p[0].load(Ordering::Relaxed)
                + p[2].load(Ordering::Relaxed);
            let tot = p[1].load(Ordering::Relaxed) + p[3].load(Ordering::Relaxed);
            if let Some(mut t) = st.task(&tid) {
                t.downloaded = down;
                t.status = TaskStatus::Downloading;
                if tot > 0 {
                    t.total = tot;
                    
                    t.progress = (down as f32 / tot as f32).clamp(0.0, 0.99);
                }
                t.speed_bps = down as f64 / t0.elapsed().as_secs_f64().max(0.001);
                st.upsert_task(t);
            }
            emit_task(&ap, &st, &tid);
        }
    };
    let cancel_a = {
        let st = state.clone();
        let tid = id.clone();
        move || st.is_cancelled(&tid)
    };
    let cancel_b = {
        let st = state.clone();
        let tid = id.clone();
        move || st.is_cancelled(&tid)
    };

    
    let (ra_res, rb_res) = tokio::join!(
        fetch_to_file(&client, &a_url, ra.as_deref(), ca.as_deref(), &pa, cancel_a, make_cb(0)),
        fetch_to_file(&client, &b_url, rb.as_deref(), cb.as_deref(), &pb, cancel_b, make_cb(1))
    );

    
    if state.is_cancelled(&id) {
        let _ = std::fs::remove_file(&pa);
        let _ = std::fs::remove_file(&pb);
        state.set_status(&id, TaskStatus::Cancelled);
        emit_task(&app, &state, &id);
        return;
    }
    if let Err(e) = &ra_res {
        let _ = std::fs::remove_file(&pb);
        let (e_zh, e_en) = bi_part(e);
        return fail_direct(
            &app,
            &state,
            &id,
            bi(
                &format!("第一条流下载失败: {e_zh}"),
                &format!("Failed to download the video stream: {e_en}"),
            ),
        );
    }
    if let Err(e) = &rb_res {
        let _ = std::fs::remove_file(&pa);
        let (e_zh, e_en) = bi_part(e);
        return fail_direct(
            &app,
            &state,
            &id,
            bi(
                &format!("第二条流下载失败: {e_zh}"),
                &format!("Failed to download the audio stream: {e_en}"),
            ),
        );
    }

    
    for p in [&pa, &pb] {
        if !valid_media(p) {
            let size = std::fs::metadata(p).map(|m| m.len()).unwrap_or(0);
            let _ = std::fs::remove_file(&pa);
            let _ = std::fs::remove_file(&pb);
            return fail_direct(
                &app,
                &state,
                &id,
                bi(
                    &format!(
                        "其中一条流只拿到 {size} 字节的非视频内容(疑似防盗链拦截)。请重新「捕获」后再合并。"
                    ),
                    &format!(
                        "One of the streams only got {size} bytes of non-video content (likely hotlink protection). Re-capture before merging."
                    ),
                ),
            );
        }
    }

    state.set_status(&id, TaskStatus::Merging);
    state.set_error(&id, String::new());
    emit_task(&app, &state, &id);

    let out_tmp = std::path::PathBuf::from(&dir).join(format!("{id}__merged.mp4"));
    let ps = |p: &std::path::PathBuf| p.to_string_lossy().to_string();
    if let Err(e) = crate::merge::merge_av(&ps(&pa), &ps(&pb), &ps(&out_tmp)) {
        let _ = std::fs::remove_file(&pa);
        let _ = std::fs::remove_file(&pb);
        let _ = std::fs::remove_file(&out_tmp);
        return fail_direct(&app, &state, &id, e);
    }
    
    let _ = std::fs::remove_file(&pa);
    let _ = std::fs::remove_file(&pb);

    if !valid_media(&out_tmp) {
        let size = std::fs::metadata(&out_tmp).map(|m| m.len()).unwrap_or(0);
        let _ = std::fs::remove_file(&out_tmp);
        return fail_direct(
            &app,
            &state,
            &id,
            bi(
                &format!("合并产物异常(仅 {size} 字节), 已丢弃。"),
                &format!("Merged output is abnormal (only {size} bytes), discarded."),
            ),
        );
    }

    let final_path = finalize_name(&out_tmp, &state, &a_url, &dir, None);
    let fp = final_path.to_string_lossy().to_string();
    state.set_title(&id, file_stem(&fp));
    state.set_file_path(&id, fp);
    if let Some(mut t) = state.task(&id) {
        t.status = TaskStatus::Done;
        t.progress = 1.0;
        let size = std::fs::metadata(&final_path).map(|m| m.len()).unwrap_or(0);
        if size > 0 {
            t.total = size as i64;
            t.downloaded = size as i64;
        }
        state.upsert_task(t);
    }
    emit_task(&app, &state, &id);
}





async fn run_direct(
    app: AppHandle,
    state: AppState,
    id: String,
    url: String,
    dir: String,
    referer: Option<String>,
    cookie_header: Option<String>,
) {
    
    cleanup_parts(&dir, &id);

    let client = match build_media_client() {
        Ok(c) => c,
        Err(e) => return fail_direct(&app, &state, &id, e),
    };

    
    let ext = guess_ext(&url, "");
    let part = std::path::PathBuf::from(&dir).join(format!("{id}__dl.{ext}.part"));

    let t0 = Instant::now();
    state.set_status(&id, TaskStatus::Downloading);
    emit_task(&app, &state, &id);
    let cancel_st = state.clone();
    let tid_cancel = id.clone();
    let prog_st = state.clone();
    let prog_app = app.clone();
    let tid_prog = id.clone();
    let downloaded = match fetch_to_file(
        &client,
        &url,
        referer.as_deref(),
        cookie_header.as_deref(),
        &part,
        move || cancel_st.is_cancelled(&tid_cancel),
        move |down: i64, tot: i64| {
            push_progress(
                &prog_st,
                &tid_prog,
                down,
                tot,
                t0.elapsed().as_secs_f64().max(0.001),
            );
            emit_task(&prog_app, &prog_st, &tid_prog);
        },
    )
    .await
    {
        Ok(n) => n,
        Err(e) => {
            
            if e == "__cancelled__" {
                state.set_status(&id, TaskStatus::Cancelled);
                emit_task(&app, &state, &id);
                return;
            }
            return fail_direct(&app, &state, &id, e);
        }
    };

    
    if !valid_media(&part) {
        let size = std::fs::metadata(&part).map(|m| m.len()).unwrap_or(0);
        let _ = std::fs::remove_file(&part);
        return fail_direct(
            &app,
            &state,
            &id,
            bi(
                &format!("只拿到 {size} 字节的非视频内容(疑似防盗链/风控页)。请重新「捕获」后用可见窗口播放一次再下载。"),
                &format!("Only got {size} bytes of non-video content (likely hotlink protection / risk-control page). Re-capture, then play once in the visible window before downloading."),
            ),
        );
    }

    
    let raw = std::path::PathBuf::from(&dir).join(format!("{id}__dl.{ext}"));
    let _ = std::fs::rename(&part, &raw);
    let src = if raw.exists() { raw.as_path() } else { part.as_path() };
    let final_path = finalize_name(src, &state, &url, &dir, Some(short_title(&url)));
    let fp = final_path.to_string_lossy().to_string();
    state.set_title(&id, file_stem(&fp));
    state.set_file_path(&id, fp);
    if let Some(mut t) = state.task(&id) {
        t.status = TaskStatus::Done;
        t.progress = 1.0;
        t.downloaded = downloaded;
        if t.total <= 0 {
            t.total = downloaded;
        }
        state.upsert_task(t);
    }
    emit_task(&app, &state, &id);
}


fn push_progress(state: &AppState, id: &str, downloaded: i64, total: i64, secs: f64) {
    if let Some(mut t) = state.task(id) {
        t.downloaded = downloaded;
        if total > 0 {
            t.total = total;
            t.progress = (downloaded as f32 / total as f32).clamp(0.0, 1.0);
        }
        t.speed_bps = downloaded as f64 / secs;
        t.status = TaskStatus::Downloading;
        state.upsert_task(t);
    }
}


fn fail_direct(app: &AppHandle, state: &AppState, id: &str, msg: String) {
    state.set_error(id, msg);
    state.set_status(id, TaskStatus::Failed);
    emit_task(app, state, id);
}


fn cleanup_parts(dir: &str, id: &str) {
    let prefix = format!("{id}__");
    if let Ok(rd) = std::fs::read_dir(dir) {
        for e in rd.flatten() {
            let n = e.file_name().to_string_lossy().to_string();
            if n.starts_with(&prefix) && n.to_ascii_lowercase().ends_with(".part") {
                let _ = std::fs::remove_file(e.path());
            }
        }
    }
}


fn origin_of_str(u: &str) -> Option<String> {
    let (scheme, rest) = u.split_once("://")?;
    let host = rest.split(['/', '?', '#']).next()?;
    if scheme.is_empty() || host.is_empty() {
        None
    } else {
        Some(format!("{scheme}://{host}"))
    }
}


fn guess_ext(url: &str, ct: &str) -> String {
    let path = url.split(['?', '#']).next().unwrap_or(url).to_ascii_lowercase();
    for e in [
        "mp4", "m4v", "flv", "f4v", "mov", "mkv", "webm", "ts", "m4s", "mp3",
    ] {
        if path.ends_with(&format!(".{e}")) {
            return e.to_string();
        }
    }
    for (k, v) in [
        ("flv", "flv"),
        ("mp2t", "ts"),
        ("mpeg-ts", "ts"),
        ("webm", "webm"),
        ("matroska", "mkv"),
        ("quicktime", "mov"),
        ("audio/", "m4a"),
    ] {
        if ct.contains(k) {
            return v.to_string();
        }
    }
    "mp4".into()
}


async fn run_yt_dlp(
    app: AppHandle,
    state: AppState,
    id: String,
    url: String,
    format: String,
    dir: String,
    referer: Option<String>,
    cookie_browser: Option<String>,
    cookie_header: Option<String>,
    cookies_txt: Option<String>,
) {
    
    cleanup_parts(&dir, &id);
    
    let out_tmpl = format!("{}/{}__%(title).60s.%(ext)s", dir, id);
    
    let yt_format = if format.trim() == "best" {
        
        
        "bestvideo+bestaudio/best".to_string()
    } else {
        format.clone()
    };
    let yt_attempts: Vec<Option<String>> = if is_youtube(&url) {
        vec![
            Some("youtube:player_client=default,web_embedded".into()),
            Some("youtube:player_client=tv,web_embedded;player_skip=webpage".into()),
            Some("youtube:player_client=android".into()),
        ]
    } else {
        vec![None]
    };
    for (att, extra_ea) in yt_attempts.iter().enumerate() {
    let mut cmd = TokioCommand::new("yt-dlp");
    #[cfg(windows)]
    cmd.creation_flags(CREATE_NO_WINDOW);
    cmd.arg("-f")
        .arg(&yt_format)
        .arg("-o")
        .arg(&out_tmpl)
        .arg("--no-playlist")
        .arg("--newline")
        
        
        .arg("--windows-filenames")
        .arg("--trim-filenames")
        .arg("120")
        .arg("--socket-timeout")
        .arg("20")
        .arg("--retries")
        .arg("5")
        .arg("--fragment-retries")
        .arg("5")
        
        
        .arg("--merge-output-format")
        .arg("mp4")
        .arg("--user-agent")
        .arg(DESKTOP_UA);
    if let Some(r) = &referer {
        cmd.arg("--add-header").arg(format!("Referer: {r}"));
    }
    let mut cookies_file: Option<std::path::PathBuf> = None;
    if let Some(txt) = &cookies_txt {
        if let Some(p) = write_cookies_file(txt) {
            cookies_file = Some(p.clone());
            cmd.arg("--cookies").arg(&p);
        }
    }
    if cookies_file.is_none() {
        if let Some(c) = &cookie_header {
            cmd.arg("--add-header").arg(format!("Cookie: {c}"));
        }
    }
    if let Some(b) = &cookie_browser {
        cmd.arg("--cookies-from-browser").arg(b);
    }
    if is_youtube(&url) {
        cmd.arg("--js-runtimes").arg("node");
        if let Some(ea) = extra_ea {
            cmd.arg("--extractor-args").arg(ea);
        }
    }
    cmd.arg(&url);
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());

    let mut child = match cmd.spawn() {
        Ok(c) => c,
        Err(e) => {
            state.set_error(
                &id,
                bi(
                    &format!("启动 yt-dlp 失败: {e}（请确认本机已安装 yt-dlp 并在 PATH 中）"),
                    &format!("Failed to launch yt-dlp: {e} (make sure yt-dlp is installed and in PATH)"),
                ),
            );
            state.set_status(&id, TaskStatus::Failed);
            emit_task(&app, &state, &id);
            return;
        }
    };
    state.set_status(&id, TaskStatus::Downloading);
    emit_task(&app, &state, &id);

    
    if let Some(out) = child.stdout.take() {
        let st = state.clone();
        let tid = id.clone();
        let a = app.clone();
        tauri::async_runtime::spawn(async move {
            let mut lines = BufReader::new(out).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                apply_progress(&st, &tid, line.trim());
                emit_task(&a, &st, &tid);
            }
        });
    }

    
    let mut last_err = String::new();
    if let Some(out) = child.stderr.take() {
        let mut lines = BufReader::new(out).lines();
        while let Ok(Some(line)) = lines.next_line().await {
            if state.is_cancelled(&id) {
                let _ = child.kill().await;
                state.set_status(&id, TaskStatus::Cancelled);
                emit_task(&app, &state, &id);
                return;
            }
            let l = line.trim().to_string();
            if l.contains("ERROR:") {
                last_err = l.clone();
            }
            apply_progress(&state, &id, &l);
            emit_task(&app, &state, &id);
        }
    }
    let status = child.wait().await;

    
    if state.is_cancelled(&id) {
        state.set_status(&id, TaskStatus::Cancelled);
        emit_task(&app, &state, &id);
        return;
    }

    
    if !status.map(|s| s.success()).unwrap_or(false) {
        let msg = if last_err.is_empty() {
            bi(
                "yt-dlp 下载失败(进程退出码非 0)",
                "yt-dlp download failed (non-zero exit code)",
            )
        } else {
            friendly_err(&last_err)
        };
        let is403 = last_err.contains("403") || last_err.to_ascii_lowercase().contains("forbidden");
        if is403 && att + 1 < yt_attempts.len() {
            continue;
        }
        state.set_error(&id, msg);
        state.set_status(&id, TaskStatus::Failed);
        emit_task(&app, &state, &id);
        return;
    }
    break;
    }



    match find_output(&dir, &id) {
        Some(p) if valid_media(&p) => {
            let fp = finalize_name(&p, &state, &url, &dir, None);
            let fp_str = fp.to_string_lossy().to_string();
            state.set_title(&id, file_stem(&fp_str));
            state.set_file_path(&id, fp_str);
            state.set_status(&id, TaskStatus::Done);
            emit_task(&app, &state, &id);
        }
        Some(p) => {
            let size = std::fs::metadata(&p).map(|m| m.len()).unwrap_or(0);
            let _ = std::fs::remove_file(&p); 
            state.set_error(
                &id,
                bi(
                    &format!("下载到的是 {size} 字节的非视频内容, 多为站点防盗链拦截。请勾选「弹窗」用可见窗口播放一次再下载, 或改用「解析」。"),
                    &format!("Downloaded {size} bytes of non-video content, usually blocked by hotlink protection. Enable \"visible window\" and play once before downloading, or use the parser instead."),
                ),
            );
            state.set_status(&id, TaskStatus::Failed);
            emit_task(&app, &state, &id);
        }
        None => {
            state.set_error(
                &id,
                bi(
                    "未找到下载产物: 链接可能已过期或被防盗链拦截。请重新「捕获」后再下载。",
                    "No output file found: the link may have expired or been blocked by hotlink protection. Re-capture before downloading.",
                ),
            );
            state.set_status(&id, TaskStatus::Failed);
            emit_task(&app, &state, &id);
        }
    }
}



pub async fn resolve_page(app: AppHandle, state: AppState, url: String) {
    let mut cmd = TokioCommand::new("yt-dlp");
    #[cfg(windows)]
    cmd.creation_flags(CREATE_NO_WINDOW);
    cmd.arg("-J").arg("--no-playlist").arg(&url);
    let cookies_txt = tauri::async_runtime::spawn_blocking({
        let app = app.clone();
        let u = url.clone();
        move || webview_cookies_txt(&app, &u)
    })
    .await
    .unwrap_or(None);
    if let Some(txt) = &cookies_txt {
        if let Some(p) = write_cookies_file(txt) {
            cmd.arg("--cookies").arg(&p);
        }
    }
    if let Some(b) = state.cookie_browser() {
        cmd.arg("--cookies-from-browser").arg(b);
    }
    if is_youtube(&url) {
        cmd.arg("--js-runtimes").arg("node");
        cmd.arg("--extractor-args")
            .arg("youtube:player_client=default,web_embedded");
    }
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());

    match cmd.output().await {
        Ok(out) if out.status.success() => {
            let s = String::from_utf8_lossy(&out.stdout);
            match serde_json::from_str::<serde_json::Value>(&s) {
                Ok(v) => {
                    let title = v["title"].as_str().unwrap_or("未知标题").to_string();
                    let page_url = v["webpage_url"]
                        .as_str()
                        .or_else(|| v["url"].as_str())
                        .unwrap_or(&url)
                        .to_string();
                    let mut formats: Vec<ResolvedFormat> = Vec::new();
                    formats.push(ResolvedFormat {
                        format_id: "best".into(),
                        quality: "best".into(),
                        ext: "mp4".into(),
                        kind: "Video".into(),
                        size: -1,
                    });
                    if let Some(arr) = v["formats"].as_array() {
                        for f in arr {
                            if f["ext"].as_str() == Some("mhtml") {
                                continue;
                            }
                            let fid = match f["format_id"].as_str() {
                                Some(x) if !x.is_empty() => x.to_string(),
                                _ => continue,
                            };
                            let ext = f["ext"].as_str().unwrap_or("").to_string();
                            let height = f["height"].as_u64().unwrap_or(0);
                            let quality = if height > 0 {
                                format!("{}P", height)
                            } else if f["acodec"].as_str() == Some("none") {
                                "视频流".into()
                            } else {
                                "音频流".into()
                            };
                            let kind = if f["vcodec"].as_str() == Some("none") {
                                "Audio"
                            } else {
                                "Video"
                            }
                            .to_string();
                            let size = f["filesize"].as_i64().unwrap_or(-1);
                            formats.push(ResolvedFormat {
                                format_id: fid,
                                quality,
                                ext,
                                kind,
                                size,
                            });
                        }
                    }
                    let payload = ResolvedPage {
                        url: page_url,
                        title,
                        formats,
                    };
                    let _ = app.emit("resolve://formats", &payload);
                }
                Err(e) => emit_resolve_error(&app, &url, &format!("返回内容不是有效 JSON: {e}")),
            }
        }
        Ok(out) => {
            let stderr = String::from_utf8_lossy(&out.stderr);
            emit_resolve_error(&app, &url, &extract_error(&stderr));
        }
        Err(e) => emit_resolve_error(&app, &url, &format!("启动 yt-dlp 失败: {e}（请确认本机已安装 yt-dlp 并在 PATH 中）")),
    }
}


fn emit_resolve_error(app: &AppHandle, url: &str, err: &str) {
    eprintln!("[resolve] 失败: {err}");
    let _ = app.emit(
        "resolve://error",
        serde_json::json!({ "url": url, "error": err }),
    );
}

// 通用方案: 让 yt-dlp 自更新(不再依赖人工每 90 天检查)。-U 比对 GitHub 最新版, 仅必要时下载, 平时只是一次轻量请求。
pub async fn update_ytdlp() -> String {
    let mut cmd = TokioCommand::new("yt-dlp");
    #[cfg(windows)]
    cmd.creation_flags(CREATE_NO_WINDOW);
    cmd.arg("-U").arg("--quiet").arg("--no-warnings");
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
    match cmd.output().await {
        Ok(_) => {
            let mut v = TokioCommand::new("yt-dlp");
            #[cfg(windows)]
            v.creation_flags(CREATE_NO_WINDOW);
            v.arg("--version");
            if let Ok(o) = v.output().await {
                let ver = String::from_utf8_lossy(&o.stdout).trim().to_string();
                if !ver.is_empty() {
                    return bi(
                        &format!("yt-dlp 自检/更新完成, 当前版本 {ver}"),
                        &format!("yt-dlp checked/updated, current version {ver}"),
                    );
                }
            }
            bi("yt-dlp 更新完成(无法读取版本号)", "yt-dlp update done (version unknown)")
        }
        Err(e) => bi(
            &format!("未找到 yt-dlp 或更新失败: {e}（请确认本机已安装 yt-dlp 并在 PATH 中）"),
            &format!("yt-dlp not found or update failed: {e} (make sure yt-dlp is installed and on PATH)"),
        ),
    }
}


fn extract_error(stderr: &str) -> String {
    let key = stderr
        .lines()
        .filter(|l| l.contains("ERROR:") || l.contains("is not a valid URL"))
        .last()
        .map(|l| l.trim().to_string())
        .unwrap_or_else(|| stderr.lines().last().unwrap_or("未知错误").trim().to_string());
    
    if key.contains("Unsupported URL") {
        format!(
            "{key} —— yt-dlp 没有这个站点的解析器(快手/小红书等国内站点已不在 yt-dlp 支持列表里)。\
             这不是链接坏了: 请改用「🧲 捕获」, 让内置浏览器真实渲染页面即可自动抓到直链。"
        )
    } else if key.contains("Fresh cookies") || key.contains("cookies") {
        format!("{key} —— 该平台需要页面签名/新鲜 cookie, yt-dlp 解析不了。请改用「捕获」页: 用内置浏览器打开并播放即可自动抓直链")
    } else {
        key
    }
}


fn friendly_err(msg: &str) -> String {
    let m = msg.to_ascii_lowercase();
    if m.contains("403") || m.contains("forbidden") || m.contains("412") {
        bi(
            &format!("{msg} —— CDN 拒绝访问(签名/风控)。已自动换客户端重试仍失败: 请更新 yt-dlp(pip install -U yt-dlp yt-dlp-ejs)并确保 node 在 PATH 中, 再试; 或勾选「弹窗」播放一次后下载。"),
            &format!("{msg} — CDN refused access (signature/bot risk). Client auto-retry exhausted: update yt-dlp (pip install -U yt-dlp yt-dlp-ejs), make sure node is in PATH, then retry; or enable \"visible window\", play once, then download."),
        )
    } else if msg.contains("cookies") {
        bi(
            &format!("{msg} —— 该平台需要登录态 cookie, 可在「设置」里指定浏览器读取 cookie。"),
            &format!("{msg} — this platform requires login cookies; pick a browser in Settings for cookie import."),
        )
    } else {
        msg.to_string()
    }
}


pub fn test_engine() -> EngineInfo {
    let mut ytc = std::process::Command::new("yt-dlp");
    ytc.arg("--version");
    #[cfg(windows)]
    ytc.creation_flags(CREATE_NO_WINDOW);
    let yt = ytc.output();
    let yt_ok = yt.as_ref().map(|o| o.status.success()).unwrap_or(false);
    let yt_ver = yt
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default();
    let mut ffc = std::process::Command::new("ffmpeg");
    ffc.arg("-version");
    #[cfg(windows)]
    ffc.creation_flags(CREATE_NO_WINDOW);
    let ff = ffc.output();
    let ff_ok = ff.as_ref().map(|o| o.status.success()).unwrap_or(false);
    EngineInfo {
        yt_dlp: yt_ok,
        ffmpeg: ff_ok,
        yt_dlp_version: if yt_ok { yt_ver } else { String::new() },
    }
}

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

pub fn emit_task(app: &AppHandle, state: &AppState, id: &str) {
    if let Some(t) = state.task(id) {
        let _ = app.emit("download://task", &t);
    }
}


fn apply_progress(state: &AppState, id: &str, line: &str) {
    let l = line.trim();
    if l.contains("Merger") || l.contains("[ffmpeg]") || l.contains("Merging") {
        state.set_status(id, TaskStatus::Merging);
    }
    let pct = parse_pct(l);
    let total = parse_total(l);
    let speed = parse_speed(l);
    if pct.is_none() && total.is_none() && speed.is_none() {
        return;
    }
    if let Some(mut t) = state.task(id) {
        if let Some(p) = pct {
            t.progress = p;
            t.status = TaskStatus::Downloading;
        }
        if let Some(tot) = total {
            t.total = tot;
            t.downloaded = (t.progress * tot as f32) as i64;
        }
        if let Some(s) = speed {
            t.speed_bps = s;
        }
        state.upsert_task(t);
    }
}


fn find_output(dir: &str, id: &str) -> Option<std::path::PathBuf> {
    let prefix = format!("{id}__");
    let mut best: Option<(u64, std::path::PathBuf)> = None;
    if let Ok(rd) = std::fs::read_dir(dir) {
        for e in rd.flatten() {
            let name = e.file_name().to_string_lossy().to_string();
            if !name.starts_with(&prefix) {
                continue;
            }
            let low = name.to_ascii_lowercase();
            if low.ends_with(".part") || low.ends_with(".ytdl") || low.ends_with(".temp") {
                continue;
            }
            let len = e.metadata().map(|m| m.len()).unwrap_or(0);
            if best.as_ref().map(|(l, _)| len > *l).unwrap_or(true) {
                best = Some((len, e.path()));
            }
        }
    }
    best.map(|(_, p)| p)
}


fn valid_media(p: &std::path::Path) -> bool {
    let meta = match std::fs::metadata(p) {
        Ok(m) => m,
        Err(_) => return false,
    };
    if meta.len() < 1024 {
        return false;
    }
    let mut head = [0u8; 32];
    let n = match std::fs::File::open(p) {
        Ok(mut f) => f.read(&mut head).unwrap_or(0),
        Err(_) => return false,
    };
    if n == 0 {
        return false;
    }
    let h = &head[..n];
    
    !h.iter().all(|b| b.is_ascii_graphic() || *b == b'\n' || *b == b'\r' || *b == b'\t')
}


fn sniff_ext(p: &std::path::Path) -> Option<&'static str> {
    let mut head = [0u8; 64];
    let n = std::fs::File::open(p).and_then(|mut f| f.read(&mut head)).unwrap_or(0);
    if n < 4 {
        return None;
    }
    let h = &head[..n];
    if n >= 8 && &h[4..8] == b"ftyp" {
        return Some("mp4");
    }
    if &h[..3] == b"FLV" {
        return Some("flv");
    }
    if h[..4] == [0x1A, 0x45, 0xDF, 0xA3] {
        
        return Some(if contains(h, b"webm") { "webm" } else { "mkv" });
    }
    if h[0] == 0x47 {
        return Some("ts");
    }
    None
}


fn contains(hay: &[u8], needle: &[u8]) -> bool {
    hay.windows(needle.len()).any(|w| w == needle)
}



fn finalize_name(
    p: &std::path::Path,
    state: &AppState,
    url: &str,
    dir: &str,
    fallback: Option<String>,
) -> std::path::PathBuf {
    let mut ext = p
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    if let Some(e) = sniff_ext(p) {
        ext = e.to_string();
    }
    let raw_stem = p
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();
    let cap = state.capture_title();
    let base_raw = if !cap.trim().is_empty() {
        cap
    } else {
        fallback.unwrap_or_else(|| host_of(url))
    };
    let mut base = sanitize(&base_raw);
    if base.is_empty() {
        base = sanitize(&raw_stem);
    }
    if base.is_empty() || ext.is_empty() {
        return p.to_path_buf();
    }
    let mut target = std::path::PathBuf::from(dir).join(format!("{base}.{ext}"));
    let mut n = 1;
    while target.exists() && target.as_path() != p {
        target = std::path::PathBuf::from(dir).join(format!("{base} ({n}).{ext}"));
        n += 1;
    }
    if target.as_path() == p {
        return p.to_path_buf();
    }
    match std::fs::rename(p, &target) {
        Ok(_) => target,
        Err(_) => p.to_path_buf(),
    }
}



fn sanitize(s: &str) -> String {
    let cleaned: String = s
        .chars()
        .map(|c| if c.is_control() || "\\/:*?\"<>|%".contains(c) { '_' } else { c })
        .collect();
    let t = cleaned.trim().trim_matches('.').trim();
    let t: String = if t.chars().count() > 80 { t.chars().take(80).collect() } else { t.to_string() };
    t.trim().to_string()
}

fn default_out_dir() -> String {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.join("downloads")))
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|| "downloads".into())
}

fn now_ms() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0)
}


fn short_title(url: &str) -> String {
    let base = url.split(['?', '#']).next().unwrap_or(url);
    let seg = base.trim_end_matches('/').rsplit('/').next().unwrap_or("");
    let looks_ok = !seg.is_empty()
        && seg.chars().any(|c| c.is_ascii_alphanumeric())
        && seg.chars().filter(|c| c.is_ascii_alphanumeric()).count() >= 4;
    if looks_ok {
        if seg.chars().count() > 60 {
            seg.chars().take(60).collect()
        } else {
            seg.to_string()
        }
    } else {
        host_of(url)
    }
}


fn file_stem(path: &str) -> String {
    std::path::Path::new(path)
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| path.to_string())
}


fn parse_pct(line: &str) -> Option<f32> {
    let b = line.as_bytes();
    for i in 0..b.len() {
        if b[i] == b'%' {
            let mut j = i;
            while j > 0 && (b[j - 1].is_ascii_digit() || b[j - 1] == b'.') {
                j -= 1;
            }
            let s = line[j..i].trim();
            if let Ok(v) = s.parse::<f32>() {
                return Some(v / 100.0);
            }
        }
    }
    None
}


fn parse_total(line: &str) -> Option<i64> {
    let pos = line.find(" of ")?;
    let tok = line[pos + 4..].split_whitespace().next()?;
    Some(parse_size_token(tok))
}


fn parse_speed(line: &str) -> Option<f64> {
    let pos = line.find(" at ")?;
    let tok = line[pos + 4..].split_whitespace().next()?;
    let t = tok.trim_end_matches("/s");
    Some(parse_size_token(t) as f64)
}


fn parse_size_token(tok: &str) -> i64 {
    let (num, unit) = split_num_unit(tok);
    let v: f64 = num.parse().unwrap_or(0.0);
    let mul = match unit.to_uppercase().as_str() {
        "KI" | "KIB" | "K" => 1024.0,
        "MI" | "MIB" | "M" => 1024.0 * 1024.0,
        "GI" | "GIB" | "G" => 1024.0 * 1024.0 * 1024.0,
        _ => 1.0,
    };
    (v * mul) as i64
}


fn split_num_unit(tok: &str) -> (&str, &str) {
    let bytes = tok.as_bytes();
    let mut i = 0;
    while i < bytes.len() && (bytes[i].is_ascii_digit() || bytes[i] == b'.') {
        i += 1;
    }
    (&tok[..i], &tok[i..])
}
