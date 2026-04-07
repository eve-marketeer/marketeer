<script setup lang="ts">
import { computed, ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { X, Loader2, Copy, ClipboardCheck } from "lucide-vue-next";

defineProps<{ open: boolean }>();
const emit = defineEmits<{ close: [] }>();

type BulkResult = {
    input_name: string;
    resolved_name: string | null;
    type_id: number | null;
    qty: number;
    lowest_sell: number | null;
    undercut: number | null;
    error: string | null;
};

const input = ref("");
const results = ref<BulkResult[]>([]);
const loading = ref(false);
const copied = ref(false);
const errorMsg = ref<string | null>(null);

async function run() {
    if (!input.value.trim()) return;
    loading.value = true;
    errorMsg.value = null;
    results.value = [];
    try {
        results.value = await invoke<BulkResult[]>("bulk_undercut_items", {
            text: input.value,
        });
    } catch (e) {
        errorMsg.value = String(e);
    } finally {
        loading.value = false;
    }
}

async function copyAll() {
    if (!results.value.length) return;
    const ok = await invoke<boolean>("copy_bulk_results", {
        results: results.value,
    });
    if (ok) {
        copied.value = true;
        setTimeout(() => (copied.value = false), 1500);
    }
}

function close() {
    emit("close");
}

const summary = computed(() => {
    let items = 0;
    let units = 0;
    let total = 0;
    let missing = 0;
    for (const r of results.value) {
        if (r.undercut != null) {
            items += 1;
            units += r.qty;
            total += r.undercut * r.qty;
        } else {
            missing += 1;
        }
    }
    return { items, units, total, missing };
});

function fmt(n: number | null): string {
    if (n == null) return "—";
    return n.toLocaleString("en-US", { maximumFractionDigits: 2 });
}
</script>

<template>
    <div
        v-if="open"
        class="fixed inset-0 z-50 flex items-center justify-center bg-black/60 p-4"
        @click.self="close"
    >
        <div
            class="flex max-h-[90vh] w-full max-w-4xl flex-col overflow-hidden rounded-xl border border-[var(--border)] bg-[var(--card)] shadow-2xl"
        >
            <!-- Header -->
            <div
                class="flex items-center justify-between border-b border-[var(--border)] px-5 py-3"
            >
                <h2 class="text-base font-semibold">Bulk Undercut (Jita 4-4)</h2>
                <button
                    class="rounded-md p-1 text-[var(--muted-foreground)] hover:bg-[var(--accent)]"
                    @click="close"
                >
                    <X class="h-4 w-4" />
                </button>
            </div>

            <!-- Body -->
            <div class="flex-1 space-y-4 overflow-auto p-5">
                <div>
                    <label class="mb-1 block text-xs text-[var(--muted-foreground)]">
                        Paste your requirements list (one per line: "Item Name" + tab/space +
                        quantity)
                    </label>
                    <textarea
                        v-model="input"
                        rows="8"
                        spellcheck="false"
                        class="w-full resize-y rounded-md border border-[var(--border)] bg-[var(--background)] p-2 font-mono text-xs outline-none focus:border-[var(--ring)]"
                        placeholder="Multispectrum Shield Hardener II    7&#10;1600mm Steel Plates II    2"
                    ></textarea>
                </div>

                <div class="flex items-center gap-2">
                    <button
                        :disabled="loading || !input.trim()"
                        class="flex items-center gap-2 rounded-md bg-[var(--primary)] px-3 py-1.5 text-sm text-[var(--primary-foreground)] hover:opacity-90 disabled:opacity-40"
                        @click="run"
                    >
                        <Loader2 v-if="loading" class="h-4 w-4 animate-spin" />
                        <span>{{ loading ? "Fetching…" : "Fetch & Undercut" }}</span>
                    </button>

                    <button
                        v-if="results.length"
                        class="ml-auto flex items-center gap-2 rounded-md border border-[var(--border)] px-3 py-1.5 text-sm hover:bg-[var(--accent)]"
                        :class="copied ? 'text-[var(--sell)]' : ''"
                        @click="copyAll"
                    >
                        <ClipboardCheck v-if="copied" class="h-4 w-4" />
                        <Copy v-else class="h-4 w-4" />
                        <span>{{ copied ? "Copied!" : "Copy Sell Orders" }}</span>
                    </button>
                </div>

                <div
                    v-if="errorMsg"
                    class="rounded-md border border-red-500/40 bg-red-500/10 p-2 text-xs text-red-400"
                >
                    {{ errorMsg }}
                </div>

                <!-- Results -->
                <div
                    v-if="results.length"
                    class="overflow-hidden rounded-md border border-[var(--border)]"
                >
                    <table class="w-full text-xs">
                        <thead class="bg-[var(--muted)] text-[var(--muted-foreground)]">
                            <tr>
                                <th class="px-3 py-2 text-left">Item</th>
                                <th class="px-3 py-2 text-right">Qty</th>
                                <th class="px-3 py-2 text-right">Lowest Sell</th>
                                <th class="px-3 py-2 text-right">Undercut</th>
                            </tr>
                        </thead>
                        <tbody>
                            <tr
                                v-for="(r, i) in results"
                                :key="i"
                                class="border-t border-[var(--border)]"
                            >
                                <td class="px-3 py-1.5">
                                    <div>{{ r.resolved_name || r.input_name }}</div>
                                    <div v-if="r.error" class="text-[10px] text-red-400">
                                        {{ r.error }}
                                    </div>
                                </td>
                                <td class="px-3 py-1.5 text-right font-mono">{{ r.qty }}</td>
                                <td class="px-3 py-1.5 text-right font-mono">
                                    {{ fmt(r.lowest_sell) }}
                                </td>
                                <td class="px-3 py-1.5 text-right font-mono text-[var(--sell)]">
                                    {{ fmt(r.undercut) }}
                                </td>
                            </tr>
                        </tbody>
                        <tfoot class="border-t border-[var(--border)] bg-[var(--muted)]/40">
                            <tr>
                                <td class="px-3 py-2 text-[var(--muted-foreground)]">
                                    {{ summary.items }} item{{ summary.items === 1 ? "" : "s" }}
                                    <span v-if="summary.missing" class="text-red-400">
                                        ({{ summary.missing }} missing)
                                    </span>
                                </td>
                                <td class="px-3 py-2 text-right font-mono">
                                    {{ summary.units }}
                                </td>
                                <td class="px-3 py-2 text-right text-[var(--muted-foreground)]">
                                    Total
                                </td>
                                <td
                                    class="px-3 py-2 text-right font-mono font-semibold text-[var(--sell)]"
                                >
                                    {{ fmt(summary.total) }}
                                </td>
                            </tr>
                        </tfoot>
                    </table>
                </div>
            </div>
        </div>
    </div>
</template>
