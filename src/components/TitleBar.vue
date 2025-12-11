<script setup lang="ts">
import { Activity } from "lucide-vue-next";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { platform } from "@tauri-apps/plugin-os";
import { ref, onMounted } from "vue";

const isMac = ref(false);
const appWindow = getCurrentWindow();

onMounted(async () => {
  try {
    const os = platform();
    isMac.value = os === "macos";
  } catch {
    isMac.value = false;
  }
});

async function startDrag(e: MouseEvent) {
  if (e.button !== 0) return;
  const target = e.target as HTMLElement;
  if (target.closest("button")) return;
  
  await appWindow.startDragging();
}

async function minimize() {
  await appWindow.minimize();
}

async function toggleMaximize() {
  const isMaximized = await appWindow.isMaximized();
  if (isMaximized) {
    await appWindow.unmaximize();
  } else {
    await appWindow.maximize();
  }
}

async function close() {
  await appWindow.close();
}
</script>

<template>
  <header
    @mousedown="startDrag"
    class="fixed top-0 left-0 right-0 z-50 flex h-7 cursor-default items-center justify-between border-b border-[var(--border)]/50 bg-[var(--card)]/60 backdrop-blur-sm select-none"
  >
    <!-- macOS: Leave space for native traffic lights -->
    <div v-if="isMac" class="w-[70px] shrink-0" />

    <!-- App title - centered -->
    <div
      class="flex items-center gap-1.5 pointer-events-none"
      :class="isMac ? 'absolute left-1/2 -translate-x-1/2' : 'px-2'"
    >
      <Activity class="h-3 w-3 text-[var(--sell)]" />
      <span class="text-xs font-medium text-[var(--muted-foreground)]">
        Marketeer
      </span>
    </div>

    <!-- Spacer -->
    <div class="flex-1" />

    <!-- Windows/Linux controls -->
    <div v-if="!isMac" class="ml-auto flex">
      <button
        @click="minimize"
        class="flex h-7 w-10 items-center justify-center transition-colors hover:bg-[var(--accent)]"
      >
        <svg class="h-3 w-3 pointer-events-none" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <line x1="5" y1="12" x2="19" y2="12" />
        </svg>
      </button>
      <button
        @click="toggleMaximize"
        class="flex h-7 w-10 items-center justify-center transition-colors hover:bg-[var(--accent)]"
      >
        <svg class="h-2.5 w-2.5 pointer-events-none" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <rect x="4" y="4" width="16" height="16" rx="1" />
        </svg>
      </button>
      <button
        @click="close"
        class="flex h-7 w-10 items-center justify-center transition-colors hover:bg-red-600"
      >
        <svg class="h-3 w-3 pointer-events-none" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <line x1="6" y1="6" x2="18" y2="18" />
          <line x1="6" y1="18" x2="18" y2="6" />
        </svg>
      </button>
    </div>
  </header>
</template>
