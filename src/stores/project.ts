import { defineStore } from "pinia";
import {
  createProject as createProjectCommand,
  listRecentProjects,
  openProject as openProjectCommand,
  removeRecentProject as removeRecentProjectCommand,
  saveProjectCopyAs as saveProjectCopyAsCommand,
  updateProject as updateProjectCommand,
  type CommandError,
  type CreateProjectInput,
  type ProjectSnapshot,
  type RecentProject,
  type UpdateProjectInput,
} from "../api/project";

function normalizeError(error: unknown) {
  if (typeof error === "object" && error && "message" in error) {
    return (error as CommandError).message;
  }

  if (typeof error === "string") {
    return error;
  }

  return "Die Projektaktion ist fehlgeschlagen.";
}

export const useProjectStore = defineStore("project", {
  state: () => ({
    activeProject: null as ProjectSnapshot | null,
    recentProjects: [] as RecentProject[],
    isLoading: false,
    isSavingCopy: false,
    lastCopyPath: "",
    copyError: "",
    error: "",
  }),
  actions: {
    async loadRecentProjects() {
      this.error = "";

      try {
        this.recentProjects = await listRecentProjects();
      } catch (error) {
        this.error = normalizeError(error);
      }
    },
    async createProject(input: CreateProjectInput) {
      this.isLoading = true;
      this.error = "";
      this.copyError = "";
      this.lastCopyPath = "";

      try {
        this.activeProject = await createProjectCommand(input);
        await this.loadRecentProjects();
      } catch (error) {
        this.error = normalizeError(error);
      } finally {
        this.isLoading = false;
      }
    },
    async openProject(path: string) {
      this.isLoading = true;
      this.error = "";
      this.copyError = "";
      this.lastCopyPath = "";

      try {
        this.activeProject = await openProjectCommand(path);
        await this.loadRecentProjects();
      } catch (error) {
        this.error = normalizeError(error);
      } finally {
        this.isLoading = false;
      }
    },
    async removeRecentProject(path: string) {
      this.error = "";

      try {
        await removeRecentProjectCommand(path);
        await this.loadRecentProjects();
      } catch (error) {
        this.error = normalizeError(error);
      }
    },
    async updateProject(input: UpdateProjectInput) {
      this.isLoading = true;
      this.error = "";

      try {
        this.activeProject = await updateProjectCommand(input);
        await this.loadRecentProjects();
      } catch (error) {
        this.error = normalizeError(error);
      } finally {
        this.isLoading = false;
      }
    },
    async saveCopyAs(path: string) {
      this.isSavingCopy = true;
      this.copyError = "";

      try {
        this.lastCopyPath = await saveProjectCopyAsCommand(path);
      } catch (error) {
        this.copyError = normalizeError(error);
      } finally {
        this.isSavingCopy = false;
      }
    },
    setCopyError(message: string) {
      this.copyError = message;
    },
  },
});
