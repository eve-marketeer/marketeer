<script setup lang="ts">
import { FileText, RefreshCw } from "lucide-vue-next";

const emit = defineEmits<{
  refresh: [];
}>();
</script>

<template>
  <div class="flex min-h-[calc(100vh-4rem)] items-center justify-center p-8">
    <div
      class="flex max-w-xs flex-col items-center rounded-2xl border-2 border-dashed border-[var(--border)] bg-[var(--card)]/30 p-10 text-center"
    >
      <div class="mb-4 rounded-full bg-[var(--muted)] p-4">
        <FileText class="h-8 w-8 text-[var(--muted-foreground)]" />
      </div>

      <h2 class="mb-2 text-lg font-semibold text-[var(--foreground)]">
        No Market Data
      </h2>

      <p class="mb-6 text-sm leading-relaxed text-[var(--muted-foreground)]">
        Export market data from EVE Online for an item to view its orders and price history.
      </p>

      <button
        @click="emit('refresh')"
        class="flex items-center gap-2 rounded-lg bg-[var(--accent)] px-4 py-2 text-sm font-medium text-[var(--foreground)] transition-colors hover:bg-[var(--muted)]"
      >
        <RefreshCw class="h-4 w-4" />
        Refresh
      </button>
    </div>
  </div>
</template>

