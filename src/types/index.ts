export interface WindowInfo {
  hwnd: number;
  title: string;
  process_name: string | null;
  is_hidden: boolean;
}

export interface AppConfig {
  hotkey: string;
  language: string;
  last_selected_hwnd: number | null;
  theme: string;
  font_size: string;
  auto_start: boolean;
  silent_start: boolean;
  mute_on_hide: boolean;
  pause_on_hide: boolean;
  pause_hotkey: string;
  update_source: string;
  download_source: string;
  mirror_chan_sdk: string;
  auto_check_updates: boolean;
  mouse_side_button_listener: boolean;
  privacy_consent: boolean;
}

export interface ConfigUpdate {
  hotkey?: string | null;
  language?: string | null;
  last_selected_hwnd?: number | null;
  theme?: string | null;
  font_size?: string | null;
  auto_start?: boolean | null;
  silent_start?: boolean | null;
  mute_on_hide?: boolean | null;
  pause_on_hide?: boolean | null;
  pause_hotkey?: string | null;
  update_source?: string | null;
  download_source?: string | null;
  mirror_chan_sdk?: string | null;
  auto_check_updates?: boolean | null;
  mouse_side_button_listener?: boolean | null;
  privacy_consent?: boolean | null;
}

export interface LogEntry {
  level: string;
  message: string;
  timestamp: string;
}

export interface MemoryCleanupReport {
  scanned: number;
  cleaned: number;
  failed: number;
  reclaimed_bytes: number;
}

export interface MemoryStatusInfo {
  total_bytes: number;
  used_bytes: number;
  usage_percent: number;
}

export interface CacheCleanupOptions {
  system_cache: boolean;
  temp_files: boolean;
  thumbnail_cache: boolean;
  app_cache: boolean;
  recycle_bin: boolean;
}

export interface CacheCleanupReport {
  selected: number;
  cleaned: number;
  failed: number;
  reclaimed_bytes: number;
}

export interface UpdateCheckInfo {
  source: string;
  current_version: string;
  latest_version: string;
  changelog: string;
  has_update: boolean;
  download_url: string | null;
  download_candidates: string[];
  sha256: string | null;
  mirror_code: number | null;
  mirror_message: string | null;
}

export interface UpdateDownloadResult {
  file_path: string;
  sha256: string | null;
  used_url: string;
  fallback_used: boolean;
}

export interface MirrorDownloadInfo {
  url: string | null;
  sha256: string | null;
  mirror_code: number | null;
  mirror_message: string | null;
}

export interface MirrorCdkValidationInfo {
  valid: boolean;
  mirror_code: number | null;
  mirror_message: string | null;
}
