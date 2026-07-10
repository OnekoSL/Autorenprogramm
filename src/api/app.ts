import { invoke } from "@tauri-apps/api/core";
import { requireTauriRuntime } from "./tauri";

export interface AppInfo {
  name: string;
  version: string;
  environment: string;
}

export function getAppInfo(): Promise<AppInfo> {
  requireTauriRuntime();
  return invoke<AppInfo>("get_app_info");
}
