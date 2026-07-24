import { defineStore } from "pinia";
import { normalizeUiError, type UiError } from "../errors";
import { searchProject, type SearchProjectResult, type SearchScope } from "../api/search";

export const useSearchStore = defineStore("search", {
  state: () => ({
    query: "",
    scope: "all" as SearchScope,
    results: [] as SearchProjectResult[],
    isLoading: false,
    error: null as UiError | null,
  }),
  actions: {
    reset() {
      this.query = "";
      this.scope = "all";
      this.results = [];
      this.error = null;
    },
    setQuery(query: string) {
      this.query = query;
      if (!query.trim()) {
        this.results = [];
        this.error = null;
      }
    },
    setScope(scope: SearchScope) {
      this.scope = scope;
    },
    async runSearch() {
      const query = this.query.trim();
      if (!query) {
        this.results = [];
        this.error = null;
        return;
      }

      this.isLoading = true;
      this.error = null;

      try {
        this.results = await searchProject(query, this.scope);
      } catch (error) {
        this.results = [];
        this.error = normalizeUiError(error);
      } finally {
        this.isLoading = false;
      }
    },
  },
});
