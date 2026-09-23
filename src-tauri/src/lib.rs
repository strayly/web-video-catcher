
mod schema;
mod state;
mod cert;
mod sniffer;
mod proxy;
mod download;
mod merge;
mod capture;

use crate::schema::*;
use crate::state::AppState;
use tauri::{AppHandle, Manager, State};

#[tauri::command]
fn health_check() -> String {
    "ok".into()
}

#[tauri::command]
fn proxy_status(state: State<AppState>) -> ProxyStatus {
    ProxyStatus {
        running: state.proxy_running(),
        addr: state.proxy_addr(),
        ca_installed: false,
    }
}

#[tauri::command]
fn start_proxy(app: AppHandle, state: State<AppState>) -> ProxyStatus {
    if !state.proxy_running() {
        state.set_proxy_running(true);
        
        if let Ok(cfg) = app.path().app_config_dir() {
            let ca_dir = cfg.join("ca");
            if let Err(e) = cert::ensure_ca(&ca_dir) {
                eprintln!("[cert] 生成 CA 失败: {e}");
            }
        }
        let st = state.inner().clone();
        let a = app.clone();
        let addr = state.proxy_addr();
        tauri::async_runtime::spawn(async move {
            proxy::run_proxy(a, st, addr).await;
        });
    }
    proxy_status(state)
}

#[tauri::command]
fn stop_proxy(state: State<AppState>) -> ProxyStatus {
    state.set_proxy_running(false);
    proxy_status(state)
}

#[tauri::command]
fn list_media(state: State<AppState>) -> Vec<MediaItem> {
    state.media()
}

#[tauri::command]
fn clear_media(state: State<AppState>) {
    state.clear_media();
}

#[tauri::command]
fn resolve_page(app: AppHandle, state: State<AppState>, url: String) {
    let st = state.inner().clone();
    let a = app.clone();
    tauri::async_runtime::spawn(async move {
        download::resolve_page(a, st, url).await;
    });
}

#[tauri::command]
fn download(app: AppHandle, state: State<AppState>, url: String, opts: DownloadOptions) -> String {
    let st = state.inner().clone();
    download::enqueue(app, st, url, opts.format, opts.out_dir)
}

#[tauri::command]
fn list_tasks(state: State<AppState>) -> Vec<DownloadTask> {
    state.tasks().into_values().collect()
}

#[tauri::command]
fn cancel_task(app: AppHandle, state: State<AppState>, id: String) {
    state.mark_cancel(&id, true);
    if let Some(t) = state.task(&id) {
        if t.status == TaskStatus::Queued {
            state.set_status(&id, TaskStatus::Cancelled);
            let st = state.inner().clone();
            download::emit_task(&app, &st, &id);
        }
    }
}

#[tauri::command]
fn retry_task(app: AppHandle, state: State<AppState>, id: String) {
    if let Some(t) = state.task(&id) {
        let st = state.inner().clone();
        download::enqueue(app, st, t.url, t.format.clone().unwrap_or_default(), t.out_path);
    }
}

#[tauri::command]
fn set_cookie_browser(state: State<AppState>, browser: Option<String>) {
    state.set_cookie_browser(browser.filter(|b| !b.is_empty()));
}

#[tauri::command]
fn test_engine() -> EngineInfo {
    download::test_engine()
}










#[tauri::command]
async fn open_capture(
    app: AppHandle,
    state: State<'_, AppState>,
    url: String,
    inject: Option<bool>,
    silent: Option<bool>,
) -> Result<String, String> {
    
    let app2 = app.clone();
    let st = state.inner().clone();
    let visible = !silent.unwrap_or(false);
    tauri::async_runtime::spawn_blocking(move || capture::open(&app2, &st, url, inject.unwrap_or(true), visible))
        .await
        .map_err(|e| format!("任务执行失败: {e}"))?
}


#[tauri::command]
fn close_capture(app: AppHandle) {
    if let Some(w) = app.get_webview_window("capture") {
        let _ = w.close();
    }
}


#[tauri::command]
fn open_path(path: String) -> Result<(), String> {
    let p = path.trim();
    if p.is_empty() {
        return Err("文件路径为空, 任务可能未完成".into());
    }
    if !std::path::Path::new(p).exists() {
        return Err(format!("文件不存在: {p}"));
    }
    #[cfg(target_os = "windows")]
    {
        
        
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        let quoted = p.replace('"', "");
        std::process::Command::new("cmd")
            .raw_arg(format!("/c start \"\" \"{quoted}\""))
            .creation_flags(CREATE_NO_WINDOW)
            .spawn()
            .map_err(|e| format!("打开失败: {e}"))?;
        Ok(())
    }
    #[cfg(not(target_os = "windows"))]
    {
        std::process::Command::new("xdg-open")
            .arg(p)
            .spawn()
            .map_err(|e| format!("打开失败: {e}"))?;
        Ok(())
    }
}


#[tauri::command]
fn reveal_path(path: String) -> Result<(), String> {
    let p = path.trim();
    if p.is_empty() {
        return Err("路径为空".into());
    }
    let path = std::path::Path::new(p);
    let (dir, file) = if path.is_file() {
        (
            path.parent().map(|d| d.to_path_buf()).unwrap_or_default(),
            true,
        )
    } else {
        (path.to_path_buf(), false)
    };
    if !dir.exists() {
        return Err(format!("目录不存在: {}", dir.display()));
    }
    #[cfg(target_os = "windows")]
    {
        
        
        
        use std::os::windows::process::CommandExt;
        let target = if file {
            format!("/select,\"{}\"", p.replace('"', ""))
        } else {
            format!("\"{}\"", dir.to_string_lossy().replace('"', ""))
        };
        std::process::Command::new("explorer")
            .raw_arg(target)
            .spawn()
            .map_err(|e| format!("打开文件夹失败: {e}"))?;
        Ok(())
    }
    #[cfg(not(target_os = "windows"))]
    {
        std::process::Command::new("xdg-open")
            .arg(&dir)
            .spawn()
            .map_err(|e| format!("打开文件夹失败: {e}"))?;
        Ok(())
    }
}



#[tauri::command]
fn download_pair(
    app: AppHandle,
    state: State<AppState>,
    a: String,
    b: String,
    opts: DownloadOptions,
) -> String {
    let st = state.inner().clone();
    download::enqueue_pair(app, st, a, b, opts.out_dir)
}


#[tauri::command]
fn delete_task(state: State<AppState>, id: String) {
    state.remove_task(&id);
}


#[tauri::command]
fn delete_tasks(state: State<AppState>, ids: Vec<String>) {
    state.remove_tasks(&ids);
}


#[tauri::command]
fn remove_media(state: State<AppState>, ids: Vec<String>) {
    state.remove_media(&ids);
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let state = AppState::new();
    tauri::Builder::default()
        .manage(state)
        .setup(|app| {
            
            let handle = app.handle().clone();
            let st = app.state::<AppState>().inner().clone();
            capture::register(&handle, st);
            
            if let Ok(url) = std::env::var("VD_CAPTURE_URL") {
                let h = app.handle().clone();
                let st2 = app.state::<AppState>().inner().clone();
                std::thread::spawn(move || {
                    std::thread::sleep(std::time::Duration::from_secs(3));
                    
                    let _ = capture::open(&h, &st2, url, true, true);
                });
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            health_check,
            proxy_status,
            start_proxy,
            stop_proxy,
            list_media,
            clear_media,
            resolve_page,
            download,
            download_pair,
            list_tasks,
            cancel_task,
            retry_task,
            set_cookie_browser,
            test_engine,
            open_capture,
            close_capture,
            open_path,
            reveal_path,
            delete_task,
            delete_tasks,
            remove_media
        ])
        .run(tauri::generate_context!())
        .expect("启动 Tauri 应用失败");
}
