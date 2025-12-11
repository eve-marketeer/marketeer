<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { Package, Check, Radio } from "lucide-vue-next";
import type { TypeInfo } from "../types";

const props = defineProps<{
  itemName: string | null;
  timestamp: string | null;
  typeInfo: TypeInfo | null;
  lastCopiedPrice: number | null;
}>();

const iconUrl = computed(() => props.typeInfo?.iconUrl ?? "");
const iconError = ref(false);

const showIcon = computed(() => iconUrl.value && !iconError.value);

const displayName = computed(
  () => props.typeInfo?.name || props.itemName || "Unknown Item"
);

const showCopied = ref(false);

watch(
  () => props.lastCopiedPrice,
  (newVal, oldVal) => {
    if (newVal && newVal !== oldVal) {
      showCopied.value = true;
      setTimeout(() => {
        showCopied.value = false;
      }, 2000);
    }
  }
);

watch(iconUrl, () => {
  iconError.value = false;
});

function onIconError() {
  iconError.value = true;
}

function formatTimestamp(timestamp: string | null): string {
  if (!timestamp) return "—";

  const match = timestamp.match(/^(\d{4})\.(\d{2})\.(\d{2})\s+(\d{2})(\d{2})(\d{2})$/);
  if (!match) return timestamp;

  const [, year, month, day, hour, minute, second] = match;
  const date = new Date(
    parseInt(year),
    parseInt(month) - 1,
    parseInt(day),
    parseInt(hour),
    parseInt(minute),
    parseInt(second)
  );

  const dateStr = date.toLocaleDateString("en-US", {
    month: "short",
    day: "numeric",
    year: "numeric",
  });
  const timeStr = date.toLocaleTimeString("en-US", {
    hour: "numeric",
    minute: "2-digit",
    hour12: true,
  });

  return `${dateStr} at ${timeStr}`;
}
</script>

<template>
  <div
    class="relative overflow-hidden rounded-xl border border-[var(--border)] bg-gradient-to-br from-[var(--card)] to-[var(--card)]/50"
  >
    <!-- Status badges -->
    <div class="absolute right-3 top-3 flex items-center gap-2">
      <Transition
        enter-active-class="transition-all duration-300"
        enter-from-class="opacity-0 translate-x-2"
        enter-to-class="opacity-100 translate-x-0"
        leave-active-class="transition-all duration-200"
        leave-from-class="opacity-100"
        leave-to-class="opacity-0"
      >
        <span
          v-if="showCopied"
          class="flex items-center gap-1 rounded-full bg-[var(--sell)]/15 px-2 py-0.5 text-[10px] font-medium text-[var(--sell)]"
        >
          <Check class="h-3 w-3" />
          Copied
        </span>
      </Transition>

      <span
        class="flex items-center gap-1.5 rounded-full border border-[var(--sell)]/30 bg-[var(--sell)]/10 px-2 py-0.5 text-[10px] font-medium text-[var(--sell)]"
      >
        <Radio class="h-3 w-3 animate-pulse" />
        Live
      </span>
    </div>

    <div class="flex gap-4 p-4">
      <!-- Item icon -->
      <div class="relative shrink-0">
        <div class="absolute inset-0 rounded-lg bg-[var(--sell)]/10 blur-xl" />
        <div
          class="relative h-16 w-16 overflow-hidden rounded-lg border border-[var(--border)]/50 bg-black/30"
        >
          <img
            v-if="showIcon"
            :src="iconUrl"
            :alt="displayName"
            class="h-full w-full object-cover"
            @error="onIconError"
          />
          <div
            v-else
            class="flex h-full w-full items-center justify-center bg-[var(--muted)]"
          >
            <Package class="h-8 w-8 text-[var(--muted-foreground)]/50" />
          </div>
        </div>
      </div>

      <!-- Item info -->
      <div class="min-w-0 flex-1 pt-1">
        <h1
          class="truncate text-xl font-bold tracking-tight text-[var(--foreground)]"
          :title="displayName"
        >
          {{ displayName }}
        </h1>
        <p class="mt-0.5 text-xs text-[var(--muted-foreground)]">
          {{ formatTimestamp(timestamp) }}
        </p>

        <!-- Description -->
        <p
          v-if="typeInfo?.description"
          class="mt-2 line-clamp-2 text-xs leading-relaxed text-[var(--muted-foreground)]/80"
        >
          {{ typeInfo.description }}
        </p>
      </div>
    </div>
  </div>
</template>
