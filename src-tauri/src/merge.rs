





use std::process::{Command, Stdio};

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

#[cfg(windows)]
use std::os::windows::process::CommandExt;

fn ffmpeg() -> Command {
    let mut c = Command::new("ffmpeg");
    #[cfg(windows)]
    c.creation_flags(CREATE_NO_WINDOW);
    c
}

pub fn ffmpeg_available() -> bool {
    ffmpeg()
        .arg("-version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}


pub fn merge(input: &str, output: &str) -> std::io::Result<std::process::ExitStatus> {
    ffmpeg()
        .args(["-y", "-i", input, "-c", "copy", output])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
}








pub fn merge_av(
    video_in: &str,
    audio_in: &str,
    output: &str,
) -> Result<(), String> {
    if !ffmpeg_available() {
        return Err(crate::download::bi(
            "未检测到 ffmpeg, 无法合并音视频。请安装 ffmpeg 并加入 PATH 后重试; \
             或单独下载其中一条(视频无声 / 仅音频)。",
            "ffmpeg not found, cannot merge audio and video. Install ffmpeg and add it to PATH, then retry; \
             or download one of the streams separately (video without sound / audio only).",
        ));
    }
    let (v, a) = pick_av(video_in, audio_in);
    let out = ffmpeg()
        .args([
            "-y",
            "-i",
            v,
            "-i",
            a,
            "-map",
            "0:v:0",
            "-map",
            "1:a:0",
            "-c:v",
            "copy",
            "-c:a",
            "aac",
            "-b:a",
            "192k",
            "-movflags",
            "+faststart",
            "-shortest",
            output,
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .output()
        .map_err(|e| crate::download::bi(&format!("启动 ffmpeg 失败: {e}"), &format!("Failed to launch ffmpeg: {e}")))?;
    if out.status.success() {
        Ok(())
    } else {
        let tail = last_lines(&String::from_utf8_lossy(&out.stderr), 3);
        Err(crate::download::bi(
            &format!("ffmpeg 合并失败: {tail}"),
            &format!("ffmpeg merge failed: {tail}"),
        ))
    }
}




fn pick_av<'a>(a: &'a str, b: &'a str) -> (&'a str, &'a str) {
    let a_has_video = has_stream(a, "Video:");
    let b_has_video = has_stream(b, "Video:");
    if a_has_video && !b_has_video {
        (a, b)
    } else if b_has_video && !a_has_video {
        (b, a)
    } else {
        
        (a, b)
    }
}






fn has_stream(file: &str, kind: &str) -> bool {
    ffmpeg()
        .args(["-hide_banner", "-i", file])
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .output()
        .map(|o| {
            String::from_utf8_lossy(&o.stderr)
                .lines()
                .any(|l| l.trim_start().starts_with("Stream #") && l.contains(&format!(" {kind}")))
        })
        .unwrap_or(false)
}


fn last_lines(s: &str, n: usize) -> String {
    let lines: Vec<&str> = s.lines().map(|l| l.trim()).filter(|l| !l.is_empty()).collect();
    let start = lines.len().saturating_sub(n);
    lines[start..].join(" | ")
}
