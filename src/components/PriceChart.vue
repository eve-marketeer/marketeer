<script setup lang="ts">
import { computed } from "vue";
import { Line } from "vue-chartjs";
import {
  Chart as ChartJS,
  CategoryScale,
  LinearScale,
  PointElement,
  LineElement,
  Title,
  Tooltip,
  Filler,
  type ChartOptions,
  type ChartData,
} from "chart.js";
import type { MarketHistory } from "../types";

ChartJS.register(
  CategoryScale,
  LinearScale,
  PointElement,
  LineElement,
  Title,
  Tooltip,
  Filler
);

const props = defineProps<{
  data: MarketHistory[];
}>();

const chartData = computed<ChartData<"line">>(() => {
  const labels = props.data.map((d) => {
    const date = new Date(d.date);
    return date.toLocaleDateString("en-US", { month: "short", day: "numeric" });
  });

  return {
    labels,
    datasets: [
      {
        label: "Highest",
        data: props.data.map((d) => d.highest),
        borderColor: "oklch(0.7 0.2 145)",
        backgroundColor: "oklch(0.7 0.2 145 / 0.1)",
        borderWidth: 2,
        pointRadius: 0,
        pointHoverRadius: 4,
        tension: 0.3,
        fill: false,
      },
      {
        label: "Lowest",
        data: props.data.map((d) => d.lowest),
        borderColor: "oklch(0.75 0.18 55)",
        backgroundColor: "oklch(0.75 0.18 55 / 0.1)",
        borderWidth: 2,
        pointRadius: 0,
        pointHoverRadius: 4,
        tension: 0.3,
        fill: false,
      },
    ],
  };
});

const chartOptions = computed<ChartOptions<"line">>(() => ({
  responsive: true,
  maintainAspectRatio: false,
  interaction: {
    mode: "index",
    intersect: false,
  },
  plugins: {
    legend: {
      display: false,
    },
    tooltip: {
      backgroundColor: "oklch(0.16 0.015 260)",
      titleColor: "oklch(0.95 0 0)",
      bodyColor: "oklch(0.65 0 0)",
      borderColor: "oklch(0.25 0.02 260)",
      borderWidth: 1,
      padding: 10,
      displayColors: true,
      callbacks: {
        label: (context) => {
          const value = context.parsed.y;
          return `${context.dataset.label}: ${formatCompact(value ?? 0)} ISK`;
        },
      },
    },
  },
  scales: {
    x: {
      display: false,
    },
    y: {
      display: true,
      grid: {
        color: "oklch(0.25 0.02 260 / 0.3)",
      },
      border: {
        display: false,
      },
      ticks: {
        color: "oklch(0.65 0 0)",
        font: {
          size: 10,
        },
        callback: (value) => formatCompact(value as number),
        maxTicksLimit: 3,
      },
    },
  },
}));

function formatCompact(value: number): string {
  return new Intl.NumberFormat("en-US", {
    notation: "compact",
    maximumFractionDigits: 1,
  }).format(value);
}
</script>

<template>
  <div class="rounded-xl border border-[var(--border)] bg-[var(--card)] p-4">
    <!-- Header -->
    <div class="mb-3 flex items-center justify-between">
      <span class="text-xs font-medium uppercase tracking-wider text-[var(--muted-foreground)]">
        30 Day Price History
      </span>
      <div class="flex items-center gap-4">
        <div class="flex items-center gap-1.5">
          <span class="h-2 w-2 rounded-full bg-[var(--sell)]" />
          <span class="text-[10px] text-[var(--muted-foreground)]">High</span>
        </div>
        <div class="flex items-center gap-1.5">
          <span class="h-2 w-2 rounded-full bg-[var(--buy)]" />
          <span class="text-[10px] text-[var(--muted-foreground)]">Low</span>
        </div>
      </div>
    </div>

    <!-- Chart -->
    <div class="h-[120px]">
      <Line v-if="data.length > 1" :data="chartData" :options="chartOptions" />
      <div
        v-else
        class="flex h-full items-center justify-center text-xs text-[var(--muted-foreground)]"
      >
        No price history available
      </div>
    </div>
  </div>
</template>

