import { defineStore } from "pinia";
import { i18n } from "../i18n";
import { normalizeUiError, uiError, type UiError } from "../errors";
import {
  exportManuscriptDocx,
  exportManuscriptMarkdown,
  getExportProfile,
  updateExportProfile,
  type ExportProfile,
  type ExportFormat,
} from "../api/export";

function ensureExtension(path: string, format: ExportFormat) {
  const extension = format === "markdown" ? ".md" : ".docx";
  return path.toLocaleLowerCase().endsWith(extension) ? path : `${path}${extension}`;
}

export const useExportStore = defineStore("export", {
  state: () => ({
    profile: null as ExportProfile | null,
    isLoadingProfile: false,
    isSavingProfile: false,
    isExporting: false,
    lastExportPath: "",
    error: null as UiError | null,
  }),
  getters: {
    canExport(state) {
      return Boolean(
        state.profile &&
          Object.values(state.profile).some(Boolean) &&
          !state.isLoadingProfile &&
          !state.isSavingProfile &&
          !state.isExporting,
      );
    },
    profileSummary(state) {
      if (!state.profile) return i18n.global.t("exportProfile.loading");
      const labels: Array<[keyof ExportProfile, string]> = [
        ["includeNovelTitle", i18n.global.t("exportProfile.novelTitle")],
        ["includeAreaTitles", i18n.global.t("exportProfile.areaTitles")],
        ["includeChapterTitles", i18n.global.t("exportProfile.chapterTitles")],
        ["includeSceneTitles", i18n.global.t("exportProfile.sceneTitles")],
        ["includeSceneText", i18n.global.t("exportProfile.sceneText")],
      ];
      return labels.filter(([key]) => state.profile?.[key]).map(([, label]) => label).join(", ");
    },
  },
  actions: {
    reset() {
      this.profile = null;
      this.isLoadingProfile = false;
      this.isSavingProfile = false;
      this.lastExportPath = "";
      this.error = null;
    },
    async loadProfile() {
      this.isLoadingProfile = true;
      this.error = null;
      try {
        this.profile = await getExportProfile();
      } catch (error) {
        this.error = normalizeUiError(error);
      } finally {
        this.isLoadingProfile = false;
      }
    },
    async setProfileOption(key: keyof ExportProfile, value: boolean) {
      if (!this.profile || this.isSavingProfile) return;
      const next = { ...this.profile, [key]: value };
      if (!Object.values(next).some(Boolean)) {
        this.error = uiError("empty_export_profile");
        return;
      }
      this.isSavingProfile = true;
      this.error = null;
      try {
        this.profile = await updateExportProfile(next);
      } catch (error) {
        this.error = normalizeUiError(error);
      } finally {
        this.isSavingProfile = false;
      }
    },
    clear() {
      this.error = null;
    },
    setError(error: unknown, fallbackCode = "unexpected_error") {
      this.error = normalizeUiError(error, fallbackCode);
    },
    async exportManuscript(format: ExportFormat, path: string) {
      const exportPath = ensureExtension(path.trim(), format);
      if (!exportPath || !this.canExport) {
        return;
      }

      this.isExporting = true;
      this.error = null;

      try {
        if (format === "markdown") {
          await exportManuscriptMarkdown(exportPath);
        } else {
          await exportManuscriptDocx(exportPath);
        }

        this.lastExportPath = exportPath;
      } catch (error) {
        this.error = normalizeUiError(error);
      } finally {
        this.isExporting = false;
      }
    },
  },
});
