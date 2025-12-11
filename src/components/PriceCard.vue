<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { TrendingDown, TrendingUp, Check } from "lucide-vue-next";

const props = defineProps<{
  type: "sell" | "buy";
  adjustedPrice: number | null;
  marketPrice: number | null;
  isActive: boolean;
}>();

const emit = defineEmits<{
  click: [];
}>();

const isSell = computed(() => props.type === "sell");

// Animated price display
const displayedPrice = ref(props.adjustedPrice ?? 0);
const displayedMarketPrice = ref(props.marketPrice ?? 0);
const priceFlash = ref(false);

watch(() => props.adjustedPrice, (newVal, oldVal) => {
  if (newVal !== null) {
    animateValue(displayedPrice, displayedPrice.value, newVal, 250);
    if (oldVal !== null && oldVal !== newVal) {
      triggerFlash();
    }
  }
}, { immediate: true });

watch(() => props.marketPrice, (newVal) => {
  if (newVal !== null) {
    animateValue(displayedMarketPrice, displayedMarketPrice.value, newVal, 250);
  }
}, { immediate: true });

function triggerFlash() {
  priceFlash.value = true;
  setTimeout(() => {
    priceFlash.value = false;
  }, 350);
}

function animateValue(target: { value: number }, from: number, to: number, duration: number) {
  const startTime = performance.now();
  const diff = to - from;
  
  function update(currentTime: number) {
    const elapsed = currentTime - startTime;
    const progress = Math.min(elapsed / duration, 1);
    
    // Ease out cubic
    const eased = 1 - Math.pow(1 - progress, 3);
    target.value = from + diff * eased;
    
    if (progress < 1) {
      requestAnimationFrame(update);
    } else {
      target.value = to;
    }
  }
  
  requestAnimationFrame(update);
}

function formatPrice(price: number | null): string {
  if (price === null) return "—";
  return new Intl.NumberFormat("en-US", {
    minimumFractionDigits: 2,
    maximumFractionDigits: 2,
  }).format(price) + " ISK";
}

function formatCompact(price: number | null): string {
  if (price === null) return "";
  return new Intl.NumberFormat("en-US", {
    notation: "compact",
    maximumFractionDigits: 1,
  }).format(price);
}
</script>

<template>
  <button
    @click="emit('click')"
    class="group relative w-full rounded-xl border p-4 text-left transition-all duration-200"
    :class="[
      isActive
        ? 'border-[var(--ring)] bg-[var(--card)] ring-2 ring-[var(--ring)]/20'
        : 'border-[var(--border)] bg-[var(--card)]/50 hover:border-[var(--muted-foreground)]/30 hover:bg-[var(--card)]',
    ]"
  >
    <!-- Header -->
    <div class="mb-3 flex items-center justify-between">
      <div class="flex items-center gap-2">
        <div
          class="flex h-7 w-7 items-center justify-center rounded-lg transition-colors duration-300"
          :class="isSell ? 'bg-[var(--sell)]/15' : 'bg-[var(--buy)]/15'"
        >
          <TrendingDown
            v-if="isSell"
            class="h-4 w-4 transition-transform duration-300"
            :class="[isSell ? 'text-[var(--sell)]' : 'text-[var(--buy)]', isActive ? 'scale-110' : '']"
          />
          <TrendingUp
            v-else
            class="h-4 w-4 transition-transform duration-300"
            :class="[isSell ? 'text-[var(--sell)]' : 'text-[var(--buy)]', isActive ? 'scale-110' : '']"
          />
        </div>
        <span
          class="text-xs font-semibold uppercase tracking-wider"
          :class="isSell ? 'text-[var(--sell)]' : 'text-[var(--buy)]'"
        >
          {{ type }}
        </span>
      </div>
      
      <Transition
        enter-active-class="transition-all duration-300"
        enter-from-class="opacity-0 scale-50"
        enter-to-class="opacity-100 scale-100"
        leave-active-class="transition-all duration-200"
        leave-from-class="opacity-100 scale-100"
        leave-to-class="opacity-0 scale-50"
      >
        <div
          v-if="isActive"
          class="flex h-5 w-5 items-center justify-center rounded-full"
          :class="isSell ? 'bg-[var(--sell)]' : 'bg-[var(--buy)]'"
        >
          <Check class="h-3 w-3 text-black" />
        </div>
      </Transition>
    </div>

    <!-- Adjusted Price -->
    <div class="flex items-baseline justify-between gap-2">
      <p
        class="font-mono text-2xl font-bold tabular-nums transition-colors duration-200"
        :class="[
          isActive ? 'text-[var(--foreground)]' : 'text-[var(--muted-foreground)]',
          priceFlash ? (isSell ? 'text-[var(--sell)]' : 'text-[var(--buy)]') : ''
        ]"
      >
        {{ formatPrice(adjustedPrice !== null ? displayedPrice : null) }}
      </p>
      <span 
        v-if="adjustedPrice" 
        class="font-mono text-2xl font-bold text-[var(--muted-foreground)]/40"
      >
        {{ formatCompact(displayedPrice) }}
      </span>
    </div>

    <!-- Market Price -->
    <p class="mt-1 text-xs text-[var(--muted-foreground)]">
      Order: {{ formatPrice(marketPrice !== null ? displayedMarketPrice : null) }}
    </p>
  </button>
</template>
