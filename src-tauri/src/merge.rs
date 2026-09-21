





use std::process::{Command, Stdio};


pub fn ffmpeg_available() -> bool {
    Command::new("ffmpeg")
        .arg("-version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}


pub fn merge(input: &str, output: &str) -> std::io::Result<std::process::ExitStatus> {
    Command::new("ffmpeg")
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
        return Err(
            "未检测到 ffmpeg, 无法合并音视频。请安装 ffmpeg 并加入 PATH 后重试; \
             或单独下载其中一条(视频无声 / 仅音频)。"
                .into(),
        );
    }
    let (v, a) = pick_av(video_in, audio_in);
    let out = Command::new("ffmpeg")
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
        .map_err(|e| format!("启动 ffmpeg 失败: {e}"))?;
    if out.status.success() {
        Ok(())
    } else {
        let tail = last_lines(&String::from_utf8_lossy(&out.stderr), 3);
        Err(format!("ffmpeg 合并失败: {tail}"))
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
    Command::new("ffmpeg")
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
