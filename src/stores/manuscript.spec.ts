import { beforeEach, describe, expect, it, vi } from "vitest";
import { createPinia, setActivePinia } from "pinia";
import type { ManuscriptTree } from "../api/manuscript";
import { setLocale, type ProjectLanguage } from "../i18n";
import { useManuscriptStore } from "./manuscript";

const manuscriptApi = vi.hoisted(() => ({
  createArea: vi.fn(), createChapter: vi.fn(), createScene: vi.fn(),
  deleteArea: vi.fn(), deleteChapter: vi.fn(), deleteScene: vi.fn(),
  getManuscriptTree: vi.fn(), moveArea: vi.fn(), moveChapter: vi.fn(), moveScene: vi.fn(),
  updateArea: vi.fn(), updateChapter: vi.fn(), updateScene: vi.fn(),
}));

vi.mock("../api/manuscript", () => manuscriptApi);

const tree: ManuscriptTree = {
  novel: { id: "novel-1", title: "Roman", subtitle: "", authorName: "" },
  areas: [{
    id: "area-1", title: "Bereich", summary: "", sortOrder: 0,
    chapters: [{
      id: "chapter-1", areaId: "area-1", title: "Kapitel", summary: "", sortOrder: 0,
      scenes: [
        { id: "scene-1", chapterId: "chapter-1", title: "Szene 1", text: "", status: "draft", summary: "", sortOrder: 0 },
        { id: "scene-2", chapterId: "chapter-1", title: "Szene 2", text: "", status: "draft", summary: "", sortOrder: 1 },
      ],
    }],
  }],
};

describe("Manuskript-Store", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    for (const mock of [manuscriptApi.createArea, manuscriptApi.createChapter, manuscriptApi.createScene]) {
      mock.mockResolvedValue(tree);
    }
  });

  it("behält eine gültige Szenenauswahl und fällt sonst auf die erste Szene zurück", () => {
    const store = useManuscriptStore();
    store.setTree(tree, "scene-2");
    expect(store.activeSceneId).toBe("scene-2");
    store.setTree(tree, "missing-scene");
    expect(store.activeSceneId).toBe("scene-1");
  });

  it.each([
    ["de", "Neuer Bereich", "Neues Kapitel", "Neue Szene"],
    ["en", "New part", "New chapter", "New scene"],
  ] as Array<[ProjectLanguage, string, string, string]>)
  ("verwendet lokalisierte Standardnamen für %s", async (language, areaTitle, chapterTitle, sceneTitle) => {
    setLocale(language);
    const store = useManuscriptStore();
    store.setTree(tree, "scene-1");

    await store.createArea();
    await store.createChapter("area-1");
    await store.createScene("chapter-1");

    expect(manuscriptApi.createArea).toHaveBeenCalledWith(expect.objectContaining({ title: areaTitle }));
    expect(manuscriptApi.createChapter).toHaveBeenCalledWith(expect.objectContaining({ title: chapterTitle }));
    expect(manuscriptApi.createScene).toHaveBeenCalledWith(expect.objectContaining({ title: sceneTitle }));
  });
});
