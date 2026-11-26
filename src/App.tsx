import { AnimatePresence, motion } from "motion/react";
import { useEffect } from "react";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { Icon } from "./components/ui";
import { LockScreen } from "./features/lock/LockScreen";
import { ItemEditor } from "./features/vault/ItemEditor";
import { VaultScreen } from "./features/vault/VaultScreen";
import { useVaultStore } from "./store/vaultStore";

function Toast() {
  const toast = useVaultStore((state) => state.toast);
  return (
    <div className="pointer-events-none fixed bottom-[calc(var(--safe-bottom)+1.5rem)] left-1/2 z-[60] -translate-x-1/2">
      <AnimatePresence>
        {toast && (
          <motion.div
            initial={{ opacity: 0, y: 4 }}
            animate={{ opacity: 1, y: 0 }}
            exit={{ opacity: 0, y: 4 }}
            transition={{ duration: 0.16, ease: "easeOut" }}
            className="glass flex items-center gap-2 whitespace-nowrap rounded-lg px-4 py-2 text-xs text-[var(--void-text)]"
          >
            <Icon name={toast.tone === "error" ? "x" : "check"} />
            {toast.text}
          </motion.div>
        )}
      </AnimatePresence>
    </div>
  );
}

export default function App() {
  const status = useVaultStore((state) => state.status);

  useEffect(() => {
    void useVaultStore.getState().init();
  }, []);

  useEffect(() => {
    const activity = () => void useVaultStore.getState().touch();
    const visibility = () => {
      if (document.visibilityState === "hidden") void useVaultStore.getState().lock();
    };

    window.addEventListener("pointerdown", activity, { passive: true });
    window.addEventListener("keydown", activity);
    document.addEventListener("visibilitychange", visibility);

    let unlistenFocus: (() => void) | undefined;
    let unlistenClipboard: (() => void) | undefined;
    let unlistenAutoLock: (() => void) | undefined;

    try {
      void getCurrentWindow()
        .onFocusChanged(({ payload: focused }) => {
          if (!focused) void useVaultStore.getState().lock();
        })
        .then((unlisten) => {
          unlistenFocus = unlisten;
        })
        .catch(() => undefined);
    } catch {
      // Running outside Tauri (e.g. plain Vite preview).
    }

    void listen("clipboard-cleared", () => {
      useVaultStore.getState().indicateClipboardCleared();
    })
      .then((unlisten) => {
        unlistenClipboard = unlisten;
      })
      .catch(() => undefined);

    // Rust уже обнулил ключ; здесь только синхронизируем интерфейс и чистим буфер.
    void listen("vault-auto-locked", () => {
      void useVaultStore.getState().lock();
    })
      .then((unlisten) => {
        unlistenAutoLock = unlisten;
      })
      .catch(() => undefined);

    return () => {
      window.removeEventListener("pointerdown", activity);
      window.removeEventListener("keydown", activity);
      document.removeEventListener("visibilitychange", visibility);
      unlistenFocus?.();
      unlistenClipboard?.();
      unlistenAutoLock?.();
    };
  }, []);

  if (!status) {
    return (
      <div className="flex min-h-screen items-center justify-center bg-[var(--void-bg)]">
        <img src="/logo.png" alt="VoidVault" className="h-12 w-12 rounded-lg border border-[var(--void-line)]" />
      </div>
    );
  }

  return (
    <div className="min-h-screen bg-[var(--void-bg)] text-[var(--void-text)]">
      <AnimatePresence mode="wait">
        {status.unlocked ? (
          <motion.div key="vault" initial={{ opacity: 0 }} animate={{ opacity: 1 }} exit={{ opacity: 0 }} transition={{ duration: 0.16 }}>
            <VaultScreen />
            <ItemEditor />
          </motion.div>
        ) : (
          <motion.div key="lock" initial={{ opacity: 0 }} animate={{ opacity: 1 }} exit={{ opacity: 0 }} transition={{ duration: 0.16 }}>
            <LockScreen />
          </motion.div>
        )}
      </AnimatePresence>
      <Toast />
    </div>
  );
}
