import { invoke } from "@tauri-apps/api/core";
import { open, save } from "@tauri-apps/plugin-dialog";
import { requireTauriRuntime } from "./tauri";

const projectFilters = [
  {
    name: "Autorenprojekt",
    extensions: ["autorenprojekt"],
  },
];

export interface CreateProjectInput {
  name: string;
  description?: string;
  defaultLanguage: string;
  path: string;
}

export interface ProjectSnapshot {
  id: string;
  name: string;
  description: string;
  default_language: string;
  editor_font_family: string;
  editor_font_size: number;
  path: string;
  storage_kind: "file";
  created_at: string;
  updated_at: string;
}

export interface UpdateProjectInput {
  name: string;
  description: string;
  defaultLanguage: string;
  editorFontFamily: string;
  editorFontSize: number;
}

export interface RecentProject {
  name: string;
  path: string;
  storage_kind: "file";
  last_opened_at: string;
}

export interface CommandError {
  code: string;
  message: string;
}

export function createProject(input: CreateProjectInput): Promise<ProjectSnapshot> {
  requireTauriRuntime();
  return invoke<ProjectSnapshot>("create_project", { input });
}

export function openProject(path: string): Promise<ProjectSnapshot> {
  requireTauriRuntime();
  return invoke<ProjectSnapshot>("open_project", { path });
}

export function updateProject(input: UpdateProjectInput): Promise<ProjectSnapshot> {
  requireTauriRuntime();
  return invoke<ProjectSnapshot>("update_project", { input });
}

export function saveProjectCopyAs(path: string): Promise<string> {
  requireTauriRuntime();
  return invoke<string>("save_project_copy_as", { input: { path } });
}

export function listRecentProjects(): Promise<RecentProject[]> {
  requireTauriRuntime();
  return invoke<RecentProject[]>("list_recent_projects");
}

export function removeRecentProject(path: string): Promise<void> {
  requireTauriRuntime();
  return invoke<void>("remove_recent_project", { path });
}

export function chooseProjectSavePath(defaultName: string): Promise<string | null> {
  requireTauriRuntime();
  return save({
    defaultPath: defaultName,
    filters: projectFilters,
  });
}

export function chooseProjectBackupPath(defaultName: string): Promise<string | null> {
  requireTauriRuntime();
  return save({
    defaultPath: defaultName,
    filters: projectFilters,
  });
}

export async function chooseProjectOpenPath(): Promise<string | null> {
  requireTauriRuntime();
  const selected = await open({
    multiple: false,
    directory: false,
    filters: projectFilters,
  });

  return Array.isArray(selected) ? (selected[0] ?? null) : selected;
}
