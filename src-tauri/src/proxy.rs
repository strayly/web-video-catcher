





use tokio::io::AsyncWriteExt;
use tokio::net::{TcpListener, TcpStream};
use tauri::{AppHandle, Emitter};
use crate::schema::*;
use crate::sniffer::classify;
use crate::state::AppState;


pub async fn run_proxy(app: AppHandle, state: AppState, addr: String) {
    let listener = match TcpListener::bind(&addr).await {
        Ok(l) => l,
        Err(e) => {
            eprintln!("[proxy] 绑定 {addr} 失败: {e}");
            state.set_proxy_running(false);
            return;
        }
    };
    eprintln!("[proxy] 监听 {addr}");
    while state.proxy_running() {
        let (sock, _) = match listener.accept().await {
            Ok(x) => x,
            Err(_) => continue,
        };
        let app = app.clone();
        let st = state.clone();
        tauri::async_runtime::spawn(async move {
            if let Err(e) = handle_conn(sock, app, st).await {
                eprintln!("[proxy] 连接处理失败: {e}");
            }
        });
    }
    eprintln!("[proxy] 已停止");
}

async fn handle_conn(mut sock: TcpStream, app: AppHandle, state: AppState) -> std::io::Result<()> {
    let mut head = [0u8; 4096];
    let n = sock.peek(&mut head).await?;
    let text = String::from_utf8_lossy(&head[..n]).to_string();

    if text.to_uppercase().starts_with("CONNECT") {
        
        let host = text
            .lines()
            .next()
            .unwrap_or("")
            .trim_start_matches("CONNECT ")
            .split_whitespace()
            .next()
            .unwrap_or("")
            .to_string();
        let server = TcpStream::connect(&host).await?;
        sock.write_all(b"HTTP/1.1 200 Connection Established\r\n\r\n").await?;
        tunnel(sock, server).await;
        eprintln!("[proxy] HTTPS 连接: {host}");
    } else {
        
        if let Some(u) = http_request_url(&text) {
            if let Some((kind, quality)) = classify(&u, "") {
                let item = MediaItem {
                    id: format!("m_{}", now_ms()),
                    url: u.clone(),
                    title: host_of(&u),
                    source: host_of(&u),
                    kind,
                    quality,
                    size_bytes: -1,
                    status: "new".into(),
                };
                state.push_media(item.clone());
                let _ = app.emit("sniffer://new", &item);
            }
        }
        let host = http_host(&text).unwrap_or_else(|| "unknown:80".into());
        let server = TcpStream::connect(&host).await?;
        tunnel(sock, server).await;
    }
    Ok(())
}


async fn tunnel(mut a: TcpStream, mut b: TcpStream) {
    let (mut ar, mut aw) = a.split();
    let (mut br, mut bw) = b.split();
    let c2s = tokio::io::copy(&mut ar, &mut bw);
    let s2c = tokio::io::copy(&mut br, &mut aw);
    let _ = tokio::join!(c2s, s2c);
}


fn http_request_url(text: &str) -> Option<String> {
    let line = text.lines().next()?;
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() < 2 {
        return None;
    }
    let target = parts[1];
    if target.starts_with("http://") {
        Some(target.to_string())
    } else {
        None
    }
}


fn http_host(text: &str) -> Option<String> {
    for line in text.lines() {
        if let Some(rest) = line.to_ascii_lowercase().strip_prefix("host:") {
            let h = rest.trim().to_string();
            return Some(if h.contains(':') { h } else { format!("{h}:80") });
        }
    }
    None
}

fn host_of(u: &str) -> String {
    u.split("//")
        .nth(1)
        .map(|s| s.split('/').next().unwrap_or("").to_string())
        .unwrap_or_else(|| u.to_string())
}

fn now_ms() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0)
}
