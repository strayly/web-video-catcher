


export type MediaKind = "Mp4" | "M3u8" | "Ts" | "Dash" | "Audio" | "Other";


export type TaskStatus =
  | "Queued"
  | "Downloading"
  | "Merging"
  | "Done"
  | "Failed"
  | "Paused"
  | "Cancelled";


export interface MediaItem {
  id: string;
  url: string;
  title: string;
  source: string;
  kind: MediaKind;
  quality: string;
  size_bytes: number;
  status: string;
}


export interface DownloadTask {
  id: string;
  url: string;
  title: string;
  out_path: string;
  status: TaskStatus;
  progress: number;
  speed_bps: number;
  downloaded: number;
  total: number;
  error: string;
  
  format: string | null;
  
  file_path: string;
  
  pair_url?: string;
}


export interface DownloadOptions {
  format: string;
  quality: string;
  out_dir: string;
}


export interface EngineInfo {
  yt_dlp: boolean;
  ffmpeg: boolean;
  yt_dlp_version: string;
}


export interface ProxyStatus {
  running: boolean;
  addr: string;
  ca_installed: boolean;
}
