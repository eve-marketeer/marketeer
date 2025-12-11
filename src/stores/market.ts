import { defineStore } from "pinia";
import { ref, computed } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type {
  MarketData,
  MarketMode,
  TypeInfo,
  MarketHistory,
} from "../types";

export const useMarketStore = defineStore("market", () => {
  const marketData = ref<MarketData>({
    filename: null,
    typeId: null,
    itemName: null,
    timestamp: null,
    cheapestSell: null,
    highestBuy: null,
    adjustedSell: null,
    adjustedBuy: null,
    sellOrderCount: 0,
    buyOrderCount: 0,
    sellOrders: [],
    buyOrders: [],
  });

  const mode = ref<MarketMode>("sell");
  const typeInfo = ref<TypeInfo | null>(null);
  const priceHistory = ref<MarketHistory[]>([]);
  const loading = ref(false);
  const lastCopiedPrice = ref<number | null>(null);

  const hasData = computed(() => marketData.value.filename !== null);

  const activePrice = computed(() => {
    return mode.value === "sell"
      ? marketData.value.adjustedSell
      : marketData.value.adjustedBuy;
  });

  async function fetchMarketData() {
    loading.value = true;
    try {
      const data = await invoke<MarketData>("get_market_data");
      marketData.value = data;

      if (data.typeId) {
        await Promise.all([fetchTypeInfo(data.typeId), fetchPriceHistory(data.typeId)]);
      }

      if (activePrice.value) {
        lastCopiedPrice.value = activePrice.value;
      }
    } finally {
      loading.value = false;
    }
  }

  async function fetchTypeInfo(typeId: number) {
    const info = await invoke<TypeInfo | null>("fetch_type_info", { typeId });
    typeInfo.value = info;
  }

  async function fetchPriceHistory(typeId: number) {
    const history = await invoke<MarketHistory[]>("fetch_price_history", {
      typeId,
    });
    priceHistory.value = history;
  }

  async function toggleMode() {
    const newMode = await invoke<MarketMode>("toggle_mode");
    mode.value = newMode;
    
    if (activePrice.value) {
      lastCopiedPrice.value = activePrice.value;
    }
  }

  async function refreshData() {
    loading.value = true;
    try {
      const data = await invoke<MarketData>("refresh_market_data");
      marketData.value = data;

      if (data.typeId) {
        await Promise.all([fetchTypeInfo(data.typeId), fetchPriceHistory(data.typeId)]);
      }

      if (activePrice.value) {
        lastCopiedPrice.value = activePrice.value;
      }
    } finally {
      loading.value = false;
    }
  }

  async function initialize() {
    // Fetch initial data
    await fetchMarketData();
    const currentMode = await invoke<MarketMode>("get_current_mode");
    mode.value = currentMode;

    // Listen for market updates from file watcher
    listen<MarketData>("market-updated", async (event) => {
      marketData.value = event.payload;

      if (event.payload.typeId) {
        await Promise.all([
          fetchTypeInfo(event.payload.typeId),
          fetchPriceHistory(event.payload.typeId),
        ]);
      }

      if (activePrice.value) {
        lastCopiedPrice.value = activePrice.value;
      }
    });

    // Listen for mode changes
    listen<MarketMode>("mode-changed", (event) => {
      mode.value = event.payload;
      if (activePrice.value) {
        lastCopiedPrice.value = activePrice.value;
      }
    });
  }

  return {
    marketData,
    mode,
    typeInfo,
    priceHistory,
    loading,
    lastCopiedPrice,
    hasData,
    activePrice,
    fetchMarketData,
    toggleMode,
    refreshData,
    initialize,
  };
});

