import { defineStore } from "pinia";
import {
  exportManuscriptDocx,
  exportManuscriptMarkdown,
  type ExportFormat,
} from "../api/export";
import type { CommandError } from "../api/project";

function normalizeError(error: unknown) {
  if (typeof error === "object" && error && "message" in error) {
    return (error as CommandError).message;
  }

  if (typeof error === "string") {
    return error;
  }

  return "Der Export ist fehlgeschlagen.";
}

function ensureExtension(path: string, format: ExportFormat) {
  const extension = format === "markdown" ? ".md" : ".docx";
  return path.toLocaleLowerCase().endsWith(extension) ? path : `${path}${extension}`;
}

export const useExportStore = defineStore("export", {
  state: () => ({
    isExporting: false,
    lastExportPath: "",
    error: "",
  }),
  actions: {
    clear() {
      this.error = "";
    },
    setError(message: string) {
      this.error = message;
    },
    async exportManuscript(format: ExportFormat, path: string) {
      const exportPath = ensureExtension(path.trim(), format);
      if (!exportPath) {
        return;
      }

      this.isExporting = true;
      this.error = "";

      try {
        if (format === "markdown") {
          await exportManuscriptMarkdown(exportPath);
        } else {
          await exportManuscriptDocx(exportPath);
        }

        this.lastExportPath = exportPath;
      } catch (error) {
        this.error = normalizeError(error);
      } finally {
        this.isExporting = false;
      }
    },
  },
});
