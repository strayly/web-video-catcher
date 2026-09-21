
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use crate::schema::*;

struct Inner {
    media: Mutex<(Vec<MediaItem>, HashSet<String>)>,
    tasks: Mutex<HashMap<String, DownloadTask>>,
    proxy_running: AtomicBool,
    proxy_addr: String,
    cookie_browser: Mutex<Option<String>>,
    cancel: Mutex<HashMap<String, bool>>,
    
    capture_page: Mutex<Option<String>>,
    
    capture_title: Mutex<String>,
    
    referer_by_url: Mutex<HashMap<String, String>>,
}


#[derive(Clone)]
pub struct AppState(Arc<Inner>);

impl AppState {
    
    pub fn new() -> Self {
        AppState(Arc::new(Inner {
            media: Mutex::new((Vec::new(), HashSet::new())),
            tasks: Mutex::new(HashMap::new()),
            proxy_running: AtomicBool::new(false),
            proxy_addr: "127.0.0.1:8888".into(),
            cookie_browser: Mutex::new(None),
            cancel: Mutex::new(HashMap::new()),
            capture_page: Mutex::new(None),
            capture_title: Mutex::new(String::new()),
            referer_by_url: Mutex::new(HashMap::new()),
        }))
    }

    pub fn proxy_addr(&self) -> String {
        self.0.proxy_addr.clone()
    }

    pub fn set_proxy_running(&self, v: bool) {
        self.0.proxy_running.store(v, Ordering::SeqCst);
    }

    pub fn proxy_running(&self) -> bool {
        self.0.proxy_running.load(Ordering::SeqCst)
    }

    
    
    fn media_key(url: &str, kind: MediaKind) -> String {
        let base = url
            .split(['?', '#'])
            .next()
            .unwrap_or(url)
            .trim_end_matches('/')
            .to_string();
        format!("{:?}|{}", kind, base)
    }

    
    pub fn push_media(&self, m: MediaItem) -> bool {
        let key = Self::media_key(&m.url, m.kind);
        let mut g = self.0.media.lock().unwrap();
        if g.1.contains(&key) {
            return false;
        }
        
        const MAX: usize = 800;
        if g.0.len() >= MAX {
            if let Some(old) = g.0.first() {
                let old_key = Self::media_key(&old.url, old.kind);
                g.1.remove(&old_key);
            }
            g.0.remove(0);
        }
        g.1.insert(key);
        g.0.push(m);
        true
    }

    pub fn media(&self) -> Vec<MediaItem> {
        self.0.media.lock().unwrap().0.clone()
    }

    pub fn clear_media(&self) {
        *self.0.media.lock().unwrap() = (Vec::new(), HashSet::new());
    }

    
    pub fn remove_media(&self, ids: &[String]) {
        let set: HashSet<&String> = ids.iter().collect();
        let mut g = self.0.media.lock().unwrap();
        g.0.retain(|m| !set.contains(&m.id));
        
        let keys: HashSet<String> = g.0.iter().map(|m| Self::media_key(&m.url, m.kind)).collect();
        g.1 = keys;
    }

    pub fn tasks(&self) -> HashMap<String, DownloadTask> {
        self.0.tasks.lock().unwrap().clone()
    }

    pub fn task(&self, id: &str) -> Option<DownloadTask> {
        self.0.tasks.lock().unwrap().get(id).cloned()
    }

    pub fn upsert_task(&self, t: DownloadTask) {
        self.0.tasks.lock().unwrap().insert(t.id.clone(), t);
    }

    
    pub fn remove_task(&self, id: &str) {
        self.0.tasks.lock().unwrap().remove(id);
    }

    
    pub fn remove_tasks(&self, ids: &[String]) {
        let set: HashSet<&String> = ids.iter().collect();
        self.0.tasks.lock().unwrap().retain(|id, _| !set.contains(id));
    }

    pub fn set_status(&self, id: &str, s: TaskStatus) {
        if let Some(t) = self.0.tasks.lock().unwrap().get_mut(id) {
            t.status = s;
        }
    }

    pub fn set_error(&self, id: &str, e: String) {
        if let Some(t) = self.0.tasks.lock().unwrap().get_mut(id) {
            t.error = e;
        }
    }

    
    pub fn set_file_path(&self, id: &str, p: String) {
        if let Some(t) = self.0.tasks.lock().unwrap().get_mut(id) {
            t.file_path = p;
        }
    }

    
    pub fn set_title(&self, id: &str, title: String) {
        if let Some(t) = self.0.tasks.lock().unwrap().get_mut(id) {
            t.title = title;
        }
    }

    pub fn status_of(&self, id: &str) -> TaskStatus {
        self.task(id).map(|t| t.status).unwrap_or(TaskStatus::Failed)
    }

    pub fn mark_cancel(&self, id: &str, v: bool) {
        self.0.cancel.lock().unwrap().insert(id.to_string(), v);
    }

    pub fn is_cancelled(&self, id: &str) -> bool {
        self.0.cancel.lock().unwrap().get(id).copied().unwrap_or(false)
    }

    pub fn cookie_browser(&self) -> Option<String> {
        self.0.cookie_browser.lock().unwrap().clone()
    }

    pub fn set_cookie_browser(&self, b: Option<String>) {
        *self.0.cookie_browser.lock().unwrap() = b;
    }

    
    pub fn set_capture_page(&self, url: String) {
        *self.0.capture_page.lock().unwrap() = Some(url);
        *self.0.capture_title.lock().unwrap() = String::new();
    }

    
    pub fn capture_page_origin(&self) -> Option<String> {
        let g = self.0.capture_page.lock().unwrap();
        origin_of(g.as_deref()?)
    }

    pub fn set_capture_title(&self, t: String) {
        *self.0.capture_title.lock().unwrap() = t;
    }

    pub fn capture_title(&self) -> String {
        self.0.capture_title.lock().unwrap().clone()
    }

    
    pub fn set_referer(&self, url: &str, referer: String) {
        self.0.referer_by_url.lock().unwrap().insert(url.to_string(), referer);
    }

    
    pub fn referer_of(&self, url: &str) -> Option<String> {
        self.0.referer_by_url.lock().unwrap().get(url).cloned()
    }

    
    
    
    
    pub fn set_media_size(&self, id: &str, size: i64) -> Option<MediaItem> {
        let mut g = self.0.media.lock().unwrap();
        let item = g.0.iter_mut().find(|m| m.id == id)?;
        item.size_bytes = size;
        Some(item.clone())
    }

    
    pub fn capture_page_url(&self) -> Option<String> {
        self.0.capture_page.lock().unwrap().clone()
    }
}


fn origin_of(u: &str) -> Option<String> {
    let (scheme, rest) = u.split_once("://")?;
    let host = rest.split(['/', '?', '#']).next()?;
    if scheme.is_empty() || host.is_empty() {
        None
    } else {
        Some(format!("{scheme}://{host}/"))
    }
}
