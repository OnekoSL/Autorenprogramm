import { defineStore } from "pinia";
import { searchProject, type SearchProjectResult, type SearchScope } from "../api/search";
import type { CommandError } from "../api/project";

function normalizeError(error: unknown) {
  if (typeof error === "object" && error && "message" in error) {
    return (error as CommandError).message;
  }

  if (typeof error === "string") {
    return error;
  }

  return "Die Suche ist fehlgeschlagen.";
}

export const useSearchStore = defineStore("search", {
  state: () => ({
    query: "",
    scope: "all" as SearchScope,
    results: [] as SearchProjectResult[],
    isLoading: false,
    error: "",
  }),
  actions: {
    reset() {
      this.query = "";
      this.scope = "all";
      this.results = [];
      this.error = "";
    },
    setQuery(query: string) {
      this.query = query;
      if (!query.trim()) {
        this.results = [];
        this.error = "";
      }
    },
    setScope(scope: SearchScope) {
      this.scope = scope;
    },
    async runSearch() {
      const query = this.query.trim();
      if (!query) {
        this.results = [];
        this.error = "";
        return;
      }

      this.isLoading = true;
      this.error = "";

      try {
        this.results = await searchProject(query, this.scope);
      } catch (error) {
        this.results = [];
        this.error = normalizeError(error);
      } finally {
        this.isLoading = false;
      }
    },
  },
});
