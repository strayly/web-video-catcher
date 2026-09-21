









use crate::schema::MediaKind;



const TRACKER_HOSTS: [&str; 15] = [
    "data.bilibili.com",
    "dataflow.bilibili.com",
    "cm.bilibili.com",
    "google-analytics.com",
    "googletagmanager.com",
    "doubleclick.net",
    "hm.baidu.com",
    "cnzz.com",
    "umeng.com",
    "sensorsdata.cn",
    "talkingdata.com",
    
    "analytics.tiktok.com",
    "mon.tiktokv.com",
    "log.tiktokv.com",
    "mcs.zijieapi.com",
];


const TRACKER_PATHS: [&str; 6] = [
    "/log/",
    "/beacon",
    "/collect",
    "/report",
    "/app_log",
    "/monitor_browser",
];


const NON_MEDIA_CT: [&str; 10] = [
    "text/",
    "application/json",
    "application/javascript",
    "application/xml",
    "application/xhtml",
    "image/",
    "font/",
    "application/wasm",
    "application/pdf",
    "application/zip",
];


const AUDIO_EXTS: [&str; 7] = ["mp3", "m4a", "aac", "flac", "ogg", "opus", "wav"];
const VIDEO_EXTS: [&str; 9] = ["mp4", "m4v", "mov", "webm", "mkv", "flv", "f4v", "m4s", "3gp"];

const TS_EXTS: [&str; 2] = ["ts", "m2ts"];






const TRUSTED_MEDIA_CDNS: [&str; 8] = [
    "douyinvod.com",
    "bytecdntp.com",
    "zjcdn.com",
    "ixiguavideo.com",
    "ixigua.com",
    "tiktokcdn.com",
    "tiktokcdn-us.com",
    "tiktokcdn-eu.com",
];





const MEDIA_PATH_HINTS: [&str; 3] = ["/video/tos/", "/aweme/v1/play", "/upgcxcode/"];





pub fn classify(url: &str, content_type: &str) -> Option<(MediaKind, String)> {
    let low = url.trim().to_ascii_lowercase();
    if !(low.starts_with("http://") || low.starts_with("https://")) {
        return None;
    }
    let path = path_of(&low);
    if path.is_empty() {
        return None;
    }
    if is_tracking(host_slice(&low), path) {
        return None;
    }
    let ct = content_type.trim().to_ascii_lowercase();
    if NON_MEDIA_CT.iter().any(|x| ct.starts_with(*x)) {
        return None;
    }

    let ext = ext_of(path);
    
    if ext == "m3u8" || ct.contains("mpegurl") {
        return Some((MediaKind::M3u8, "未知".into()));
    }
    if ext == "mpd" || ct.contains("dash+xml") {
        return Some((MediaKind::Dash, "未知".into()));
    }
    
    if let Some(hit) = bili_dash(path) {
        return Some(hit);
    }
    
    
    if MEDIA_PATH_HINTS.iter().any(|h| path.contains(*h)) {
        return Some((MediaKind::Mp4, quality_from_path(path)));
    }
    if ext_in(&ext, &TS_EXTS) || ct.contains("mp2t") {
        return Some((MediaKind::Ts, quality_from_path(path)));
    }
    if ext_in(&ext, &AUDIO_EXTS) {
        return Some((MediaKind::Audio, quality_from_path(path)));
    }
    if ext_in(&ext, &VIDEO_EXTS) {
        return Some((MediaKind::Mp4, quality_from_path(path)));
    }
    
    
    if ct.starts_with("audio/") {
        return Some((MediaKind::Audio, "未知".into()));
    }
    if ct.starts_with("video/") {
        let kind = if ct.contains("mp4") { MediaKind::Mp4 } else { MediaKind::Other };
        return Some((kind, "未知".into()));
    }
    if ct.contains("octet-stream") {
        return Some((MediaKind::Other, "未知".into()));
    }
    if ext.is_empty() && is_trusted_media_cdn(host_slice(&low)) {
        return Some((MediaKind::Other, "未知".into()));
    }
    None
}


fn path_of(url: &str) -> &str {
    let rest = url.split("://").nth(1).unwrap_or("");
    let no_query = rest.split(['?', '#']).next().unwrap_or("");
    match no_query.find('/') {
        Some(i) => &no_query[i..],
        None => "",
    }
}


fn host_slice(url: &str) -> &str {
    let rest = url.split("://").nth(1).unwrap_or(url);
    let end = rest.find(['/', '?', '#']).unwrap_or(rest.len());
    rest[..end].split(':').next().unwrap_or("")
}


fn ext_of(path: &str) -> String {
    path.rsplit('/')
        .next()
        .unwrap_or("")
        .rsplit_once('.')
        .map(|(_, e)| e.to_string())
        .unwrap_or_default()
}


fn ext_in(ext: &str, list: &[&str]) -> bool {
    !ext.is_empty() && list.iter().any(|e| *e == ext)
}


fn is_tracking(host: &str, path: &str) -> bool {
    TRACKER_HOSTS
        .iter()
        .any(|h| host == *h || host.ends_with(&format!(".{h}")))
        || TRACKER_PATHS.iter().any(|p| path.contains(*p))
}


fn is_trusted_media_cdn(host: &str) -> bool {
    TRUSTED_MEDIA_CDNS
        .iter()
        .any(|d| host == *d || host.ends_with(&format!(".{d}")))
}








fn bili_dash(path: &str) -> Option<(MediaKind, String)> {
    let seg = path.rsplit('/').next().unwrap_or("");
    let stem = match seg.rsplit_once('.') {
        Some((s, _)) => s,
        None => seg,
    };
    let id = stem.rsplit('-').next().unwrap_or("");
    if id.len() != 5 || !id.starts_with("30") {
        return None;
    }
    let n: u32 = id.parse().ok()?;
    if n >= 30200 {
        let bitrate = match n {
            30216 => "64K",
            30232 => "132K",
            30280 => "192K",
            30250 => "杜比全景声",
            30251 => "Hi-Res",
            30282 => "杜比全景声",
            _ => "未知",
        };
        return Some((MediaKind::Audio, bitrate.into()));
    }
    let res = match n {
        30006 => "240P",
        30011 | 30016 => "360P",
        30032 | 30033 => "480P",
        30064 => "720P",
        30074 => "720P60",
        30077 | 30080 => "1080P",
        30112 => "1080P+",
        30116 => "1080P60",
        30120 => "4K",
        30125 => "HDR",
        30126 => "杜比视界",
        30127 => "8K",
        _ => "未知",
    };
    Some((MediaKind::Mp4, res.into()))
}



fn quality_from_path(path: &str) -> String {
    for (token, label) in [
        ("2160p", "2160P"),
        ("1440p", "1440P"),
        ("1080p", "1080P"),
        ("720p", "720P"),
        ("480p", "480P"),
        ("360p", "360P"),
    ] {
        if path.contains(token) {
            return label.into();
        }
    }
    if path.contains("4k") {
        return "4K".into();
    }
    "未知".into()
}


pub fn host_of(u: &str) -> String {
    u.split("//")
        .nth(1)
        .map(|s| s.split('/').next().unwrap_or("").to_string())
        .unwrap_or_else(|| u.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    
    #[test]
    fn tracking_requests_are_not_media() {
        
        let log = "https://data.bilibili.com/log/web?0000171https%3A%2F%2Fwww.bilibili.com%2Fvideo%2FBV1GCen6PE7G&bw=1440";
        assert_eq!(classify(log, "text/plain; charset=utf-8"), None);
        
        assert_eq!(
            classify("https://api.example.com/track?u=/a/b.mp4", "application/json"),
            None
        );
        
        assert_eq!(classify("https://example.com/pixel.gif?src=x.mp4", "image/gif"), None);
    }

    
    #[test]
    fn bilibili_dash_video_and_audio_are_told_apart() {
        let video =
            "https://upos-sz-estgcos.bilivideo.com/upgcxcode/1/2/114514/114514-1-30080.m4s?upsig=abc";
        assert_eq!(classify(video, "video/mp4"), Some((MediaKind::Mp4, "1080P".into())));
        let audio =
            "https://upos-sz-estgoss.bilivideo.com/upgcxcode/1/2/114514/114514-1-30280.m4s?upsig=abc";
        assert_eq!(classify(audio, "audio/mp4"), Some((MediaKind::Audio, "192K".into())));
        
        assert_eq!(
            classify("https://x.bilivideo.com/upgcxcode/1/2/1/1-1-30120.m4s", ""),
            Some((MediaKind::Mp4, "4K".into()))
        );
        assert_eq!(
            classify("https://x.bilivideo.com/upgcxcode/1/2/1/1-1-30032.m4s", ""),
            Some((MediaKind::Mp4, "480P".into()))
        );
    }

    
    #[test]
    fn resolution_comes_from_path_and_needs_the_p() {
        assert_eq!(
            classify("https://x.com/a/video_1080p.mp4", "video/mp4"),
            Some((MediaKind::Mp4, "1080P".into()))
        );
        assert_eq!(
            classify("https://x.com/a/v1440x9.mp4", ""),
            Some((MediaKind::Mp4, "未知".into()))
        );
    }

    
    #[test]
    fn direct_link_without_extension() {
        let d = "https://v3-web.douyinvod.com/2cbd/abc/video/tos/cn/oQxyz/?a=6383&ch=5";
        assert_eq!(classify(d, "video/mp4"), Some((MediaKind::Mp4, "未知".into())));
        
        assert_eq!(classify(d, ""), Some((MediaKind::Mp4, "未知".into())));
        
        assert_eq!(
            classify("https://v3-web.douyinvod.com/2cbd/abc/oQxyz/", ""),
            Some((MediaKind::Other, "未知".into()))
        );
        
        assert_eq!(classify("https://example.com/x/y/?a=1", ""), None);
    }

    
    #[test]
    fn manifests_keep_their_kind() {
        assert_eq!(
            classify("https://x.com/a/index.m3u8?t=1", ""),
            Some((MediaKind::M3u8, "未知".into()))
        );
        assert_eq!(
            classify("https://x.com/a/manifest.mpd", ""),
            Some((MediaKind::Dash, "未知".into()))
        );
    }

    
    
    #[test]
    fn tiktok_direct_link_is_recognized() {
        let v = "https://v16-webapp-prime.tiktok.com/video/tos/useast5/tos-useast5-ve-0068c001-eo/oQABCDEF/?a=1988&bti=ODcuMzY%3D&ch=0&br=1018&mime_type=video_mp4";
        
        assert_eq!(classify(v, ""), Some((MediaKind::Mp4, "未知".into())));
        assert_eq!(
            classify(v, "application/octet-stream"),
            Some((MediaKind::Mp4, "未知".into()))
        );
        
        assert_eq!(
            classify("https://p16-sign-va.tiktokcdn.com/obj/tos-maliva-p-0068/abcd/", ""),
            Some((MediaKind::Other, "未知".into()))
        );
    }

    
    
    #[test]
    fn tiktok_api_and_telemetry_are_not_media() {
        
        assert_eq!(
            classify(
                "https://www.tiktok.com/api/item/detail/?aid=1988&itemId=7684935383796157716",
                "application/json"
            ),
            None
        );
        
        assert_eq!(
            classify(
                "https://mon.tiktokv.com/monitor_browser/collect/batch/?biz_id=tiktok_web",
                "application/json"
            ),
            None
        );
        assert_eq!(
            classify("https://analytics.tiktok.com/api/v2/pixel/log/", "text/plain"),
            None
        );
    }
}
