import { defineStore } from "pinia";

export type WorkspaceSection = "writing" | "bible" | "search" | "export" | "ai" | "settings";
export type SidebarTab = "context" | "bible" | "ai";
export type LayoutPanel = "navigator" | "sidebar";

export interface WorkspaceLayout {
  navigator: number;
  sidebar: number;
}

export const defaultWorkspaceLayout: WorkspaceLayout = {
  navigator: 280,
  sidebar: 360,
};

export const useWorkspaceStore = defineStore("workspace", {
  state: () => ({
    activeSection: "writing" as WorkspaceSection,
    sidebarTab: "context" as SidebarTab,
    layout: { ...defaultWorkspaceLayout },
  }),
  actions: {
    setActiveSection(section: WorkspaceSection) {
      this.activeSection = section;
    },
    setSidebarTab(tab: SidebarTab) {
      this.sidebarTab = tab;
    },
    setPanelWidth(panel: LayoutPanel, width: number) {
      this.layout[panel] = width;
    },
    setLayout(layout: WorkspaceLayout) {
      this.layout = { ...layout };
    },
    resetLayout() {
      this.layout = { ...defaultWorkspaceLayout };
    },
  },
});
