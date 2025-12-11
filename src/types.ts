export interface MarketOrder {
  price: number;
  volRemaining: number;
  typeId: number;
  orderId: string;
  isBuyOrder: boolean;
  jumps: number;
}

export interface MarketData {
  filename: string | null;
  typeId: number | null;
  itemName: string | null;
  timestamp: string | null;
  cheapestSell: number | null;
  highestBuy: number | null;
  adjustedSell: number | null;
  adjustedBuy: number | null;
  sellOrderCount: number;
  buyOrderCount: number;
  sellOrders: MarketOrder[];
  buyOrders: MarketOrder[];
}

export interface TypeInfo {
  typeId: number;
  name: string;
  description: string;
  iconUrl: string;
}

export interface MarketHistory {
  date: string;
  average: number;
  highest: number;
  lowest: number;
  orderCount: number;
  volume: number;
}

export type MarketMode = "sell" | "buy";

