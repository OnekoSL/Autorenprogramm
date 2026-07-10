import { defineStore } from "pinia";
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
import type { CommandError } from "../api/project";

function normalizeError(error: unknown) {
  if (typeof error === "object" && error && "message" in error) {
    return (error as CommandError).message;
  }

  if (typeof error === "string") {
    return error;
  }

  return "Die Manuskriptaktion ist fehlgeschlagen.";
}

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
    error: "",
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
            return `Bereich ${areaIndex + 1} / Kapitel ${chapterIndex + 1} / Szene ${
              sceneIndex + 1
            }`;
          }
        }
      }

      return "Keine Szene";
    },
  },
  actions: {
    reset() {
      this.tree = null;
      this.activeSceneId = "";
      this.error = "";
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
      this.error = "";

      try {
        this.setTree(await getManuscriptTree());
      } catch (error) {
        this.error = normalizeError(error);
      } finally {
        this.isLoading = false;
      }
    },
    async createArea() {
      const before = areaIds(this.tree);
      const activeAreaId = this.activeArea?.id;
      this.error = "";

      try {
        const tree = await createAreaCommand({
          title: "Neuer Bereich",
          afterAreaId: activeAreaId,
        });
        const newArea = tree.areas.find((area) => !before.has(area.id));
        this.setTree(tree, newArea?.chapters[0]?.scenes[0]?.id ?? this.activeSceneId);
      } catch (error) {
        this.error = normalizeError(error);
      }
    },
    async createChapter(areaId?: string) {
      const before = chapterIds(this.tree);
      const targetAreaId = areaId || this.activeArea?.id || this.areas[0]?.id || "";
      const activeChapterId =
        this.activeChapter?.areaId === targetAreaId ? this.activeChapter?.id : undefined;
      this.error = "";

      try {
        const tree = await createChapterCommand({
          areaId: targetAreaId,
          title: "Neues Kapitel",
          afterChapterId: activeChapterId,
        });
        const newChapter = allChapters(tree).find((chapter) => !before.has(chapter.id));
        this.setTree(tree, newChapter?.scenes[0]?.id ?? this.activeSceneId);
      } catch (error) {
        this.error = normalizeError(error);
      }
    },
    async createScene(chapterId?: string) {
      const before = sceneIds(this.tree);
      const activeSceneId = this.activeSceneId;
      const targetChapterId = chapterId || this.activeChapter?.id || this.chapters[0]?.id || "";
      this.error = "";

      try {
        const tree = await createSceneCommand({
          chapterId: targetChapterId,
          title: "Neue Szene",
          afterSceneId: activeSceneId || undefined,
        });
        const newScene = allScenes(tree).find((scene) => !before.has(scene.id));
        this.setTree(tree, newScene?.id ?? this.activeSceneId);
      } catch (error) {
        this.error = normalizeError(error);
      }
    },
    async updateArea(area: AreaSnapshot) {
      this.error = "";

      try {
        this.setTree(
          await updateAreaCommand({
            id: area.id,
            title: area.title,
            summary: area.summary,
          }),
        );
      } catch (error) {
        this.error = normalizeError(error);
      }
    },
    async updateChapter(chapter: ChapterSnapshot) {
      this.error = "";

      try {
        this.setTree(
          await updateChapterCommand({
            id: chapter.id,
            title: chapter.title,
            summary: chapter.summary,
          }),
        );
      } catch (error) {
        this.error = normalizeError(error);
      }
    },
    async updateScene(input: UpdateSceneInput) {
      this.isSaving = true;
      this.error = "";

      try {
        this.setTree(await updateSceneCommand(input), input.id);
      } catch (error) {
        this.error = normalizeError(error);
      } finally {
        this.isSaving = false;
      }
    },
    async moveArea(areaId: string, targetIndex: number) {
      this.error = "";

      try {
        this.setTree(await moveAreaCommand({ areaId, targetIndex }));
      } catch (error) {
        this.error = normalizeError(error);
      }
    },
    async moveChapter(chapterId: string, targetAreaId: string, targetIndex: number) {
      this.error = "";

      try {
        this.setTree(await moveChapterCommand({ chapterId, targetAreaId, targetIndex }));
      } catch (error) {
        this.error = normalizeError(error);
      }
    },
    async moveScene(sceneId: string, targetChapterId: string, targetIndex: number) {
      this.error = "";

      try {
        this.setTree(await moveSceneCommand({ sceneId, targetChapterId, targetIndex }), sceneId);
      } catch (error) {
        this.error = normalizeError(error);
      }
    },
    async deleteArea(areaId: string) {
      const deletedSceneIds = new Set(
        this.tree?.areas
          .find((area) => area.id === areaId)
          ?.chapters.flatMap((chapter) => chapter.scenes.map((scene) => scene.id)) ?? [],
      );
      const beforeTree = this.tree;
      this.error = "";

      try {
        const tree = await deleteAreaCommand({ id: areaId });
        this.setTree(
          tree,
          nextSceneAfterDelete(beforeTree, tree, deletedSceneIds, this.activeSceneId),
        );
      } catch (error) {
        this.error = normalizeError(error);
      }
    },
    async deleteChapter(chapterId: string) {
      const deletedSceneIds = new Set(
        allChapters(this.tree)
          .find((chapter) => chapter.id === chapterId)
          ?.scenes.map((scene) => scene.id) ?? [],
      );
      const beforeTree = this.tree;
      this.error = "";

      try {
        const tree = await deleteChapterCommand({ id: chapterId });
        this.setTree(
          tree,
          nextSceneAfterDelete(beforeTree, tree, deletedSceneIds, this.activeSceneId),
        );
      } catch (error) {
        this.error = normalizeError(error);
      }
    },
    async deleteScene(sceneId: string) {
      const beforeTree = this.tree;
      this.error = "";

      try {
        const tree = await deleteSceneCommand({ id: sceneId });
        this.setTree(
          tree,
          nextSceneAfterDelete(beforeTree, tree, new Set([sceneId]), this.activeSceneId),
        );
      } catch (error) {
        this.error = normalizeError(error);
      }
    },
  },
});
