import { defineStore } from "pinia";
import { normalizeUiError, type UiError } from "../errors";
import {
  applyStoryStructureTemplate,
  getProjectStructure,
  generateStoryStructureManuscript,
  listStoryStructureTemplates,
  updateStructureBeat,
  replaceStructureBeatSceneLinks,
  replaceStructureBeatCharacterLinks,
  type ProjectStructure,
  type StoryStructureTemplate,
  type StructureAct,
  type StructureBeat,
  type StructureBeatUpdate,
} from "../api/storyStructure";

export const useStoryStructureStore = defineStore("storyStructure", {
  state: () => ({
    templates: [] as StoryStructureTemplate[],
    activeStructure: null as ProjectStructure | null,
    selectedTemplateId: "",
    selectedBeatId: "",
    isLoading: false,
    isApplying: false,
    error: null as UiError | null,
  }),
  getters: {
    selectedTemplate(state): StoryStructureTemplate | null {
      return state.templates.find((template) => template.id === state.selectedTemplateId) ?? null;
    },
    displayedStructure(): StoryStructureTemplate | ProjectStructure | null {
      if (this.activeStructure?.templateId === this.selectedTemplateId) return this.activeStructure;
      return this.selectedTemplate ?? this.activeStructure;
    },
    displayedActs(): StructureAct[] {
      return this.displayedStructure?.acts ?? [];
    },
    selectedBeat(): StructureBeat | null {
      return this.displayedActs.flatMap((act) => act.beats).find((beat) => beat.id === this.selectedBeatId) ?? null;
    },
    selectedAct(): StructureAct | null {
      return this.displayedActs.find((act) => act.beats.some((beat) => beat.id === this.selectedBeatId)) ?? null;
    },
    hasGeneratedManuscript(): boolean {
      return Boolean(
        this.activeStructure?.acts.length &&
          this.activeStructure.acts.every(
            (act) =>
              act.manuscriptAreaId && act.beats.every((beat) => beat.manuscriptChapterId),
          ),
      );
    },
  },
  actions: {
    reset() {
      this.templates = [];
      this.activeStructure = null;
      this.selectedTemplateId = "";
      this.selectedBeatId = "";
      this.error = null;
    },
    selectTemplate(templateId: string) {
      this.selectedTemplateId = templateId;
      const source = this.activeStructure?.templateId === templateId
        ? this.activeStructure
        : this.templates.find((item) => item.id === templateId);
      this.selectedBeatId = source?.acts[0]?.beats[0]?.id ?? "";
    },
    selectBeat(beatId: string) {
      this.selectedBeatId = beatId;
    },
    async load() {
      this.isLoading = true;
      this.error = null;
      try {
        const [templates, activeStructure] = await Promise.all([
          listStoryStructureTemplates(),
          getProjectStructure(),
        ]);
        this.templates = templates;
        this.activeStructure = activeStructure;
        const preferredId = activeStructure?.templateId ?? templates[0]?.id ?? "";
        this.selectTemplate(preferredId);
      } catch (error) {
        this.error = normalizeUiError(error);
      } finally {
        this.isLoading = false;
      }
    },
    async applySelected(replaceExisting: boolean) {
      if (!this.selectedTemplateId) return false;
      this.isApplying = true;
      this.error = null;
      try {
        this.activeStructure = await applyStoryStructureTemplate(this.selectedTemplateId, replaceExisting);
        this.selectedBeatId = this.activeStructure.acts[0]?.beats[0]?.id ?? "";
        return true;
      } catch (error) {
        this.error = normalizeUiError(error);
        return false;
      } finally {
        this.isApplying = false;
      }
    },
    async generateManuscript() {
      if (!this.activeStructure) return false;
      this.isApplying = true;
      this.error = null;
      try {
        this.activeStructure = await generateStoryStructureManuscript();
        return true;
      } catch (error) {
        this.error = normalizeUiError(error);
        return false;
      } finally {
        this.isApplying = false;
      }
    },
    async updateBeat(input: StructureBeatUpdate) { this.isApplying = true; this.error = null; try { this.activeStructure = await updateStructureBeat(input); return true; } catch (error) { this.error = normalizeUiError(error); return false; } finally { this.isApplying = false; } },
    async replaceBeatSceneLinks(beatId: string, sceneIds: string[]) { this.isApplying = true; this.error = null; try { this.activeStructure = await replaceStructureBeatSceneLinks(beatId, sceneIds); return true; } catch (error) { this.error = normalizeUiError(error); return false; } finally { this.isApplying = false; } },
    async replaceBeatCharacterLinks(beatId: string, characterIds: string[]) { this.isApplying = true; this.error = null; try { this.activeStructure = await replaceStructureBeatCharacterLinks(beatId, characterIds); return true; } catch (error) { this.error = normalizeUiError(error); return false; } finally { this.isApplying = false; } },
  },
});
