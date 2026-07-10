import { invoke } from "@tauri-apps/api/core";
import { save } from "@tauri-apps/plugin-dialog";
import { requireTauriRuntime } from "./tauri";

export type ExportFormat = "markdown" | "docx";

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
