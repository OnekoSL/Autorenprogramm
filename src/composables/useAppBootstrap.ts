import { onMounted, ref } from "vue";
import { getAppInfo, type AppInfo } from "../api/app";
import { useAiStore } from "../stores/ai";
import { useProjectStore } from "../stores/project";

export function useAppBootstrap() {
  const projectStore = useProjectStore();
  const aiStore = useAiStore();
  const appInfo = ref<AppInfo | null>(null);
  const appInfoFailed = ref(false);

  onMounted(async () => {
    await Promise.all([
      projectStore.loadRecentProjects(),
      projectStore.loadProjectDefaults(),
      aiStore.loadSettings(),
    ]);

    try {
      appInfo.value = await getAppInfo();
    } catch {
      appInfoFailed.value = true;
    }
  });

  return { appInfo, appInfoFailed };
}
