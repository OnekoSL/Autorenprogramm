import { invoke } from "@tauri-apps/api/core";
import { requireTauriRuntime } from "./tauri";
import type { ProjectBibleCardType } from "./projectBible";

export type SearchScope = "all" | "scenes" | "bible";

export interface SearchProjectResult {
  kind: "scene" | "bible";
  id: string;
  title: string;
  subtitle: string;
  matchedFields: string[];
  snippets: string[];
  sceneId?: string | null;
  targetType?: ProjectBibleCardType | null;
  targetId?: string | null;
}

export function searchProject(query: string, scope: SearchScope): Promise<SearchProjectResult[]> {
  requireTauriRuntime();
  return invoke<SearchProjectResult[]>("search_project", { input: { query, scope } });
}
