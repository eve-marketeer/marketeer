<script setup lang="ts">
import { computed } from "vue";
import { TrendingDown, TrendingUp } from "lucide-vue-next";
import type { MarketOrder } from "../types";

const props = defineProps<{
  type: "sell" | "buy";
  orders: MarketOrder[];
  totalCount: number;
}>();

const isSell = computed(() => props.type === "sell");

function formatPrice(price: number): string {
  return new Intl.NumberFormat("en-US", {
    minimumFractionDigits: 2,
    maximumFractionDigits: 2,
  }).format(price);
}

function formatVolume(volume: number): string {
  return new Intl.NumberFormat("en-US").format(volume);
}
</script>

<template>
  <div class="overflow-hidden rounded-xl border border-[var(--border)] bg-[var(--card)]">
    <!-- Header -->
    <div class="flex items-center justify-between border-b border-[var(--border)] px-3 py-2">
      <div class="flex items-center gap-2">
        <TrendingDown
          v-if="isSell"
          class="h-3.5 w-3.5"
          :class="isSell ? 'text-[var(--sell)]' : 'text-[var(--buy)]'"
        />
        <TrendingUp
          v-else
          class="h-3.5 w-3.5"
          :class="isSell ? 'text-[var(--sell)]' : 'text-[var(--buy)]'"
        />
        <span
          class="text-xs font-semibold"
          :class="isSell ? 'text-[var(--sell)]' : 'text-[var(--buy)]'"
        >
          {{ isSell ? "Sell Orders" : "Buy Orders" }}
        </span>
      </div>
      <span
        class="rounded-full bg-[var(--muted)] px-2 py-0.5 text-[10px] font-medium text-[var(--muted-foreground)] transition-all duration-300"
      >
        {{ totalCount }}
      </span>
    </div>

    <!-- Orders List with transitions -->
    <TransitionGroup tag="div" name="order" class="relative divide-y divide-[var(--border)]/50">
      <div
        v-for="(order, index) in orders"
        :key="order.orderId"
        class="flex items-center justify-between px-3 py-1.5"
      >
        <span
          class="font-mono text-xs tabular-nums transition-colors duration-200"
          :class="
            index === 0
              ? isSell
                ? 'font-semibold text-[var(--sell)]'
                : 'font-semibold text-[var(--buy)]'
              : 'text-[var(--foreground)]'
          "
        >
          {{ formatPrice(order.price) }}
        </span>
        <span class="font-mono text-xs tabular-nums text-[var(--muted-foreground)] transition-colors duration-200">
          {{ formatVolume(order.volRemaining) }}
        </span>
      </div>
    </TransitionGroup>
    
    <!-- Empty state -->
    <div
      v-if="orders.length === 0"
      class="px-3 py-4 text-center text-xs text-[var(--muted-foreground)]"
    >
      No {{ isSell ? "sell" : "buy" }} orders available
    </div>
  </div>
</template>

<style scoped>
.order-move {
  transition: transform 0.2s ease-out;
}

.order-enter-active {
  transition: all 0.2s ease-out;
}

.order-leave-active {
  position: absolute;
  left: 0;
  right: 0;
  transition: all 0.15s ease-out;
}

.order-enter-from {
  opacity: 0;
  transform: translateX(-8px);
}

.order-leave-to {
  opacity: 0;
  transform: translateX(8px);
}
</style>
