import { invoke } from "@tauri-apps/api/core";
import { requireTauriRuntime } from "./tauri";

export type WorldSystemCategory = "magic" | "technology" | "psionics" | "biotech" | "supernatural" | "hybrid" | "other";
export interface WorldSystemSnapshot { id: string; projectId: string; name: string; category: WorldSystemCategory; templateId: string; principle: string; rules: string; capabilities: string; costs: string; limits: string; access: string; distribution: string; worldImpact: string; openQuestions: string; notes: string; createdAt: string; updatedAt: string; deletedAt?: string | null; }
export interface WorldSystemTemplate { id: string; name: string; category: WorldSystemCategory; guidingQuestions: string[]; }
export interface WorldbuildingSnapshot { systems: WorldSystemSnapshot[]; templates: WorldSystemTemplate[]; }
export type WorldSystemUpdate = Omit<WorldSystemSnapshot, "projectId" | "createdAt" | "updatedAt" | "deletedAt">;

export function getWorldbuilding(): Promise<WorldbuildingSnapshot> { requireTauriRuntime(); return invoke("get_worldbuilding"); }
export function createWorldSystem(name: string, category: WorldSystemCategory, templateId: string): Promise<WorldbuildingSnapshot> { requireTauriRuntime(); return invoke("create_world_system", { input: { name, category, templateId } }); }
export function updateWorldSystem(input: WorldSystemUpdate): Promise<WorldbuildingSnapshot> { requireTauriRuntime(); return invoke("update_world_system", { input }); }
export function deleteWorldSystem(id: string): Promise<WorldbuildingSnapshot> { requireTauriRuntime(); return invoke("delete_world_system", { input: { id } }); }
