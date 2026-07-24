import { defineStore } from "pinia";
import { normalizeUiError, type UiError } from "../errors";
import {
  createProject as createProjectCommand,
  getProjectDefaults,
  listRecentProjects,
  openProject as openProjectCommand,
  removeRecentProject as removeRecentProjectCommand,
  saveProjectCopyAs as saveProjectCopyAsCommand,
  updateProject as updateProjectCommand,
  updateProjectDefaults,
  type CreateProjectInput,
  type ProjectSnapshot,
  type ProjectDefaultsSnapshot,
  type RecentProject,
  type UpdateProjectInput,
} from "../api/project";

export const useProjectStore = defineStore("project", {
  state: () => ({
    activeProject: null as ProjectSnapshot | null,
    recentProjects: [] as RecentProject[],
    projectDefaults: null as ProjectDefaultsSnapshot | null,
    isLoading: false,
    isSavingCopy: false,
    isSavingDefaults: false,
    lastCopyPath: "",
    copyError: null as UiError | null,
    defaultsError: null as UiError | null,
    error: null as UiError | null,
  }),
  actions: {
    async loadProjectDefaults() {
      this.defaultsError = null;
      try {
        this.projectDefaults = await getProjectDefaults();
      } catch (error) {
        this.defaultsError = normalizeUiError(error);
      }
    },
    async saveProjectDefaults(storyStructureTemplateId: string) {
      this.isSavingDefaults = true;
      this.defaultsError = null;
      try {
        this.projectDefaults = await updateProjectDefaults({ storyStructureTemplateId });
        return true;
      } catch (error) {
        this.defaultsError = normalizeUiError(error);
        return false;
      } finally {
        this.isSavingDefaults = false;
      }
    },
    async loadRecentProjects() {
      this.error = null;

      try {
        this.recentProjects = await listRecentProjects();
      } catch (error) {
        this.error = normalizeUiError(error);
      }
    },
    async createProject(input: CreateProjectInput) {
      this.isLoading = true;
      this.error = null;
      this.copyError = null;
      this.lastCopyPath = "";

      try {
        this.activeProject = await createProjectCommand(input);
        await this.loadRecentProjects();
      } catch (error) {
        this.error = normalizeUiError(error);
      } finally {
        this.isLoading = false;
      }
    },
    async openProject(path: string) {
      this.isLoading = true;
      this.error = null;
      this.copyError = null;
      this.lastCopyPath = "";

      try {
        this.activeProject = await openProjectCommand(path);
        await this.loadRecentProjects();
      } catch (error) {
        this.error = normalizeUiError(error);
      } finally {
        this.isLoading = false;
      }
    },
    async removeRecentProject(path: string) {
      this.error = null;

      try {
        await removeRecentProjectCommand(path);
        await this.loadRecentProjects();
      } catch (error) {
        this.error = normalizeUiError(error);
      }
    },
    async updateProject(input: UpdateProjectInput) {
      this.isLoading = true;
      this.error = null;

      try {
        this.activeProject = await updateProjectCommand(input);
        await this.loadRecentProjects();
      } catch (error) {
        this.error = normalizeUiError(error);
      } finally {
        this.isLoading = false;
      }
    },
    async saveCopyAs(path: string) {
      this.isSavingCopy = true;
      this.copyError = null;

      try {
        this.lastCopyPath = await saveProjectCopyAsCommand(path);
      } catch (error) {
        this.copyError = normalizeUiError(error);
      } finally {
        this.isSavingCopy = false;
      }
    },
    setCopyError(error: unknown, fallbackCode = "unexpected_error") {
      this.copyError = normalizeUiError(error, fallbackCode);
    },
  },
});
