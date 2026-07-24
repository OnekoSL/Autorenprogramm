import { flushPromises, mount } from "@vue/test-utils";
import { createPinia } from "pinia";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { ProjectSnapshot } from "../api/project";
import { i18n, setLocale, type ProjectLanguage } from "../i18n";
import ProjectStartView from "./ProjectStartView.vue";

const projectApi = vi.hoisted(() => ({
  chooseProjectOpenPath: vi.fn(), chooseProjectSavePath: vi.fn(), createProject: vi.fn(),
  getProjectDefaults: vi.fn(), listRecentProjects: vi.fn(), openProject: vi.fn(),
  removeRecentProject: vi.fn(), saveProjectCopyAs: vi.fn(), updateProject: vi.fn(), updateProjectDefaults: vi.fn(),
}));

vi.mock("../api/project", () => projectApi);

function snapshot(language: ProjectLanguage): ProjectSnapshot {
  return {
    id: `project-${language}`,
    name: "Roman",
    description: "Beschreibung",
    default_language: language,
    editor_font_family: "serif",
    editor_font_size: 19,
    path: `C:/Roman-${language}.autorenprojekt`,
    storage_kind: "file",
    created_at: "2026-07-22T00:00:00Z",
    updated_at: "2026-07-22T00:00:00Z",
  };
}

function mountStartView() {
  return mount(ProjectStartView, { global: { plugins: [createPinia(), i18n] } });
}

describe("Projektstart", () => {
  beforeEach(() => {
    setLocale("de");
    projectApi.listRecentProjects.mockResolvedValue([]);
  });

  it("übersetzt eine bereits vorhandene Browser-Fehlersituation beim Sprachwechsel", async () => {
    const wrapper = mountStartView();
    expect(wrapper.text()).toContain("Das Desktop-Backend ist nicht verbunden.");
    expect(wrapper.text()).toContain("(tauri_unavailable)");

    await wrapper.get("select").setValue("en");

    expect(wrapper.text()).toContain("The desktop backend is not connected.");
    expect(wrapper.text()).toContain("(tauri_unavailable)");
  });

  it.each(["de", "en"] as ProjectLanguage[])
  ("übermittelt bei einer %s-Projekterstellung die unveränderliche Projektsprache", async (language) => {
    window.__TAURI_INTERNALS__ = {};
    projectApi.chooseProjectSavePath.mockResolvedValue(`C:/Roman-${language}.autorenprojekt`);
    projectApi.createProject.mockResolvedValue(snapshot(language));
    const wrapper = mountStartView();
    await wrapper.get('input[autocomplete="off"]').setValue("Roman");
    await wrapper.get("select").setValue(language);

    await wrapper.get("form").trigger("submit");
    await flushPromises();

    expect(projectApi.chooseProjectSavePath).toHaveBeenCalledWith(
      language === "en" ? "Roman - Project" : "Roman - Projekt",
    );
    expect(projectApi.createProject).toHaveBeenCalledWith({
      name: "Roman",
      description: "",
      defaultLanguage: language,
      path: `C:/Roman-${language}.autorenprojekt`,
    });
    wrapper.unmount();
  });
});
