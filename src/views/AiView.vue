<script lang="ts">
import { defineComponent, onUnmounted } from "vue";
import { useWorkspaceViewContext } from "../composables/workspaceViewContext";
import { useWorkspaceNavigation } from "../composables/workspaceNavigation";

export default defineComponent({
  name: "AiView",
  setup() {
    const context = useWorkspaceViewContext();
    const navigation = useWorkspaceNavigation();
    const guard = context.viewGuards?.["ai"];
    if (guard) navigation.register("ai", guard);
    onUnmounted(() => navigation.unregister("ai", guard));
    return context;
  },
});
</script>

<template>
<section class="editor ai-chat-editor" :aria-label="ui('KI-Chat')">
        <div class="ai-chat-heading">
          <div>
            <p class="eyebrow">{{ ui("KI") }}</p>
            <h2>{{ ui("Chat") }}</h2>
          </div>
          <button type="button" @click="aiStore.clearChat">{{ ui("Neue Unterhaltung") }}</button>
        </div>

        <div class="ai-chat-mode-switch" :aria-label="ui('KI-Modus')">
          <button
            type="button"
            :class="{ active: aiChatMode === 'ideas' }"
            @click="aiChatMode = 'ideas'"
          >
            {{ ui("Ideenberater") }}
          </button>
          <button
            type="button"
            :class="{ active: aiChatMode === 'writing' }"
            @click="aiChatMode = 'writing'"
          >
            {{ ui("Schreibassistent") }}
          </button>
        </div>

        <div class="ai-chat-messages" aria-live="polite">
          <p v-if="!aiStore.chatMessages.length" class="muted-text">
            {{ ui("Stelle eine Frage zum Projekt, zur aktuellen Szene oder zu einer Schreibentscheidung.") }}
          </p>
          <article
            v-for="(message, index) in aiStore.chatMessages"
            :key="`${message.role}:${index}`"
            class="ai-chat-message"
            :class="message.role"
          >
            <h3>{{ message.role === "user" ? t("dynamic.you") : ui("KI") }}</h3>
            <p>{{ message.content }}</p>
            <div v-if="message.role === 'assistant'" class="ai-chat-message-actions">
              <button
                type="button"
                class="primary-action"
                :disabled="!canAcceptAiChatText(message.content)"
                @click="acceptAiChatMessage(message.content)"
              >
                {{ ui("Übernehmen") }}
              </button>
            </div>
          </article>
          <p v-if="aiStore.isRunningChat" class="muted-text">{{ ui("KI schreibt Antwort...") }}</p>
        </div>

        <form class="ai-chat-composer" @submit.prevent="sendAiChat">
          <label class="checkbox-row">
            <input
              v-model="aiChatIncludeSceneContext"
              type="checkbox"
              :disabled="!manuscriptStore.activeSceneId"
            />
            <span>{{ ui("Aktuelle Szene als Kontext mitschicken") }}</span>
          </label>
          <textarea
            v-model="aiChatDraft"
            :placeholder="ui('Frage an die KI')"
            :disabled="aiStore.isRunningChat"
            @keydown="handleAiChatKeydown"
          />
          <button
            type="submit"
            class="primary-action"
            :disabled="!aiChatDraft.trim() || aiStore.isRunningChat || !aiStore.isProviderReachable"
          >
            {{ ui("Senden") }}
          </button>
          <p v-if="aiStore.error" class="error-text">{{ formatUiError(aiStore.error) }}</p>
        </form>
      </section>
</template>
