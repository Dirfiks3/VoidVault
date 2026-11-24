import { create } from "zustand";
import { api, errorMessage } from "../lib/ipc";
import type { Category, ItemInput, ItemSummary, VaultItem, VaultStatus, View } from "../types";

type ToastTone = "ok" | "error";

interface Toast {
  text: string;
  tone: ToastTone;
}

interface EditorState {
  open: boolean;
  item: VaultItem | null;
}

interface VaultStore {
  status: VaultStatus | null;
  items: ItemSummary[];
  query: string;
  category: Category | "all";
  view: View;
  editor: EditorState;
  toast: Toast | null;
  busy: boolean;
  clipboardLeft: number;

  init: () => Promise<void>;
  refresh: (silent?: boolean) => Promise<void>;
  setQuery: (query: string) => void;
  setCategory: (category: Category | "all") => void;
  setView: (view: View) => void;
  createVault: (masterPassword: string) => Promise<void>;
  unlock: (masterPassword: string) => Promise<void>;
  lock: () => Promise<void>;
  setAutoLock: (minutes: number) => Promise<void>;
  openNew: () => void;
  openItem: (id: string) => Promise<void>;
  closeEditor: () => void;
  saveItem: (item: ItemInput) => Promise<void>;
  deleteItem: (id: string) => Promise<void>;
  toggleFavorite: (id: string) => Promise<void>;
  copy: (text: string) => Promise<void>;
  touch: () => Promise<void>;
  notify: (text: string, tone?: ToastTone) => void;
  indicateClipboardCleared: () => void;
}

let toastTimer: number | undefined;
let clipboardTimer: number | undefined;
let lastTouch = 0;

export const useVaultStore = create<VaultStore>((set, get) => ({
  status: null,
  items: [],
  query: "",
  category: "all",
  view: "items",
  editor: { open: false, item: null },
  toast: null,
  busy: false,
  clipboardLeft: 0,

  notify: (text, tone = "ok") => {
    if (toastTimer) window.clearTimeout(toastTimer);
    set({ toast: { text, tone } });
    toastTimer = window.setTimeout(() => set({ toast: null }), 3600);
  },

  indicateClipboardCleared: () => {
    if (clipboardTimer) window.clearInterval(clipboardTimer);
    set({ clipboardLeft: 0 });
    get().notify("Буфер обмена очищен");
  },

  init: async () => {
    try {
      const status = await api.vaultStatus();
      set({ status });
      if (status.unlocked) await get().refresh(true);
    } catch (error) {
      get().notify(errorMessage(error), "error");
    }
  },

  refresh: async (silent = false) => {
    if (!get().status?.unlocked) return;
    if (!silent) set({ busy: true });
    try {
      const items = await api.listItems(get().query || undefined, get().category);
      set({ items });
    } catch (error) {
      if (!silent) get().notify(errorMessage(error), "error");
      if (errorMessage(error).includes("заблокирован")) {
        set({ status: { ...get().status!, unlocked: false }, items: [], editor: { open: false, item: null } });
      }
    } finally {
      if (!silent) set({ busy: false });
    }
  },

  setQuery: (query) => set({ query }),
  setCategory: (category) => set({ category }),
  setView: (view) => set({ view }),

  createVault: async (masterPassword) => {
    try {
      const status = await api.createVault(masterPassword);
      set({ status });
      await get().refresh(true);
      get().notify("Хранилище создано");
    } catch (error) {
      get().notify(errorMessage(error), "error");
      throw error;
    }
  },

  unlock: async (masterPassword) => {
    try {
      const status = await api.unlockVault(masterPassword);
      set({ status });
      await get().refresh(true);
      get().notify("Хранилище разблокировано");
    } catch (error) {
      get().notify(errorMessage(error), "error");
      throw error;
    }
  },

  lock: async () => {
    try {
      const status = await api.lockVault();
      set({ status, items: [], editor: { open: false, item: null } });
      if (clipboardTimer) window.clearInterval(clipboardTimer);
      set({ clipboardLeft: 0 });
    } catch (error) {
      get().notify(errorMessage(error), "error");
    }
  },

  setAutoLock: async (minutes) => {
    try {
      const status = await api.setAutoLock(minutes);
      set({ status });
      get().notify(`Автоблокировка: ${minutes} мин`);
    } catch (error) {
      get().notify(errorMessage(error), "error");
    }
  },

  openNew: () => set({ editor: { open: true, item: null } }),

  openItem: async (id) => {
    try {
      const item = await api.getItem(id);
      set({ editor: { open: true, item } });
    } catch (error) {
      get().notify(errorMessage(error), "error");
    }
  },

  closeEditor: () => set({ editor: { open: false, item: null } }),

  saveItem: async (item) => {
    try {
      await api.saveItem(item);
      set({ editor: { open: false, item: null } });
      await get().refresh(true);
      get().notify("Запись сохранена");
    } catch (error) {
      get().notify(errorMessage(error), "error");
      throw error;
    }
  },

  deleteItem: async (id) => {
    try {
      await api.deleteItem(id);
      set({ editor: { open: false, item: null } });
      await get().refresh(true);
      get().notify("Запись удалена");
    } catch (error) {
      get().notify(errorMessage(error), "error");
    }
  },

  toggleFavorite: async (id) => {
    try {
      await api.toggleFavorite(id);
      await get().refresh(true);
    } catch (error) {
      get().notify(errorMessage(error), "error");
    }
  },

  copy: async (text) => {
    try {
      await api.copySecret(text);
      if (clipboardTimer) window.clearInterval(clipboardTimer);
      set({ clipboardLeft: 30 });
      clipboardTimer = window.setInterval(() => {
        const left = get().clipboardLeft - 1;
        if (left <= 0) {
          if (clipboardTimer) window.clearInterval(clipboardTimer);
          set({ clipboardLeft: 0 });
        } else {
          set({ clipboardLeft: left });
        }
      }, 1000);
      get().notify("Скопировано · автоочистка через 30 с");
    } catch (error) {
      get().notify(errorMessage(error), "error");
    }
  },

  touch: async () => {
    if (!get().status?.unlocked) return;
    const now = Date.now();
    if (now - lastTouch < 10_000) return;
    lastTouch = now;
    try {
      await api.touchVault();
    } catch {
      // Auto-lock is enforced by the backend; keep UI state in sync lazily.
    }
  },
}));
