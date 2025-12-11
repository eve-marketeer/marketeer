<script setup lang="ts">
import { Copy, HelpCircle } from "lucide-vue-next";
import { platform } from "@tauri-apps/plugin-os";
import { WebviewWindow } from "@tauri-apps/api/webviewWindow";
import { ref, onMounted } from "vue";

defineProps<{
  mode: "sell" | "buy";
  activePrice: number | null;
}>();

const emit = defineEmits<{
  toggle: [];
  copy: [];
}>();

const isMac = ref(false);
const copied = ref(false);

onMounted(async () => {
  try {
    isMac.value = platform() === "macos";
  } catch {
    isMac.value = false;
  }
});

function handleCopy() {
  emit("copy");
  copied.value = true;
  setTimeout(() => {
    copied.value = false;
  }, 1500);
}

async function openHelp() {
  const helpWindow = new WebviewWindow("help", {
    url: "/help.html",
    title: "About Marketeer",
    width: 800,
    height: 800,
    resizable: false,
    center: true,
  });
  
  helpWindow.once("tauri://error", (e) => {
    console.error("Failed to open help window:", e);
  });
}
</script>

<template>
  <div class="flex items-center justify-between">
    <!-- Copy button -->
    <button
      @click="handleCopy"
      :disabled="!$props.activePrice"
      class="flex items-center gap-2 rounded-lg px-3 py-1.5 text-sm transition-colors hover:bg-[var(--accent)] disabled:opacity-40 disabled:hover:bg-transparent"
      :class="copied ? 'text-[var(--sell)]' : 'text-[var(--muted-foreground)]'"
    >
      <Copy class="h-4 w-4" />
      <span>{{ copied ? "Copied!" : "Copy Price" }}</span>
    </button>

    <!-- Toggle mode helper -->
    <button
      @click="emit('toggle')"
      class="flex items-center gap-2.5 rounded-lg px-3 py-1.5 text-sm transition-colors hover:bg-[var(--accent)]"
    >
      <span class="text-[var(--muted-foreground)]">Toggle Mode</span>
      <div class="flex items-center gap-1">
        <kbd class="rounded border border-[var(--border)] bg-[var(--muted)] px-2 py-0.5 font-mono text-xs text-[var(--muted-foreground)]">
          {{ isMac ? "⌘" : "Ctrl" }}
        </kbd>
        <kbd class="rounded border border-[var(--border)] bg-[var(--muted)] px-2 py-0.5 font-mono text-xs text-[var(--muted-foreground)]">
          M
        </kbd>
      </div>
    </button>

    <!-- Help button -->
    <button
      @click="openHelp"
      class="flex items-center gap-2 rounded-lg px-3 py-1.5 text-sm text-[var(--muted-foreground)] transition-colors hover:bg-[var(--accent)]"
    >
      <HelpCircle class="h-4 w-4" />
      <span>Help</span>
    </button>
  </div>
</template>

