import { beforeEach, describe, expect, it, vi } from "vitest";
import { createPinia, setActivePinia } from "pinia";
import type { ProjectSnapshot } from "../api/project";
import { useProjectStore } from "./project";

const projectApi = vi.hoisted(() => ({
  createProject: vi.fn(),
  getProjectDefaults: vi.fn(),
  listRecentProjects: vi.fn(),
  openProject: vi.fn(),
  removeRecentProject: vi.fn(),
  saveProjectCopyAs: vi.fn(),
  updateProject: vi.fn(),
  updateProjectDefaults: vi.fn(),
}));

vi.mock("../api/project", () => projectApi);

const project: ProjectSnapshot = {
  id: "project-1",
  name: "Roman",
  description: "Beschreibung",
  default_language: "de",
  editor_font_family: "serif",
  editor_font_size: 19,
  path: "C:/Roman.autorenprojekt",
  storage_kind: "file",
  created_at: "2026-07-22T00:00:00Z",
  updated_at: "2026-07-22T00:00:00Z",
};

describe("Projekt-Store", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    projectApi.listRecentProjects.mockResolvedValue([]);
  });

  it("führt Ladezustand und Projektsnapshot bei erfolgreicher Erstellung konsistent", async () => {
    let resolveProject!: (value: ProjectSnapshot) => void;
    projectApi.createProject.mockReturnValue(new Promise<ProjectSnapshot>((resolve) => {
      resolveProject = resolve;
    }));
    const store = useProjectStore();
    const input = { name: "Roman", description: "Beschreibung", defaultLanguage: "de" as const, path: project.path };

    const creation = store.createProject(input);
    expect(store.isLoading).toBe(true);
    expect(store.error).toBeNull();
    resolveProject(project);
    await creation;

    expect(projectApi.createProject).toHaveBeenCalledWith(input);
    expect(store.activeProject).toEqual(project);
    expect(store.isLoading).toBe(false);
    expect(projectApi.listRecentProjects).toHaveBeenCalledOnce();
  });

  it("bewahrt Backendcode und Diagnosedetail als strukturierten Fehler", async () => {
    projectApi.createProject.mockRejectedValue({ code: "project_file_exists", message: "C:/Roman.autorenprojekt" });
    const store = useProjectStore();

    await store.createProject({ name: "Roman", defaultLanguage: "de", path: project.path });

    expect(store.activeProject).toBeNull();
    expect(store.error).toEqual({ code: "project_file_exists", detail: "C:/Roman.autorenprojekt" });
    expect(store.isLoading).toBe(false);
  });
});
