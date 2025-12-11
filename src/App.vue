<script setup lang="ts">
import { onMounted, onUnmounted } from "vue";
import { register, unregister } from "@tauri-apps/plugin-global-shortcut";
import { useMarketStore } from "./stores/market";
import TitleBar from "./components/TitleBar.vue";
import ItemDetails from "./components/ItemDetails.vue";
import PriceChart from "./components/PriceChart.vue";
import PriceCard from "./components/PriceCard.vue";
import Toolbar from "./components/Toolbar.vue";
import OrderBook from "./components/OrderBook.vue";
import EmptyState from "./components/EmptyState.vue";

const store = useMarketStore();
const SHORTCUT = "CommandOrControl+M";

onMounted(async () => {
  await store.initialize();

  // Register global shortcut for toggle mode (works even when app is not focused)
  try {
    await register(SHORTCUT, (event) => {
      // Only trigger on key press, not release
      if (event.state === "Pressed") {
        console.log("Global shortcut triggered");
        store.toggleMode();
      }
    });
    console.log("Global shortcut registered:", SHORTCUT);
  } catch (e) {
    console.error("Failed to register global shortcut:", e);
  }
});

onUnmounted(async () => {
  try {
    await unregister(SHORTCUT);
  } catch (e) {
    console.error("Failed to unregister global shortcut:", e);
  }
});

function handleCardClick(type: "sell" | "buy") {
  if (type !== store.mode) {
    store.toggleMode();
  }
}

async function handleCopy() {
  if (store.activePrice) {
    const { invoke } = await import("@tauri-apps/api/core");
    await invoke("copy_price", { price: store.activePrice });
    store.lastCopiedPrice = store.activePrice;
  }
}
</script>

<template>
  <TitleBar />

  <!-- Main content -->
  <main v-if="store.hasData" class="min-h-screen space-y-4 p-4 pt-11">
    <!-- Item Details -->
    <ItemDetails
      :item-name="store.marketData.itemName"
      :timestamp="store.marketData.timestamp"
      :type-info="store.typeInfo"
      :last-copied-price="store.lastCopiedPrice"
    />

    <!-- Price History Chart -->
    <PriceChart
      v-if="store.priceHistory.length > 0"
      :data="store.priceHistory"
    />

    <!-- Price Cards -->
    <div class="grid grid-cols-1 gap-3 lg:grid-cols-2">
      <PriceCard
        type="sell"
        :adjusted-price="store.marketData.adjustedSell"
        :market-price="store.marketData.cheapestSell"
        :is-active="store.mode === 'sell'"
        :class="{ 'hidden lg:block': store.mode !== 'sell' }"
        @click="handleCardClick('sell')"
      />
      <PriceCard
        type="buy"
        :adjusted-price="store.marketData.adjustedBuy"
        :market-price="store.marketData.highestBuy"
        :is-active="store.mode === 'buy'"
        :class="{ 'hidden lg:block': store.mode !== 'buy' }"
        @click="handleCardClick('buy')"
      />
    </div>

    <!-- Toolbar -->
    <Toolbar
      :mode="store.mode"
      :active-price="store.activePrice"
      @toggle="store.toggleMode"
      @copy="handleCopy"
    />

    <!-- Order Books -->
    <div class="grid grid-cols-1 gap-3 lg:grid-cols-2">
      <OrderBook
        type="sell"
        :orders="store.marketData.sellOrders"
        :total-count="store.marketData.sellOrderCount"
        :class="{ 'hidden lg:block': store.mode !== 'sell' }"
      />
      <OrderBook
        type="buy"
        :orders="store.marketData.buyOrders"
        :total-count="store.marketData.buyOrderCount"
        :class="{ 'hidden lg:block': store.mode !== 'buy' }"
      />
    </div>
  </main>

  <!-- Empty state -->
  <EmptyState v-else @refresh="store.refreshData" />
</template>
