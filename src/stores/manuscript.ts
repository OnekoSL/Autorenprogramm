import { defineStore } from "pinia";
import { i18n } from "../i18n";
import { normalizeUiError, type UiError } from "../errors";
import {
  createArea as createAreaCommand,
  createChapter as createChapterCommand,
  createScene as createSceneCommand,
  deleteArea as deleteAreaCommand,
  deleteChapter as deleteChapterCommand,
  deleteScene as deleteSceneCommand,
  getManuscriptTree,
  moveArea as moveAreaCommand,
  moveChapter as moveChapterCommand,
  moveScene as moveSceneCommand,
  updateArea as updateAreaCommand,
  updateChapter as updateChapterCommand,
  updateScene as updateSceneCommand,
  type AreaSnapshot,
  type ChapterSnapshot,
  type ManuscriptTree,
  type SceneSnapshot,
  type UpdateSceneInput,
} from "../api/manuscript";

function allChapters(tree: ManuscriptTree | null) {
  return tree?.areas.flatMap((area) => area.chapters) ?? [];
}

function allScenes(tree: ManuscriptTree | null) {
  return allChapters(tree).flatMap((chapter) => chapter.scenes);
}

function areaIds(tree: ManuscriptTree | null) {
  return new Set(tree?.areas.map((area) => area.id) ?? []);
}

function chapterIds(tree: ManuscriptTree | null) {
  return new Set(allChapters(tree).map((chapter) => chapter.id));
}

function sceneIds(tree: ManuscriptTree | null) {
  return new Set(allScenes(tree).map((scene) => scene.id));
}

function firstSceneId(tree: ManuscriptTree | null) {
  return tree?.areas
    .flatMap((area) => area.chapters)
    .find((chapter) => chapter.scenes.length > 0)?.scenes[0]?.id ?? "";
}

function nextSceneAfterDelete(
  beforeTree: ManuscriptTree | null,
  afterTree: ManuscriptTree,
  deletedSceneIds: Set<string>,
  currentSceneId: string,
) {
  if (!deletedSceneIds.has(currentSceneId)) {
    return currentSceneId;
  }

  const before = allScenes(beforeTree).map((scene) => scene.id);
  const after = allScenes(afterTree).map((scene) => scene.id);
  const deletedIndex = before.findIndex((sceneId) => deletedSceneIds.has(sceneId));

  if (deletedIndex < 0) {
    return firstSceneId(afterTree);
  }

  return after[deletedIndex] ?? after[deletedIndex - 1] ?? firstSceneId(afterTree);
}

export const useManuscriptStore = defineStore("manuscript", {
  state: () => ({
    tree: null as ManuscriptTree | null,
    activeSceneId: "",
    isLoading: false,
    isSaving: false,
    error: null as UiError | null,
  }),
  getters: {
    areas(state): AreaSnapshot[] {
      return state.tree?.areas ?? [];
    },
    chapters(state): ChapterSnapshot[] {
      return allChapters(state.tree);
    },
    scenes(state): SceneSnapshot[] {
      return allScenes(state.tree);
    },
    activeScene(state): SceneSnapshot | null {
      return allScenes(state.tree).find((scene) => scene.id === state.activeSceneId) ?? null;
    },
    activeChapter(state): ChapterSnapshot | null {
      return (
        allChapters(state.tree).find((chapter) =>
          chapter.scenes.some((scene) => scene.id === state.activeSceneId),
        ) ?? allChapters(state.tree)[0] ?? null
      );
    },
    activeArea(state): AreaSnapshot | null {
      const activeChapter = allChapters(state.tree).find((chapter) =>
        chapter.scenes.some((scene) => scene.id === state.activeSceneId),
      );

      return (
        state.tree?.areas.find((area) => area.id === activeChapter?.areaId) ??
        state.tree?.areas[0] ??
        null
      );
    },
    wordCount(): number {
      const text = this.activeScene?.text.trim() ?? "";
      return text ? text.split(/\s+/).length : 0;
    },
    activeLocation(): string {
      for (const [areaIndex, area] of this.areas.entries()) {
        for (const [chapterIndex, chapter] of area.chapters.entries()) {
          const sceneIndex = chapter.scenes.findIndex((scene) => scene.id === this.activeSceneId);
          if (sceneIndex >= 0) {
            return i18n.global.t("writing.location", {
              area: areaIndex + 1,
              chapter: chapterIndex + 1,
              scene: sceneIndex + 1,
            });
          }
        }
      }

      return i18n.global.t("writing.noScene");
    },
  },
  actions: {
    reset() {
      this.tree = null;
      this.activeSceneId = "";
      this.error = null;
    },
    setTree(tree: ManuscriptTree, preferredSceneId?: string) {
      this.tree = tree;
      const selectedSceneId = preferredSceneId ?? this.activeSceneId;
      const hasPreferredScene = allScenes(tree).some((scene) => scene.id === selectedSceneId);
      this.activeSceneId = hasPreferredScene ? selectedSceneId : firstSceneId(tree);
    },
    selectScene(sceneId: string) {
      this.activeSceneId = sceneId;
    },
    async loadTree() {
      this.isLoading = true;
      this.error = null;

      try {
        this.setTree(await getManuscriptTree());
      } catch (error) {
        this.error = normalizeUiError(error);
      } finally {
        this.isLoading = false;
      }
    },
    async createArea() {
      const before = areaIds(this.tree);
      const activeAreaId = this.activeArea?.id;
      this.error = null;

      try {
        const tree = await createAreaCommand({
          title: i18n.global.t("writing.newArea"),
          afterAreaId: activeAreaId,
        });
        const newArea = tree.areas.find((area) => !before.has(area.id));
        this.setTree(tree, newArea?.chapters[0]?.scenes[0]?.id ?? this.activeSceneId);
      } catch (error) {
        this.error = normalizeUiError(error);
      }
    },
    async createChapter(areaId?: string) {
      const before = chapterIds(this.tree);
      const targetAreaId = areaId || this.activeArea?.id || this.areas[0]?.id || "";
      const activeChapterId =
        this.activeChapter?.areaId === targetAreaId ? this.activeChapter?.id : undefined;
      this.error = null;

      try {
        const tree = await createChapterCommand({
          areaId: targetAreaId,
          title: i18n.global.t("writing.newChapter"),
          afterChapterId: activeChapterId,
        });
        const newChapter = allChapters(tree).find((chapter) => !before.has(chapter.id));
        this.setTree(tree, newChapter?.scenes[0]?.id ?? this.activeSceneId);
      } catch (error) {
        this.error = normalizeUiError(error);
      }
    },
    async createScene(chapterId?: string) {
      const before = sceneIds(this.tree);
      const activeSceneId = this.activeSceneId;
      const targetChapterId = chapterId || this.activeChapter?.id || this.chapters[0]?.id || "";
      this.error = null;

      try {
        const tree = await createSceneCommand({
          chapterId: targetChapterId,
          title: i18n.global.t("writing.newScene"),
          afterSceneId: activeSceneId || undefined,
        });
        const newScene = allScenes(tree).find((scene) => !before.has(scene.id));
        this.setTree(tree, newScene?.id ?? this.activeSceneId);
      } catch (error) {
        this.error = normalizeUiError(error);
      }
    },
    async updateArea(area: AreaSnapshot) {
      this.error = null;

      try {
        this.setTree(
          await updateAreaCommand({
            id: area.id,
            title: area.title,
            summary: area.summary,
          }),
        );
      } catch (error) {
        this.error = normalizeUiError(error);
      }
    },
    async updateChapter(chapter: ChapterSnapshot) {
      this.error = null;

      try {
        this.setTree(
          await updateChapterCommand({
            id: chapter.id,
            title: chapter.title,
            summary: chapter.summary,
          }),
        );
      } catch (error) {
        this.error = normalizeUiError(error);
      }
    },
    async updateScene(input: UpdateSceneInput) {
      this.isSaving = true;
      this.error = null;

      try {
        this.setTree(await updateSceneCommand(input), input.id);
        return true;
      } catch (error) {
        this.error = normalizeUiError(error);
        return false;
      } finally {
        this.isSaving = false;
      }
    },
    async moveArea(areaId: string, targetIndex: number) {
      this.error = null;

      try {
        this.setTree(await moveAreaCommand({ areaId, targetIndex }));
      } catch (error) {
        this.error = normalizeUiError(error);
      }
    },
    async moveChapter(chapterId: string, targetAreaId: string, targetIndex: number) {
      this.error = null;

      try {
        this.setTree(await moveChapterCommand({ chapterId, targetAreaId, targetIndex }));
      } catch (error) {
        this.error = normalizeUiError(error);
      }
    },
    async moveScene(sceneId: string, targetChapterId: string, targetIndex: number) {
      this.error = null;

      try {
        this.setTree(await moveSceneCommand({ sceneId, targetChapterId, targetIndex }), sceneId);
      } catch (error) {
        this.error = normalizeUiError(error);
      }
    },
    async deleteArea(areaId: string) {
      const deletedSceneIds = new Set(
        this.tree?.areas
          .find((area) => area.id === areaId)
          ?.chapters.flatMap((chapter) => chapter.scenes.map((scene) => scene.id)) ?? [],
      );
      const beforeTree = this.tree;
      this.error = null;

      try {
        const tree = await deleteAreaCommand({ id: areaId });
        this.setTree(
          tree,
          nextSceneAfterDelete(beforeTree, tree, deletedSceneIds, this.activeSceneId),
        );
      } catch (error) {
        this.error = normalizeUiError(error);
      }
    },
    async deleteChapter(chapterId: string) {
      const deletedSceneIds = new Set(
        allChapters(this.tree)
          .find((chapter) => chapter.id === chapterId)
          ?.scenes.map((scene) => scene.id) ?? [],
      );
      const beforeTree = this.tree;
      this.error = null;

      try {
        const tree = await deleteChapterCommand({ id: chapterId });
        this.setTree(
          tree,
          nextSceneAfterDelete(beforeTree, tree, deletedSceneIds, this.activeSceneId),
        );
      } catch (error) {
        this.error = normalizeUiError(error);
      }
    },
    async deleteScene(sceneId: string) {
      const beforeTree = this.tree;
      this.error = null;

      try {
        const tree = await deleteSceneCommand({ id: sceneId });
        this.setTree(
          tree,
          nextSceneAfterDelete(beforeTree, tree, new Set([sceneId]), this.activeSceneId),
        );
      } catch (error) {
        this.error = normalizeUiError(error);
      }
    },
  },
});
