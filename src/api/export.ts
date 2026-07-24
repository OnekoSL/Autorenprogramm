import { invoke } from "@tauri-apps/api/core";
import { save } from "@tauri-apps/plugin-dialog";
import { requireTauriRuntime } from "./tauri";

export type ExportFormat = "markdown" | "docx";

export interface ExportProfile {
  includeNovelTitle: boolean;
  includeAreaTitles: boolean;
  includeChapterTitles: boolean;
  includeSceneTitles: boolean;
  includeSceneText: boolean;
}

const exportFilters: Record<ExportFormat, Array<{ name: string; extensions: string[] }>> = {
  markdown: [{ name: "Markdown", extensions: ["md"] }],
  docx: [{ name: "DOCX", extensions: ["docx"] }],
};

export function chooseExportPath(format: ExportFormat, defaultName: string): Promise<string | null> {
  requireTauriRuntime();
  return save({
    defaultPath: defaultName,
    filters: exportFilters[format],
  });
}

export function exportManuscriptMarkdown(path: string): Promise<void> {
  requireTauriRuntime();
  return invoke<void>("export_manuscript_markdown", { input: { path } });
}

export function exportManuscriptDocx(path: string): Promise<void> {
  requireTauriRuntime();
  return invoke<void>("export_manuscript_docx", { input: { path } });
}

export function getExportProfile(): Promise<ExportProfile> {
  requireTauriRuntime();
  return invoke("get_export_profile");
}

export function updateExportProfile(profile: ExportProfile): Promise<ExportProfile> {
  requireTauriRuntime();
  return invoke("update_export_profile", { input: profile });
}
