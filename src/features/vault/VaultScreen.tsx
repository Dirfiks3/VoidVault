import { AnimatePresence, motion } from "motion/react";
import { useEffect, useMemo, useRef, useState } from "react";
import { Button, CATEGORY_META, Icon, IconButton, cx } from "../../components/ui";
import { api, errorMessage } from "../../lib/ipc";
import { useVaultStore } from "../../store/vaultStore";
import type { Category, View } from "../../types";
import { BackupPanel } from "../backup/BackupPanel";
import { GeneratorPanel } from "../generator/GeneratorPanel";
import { SettingsPanel } from "../settings/SettingsPanel";
import { ItemCard } from "./ItemCard";

const NAV: Array<{ key: View; label: string; icon: "key" | "generator" | "download" | "settings" }> = [
  { key: "items", label: "Хранилище", icon: "key" },
  { key: "generator", label: "Генератор", icon: "generator" },
  { key: "backup", label: "Бэкапы", icon: "download" },
  { key: "settings", label: "Настройки", icon: "settings" },
];

const isMac =
  typeof navigator !== "undefined" && /Mac|iPhone|iPad/.test(navigator.platform || navigator.userAgent);
const searchShortcut = isMac ? "⌘ K" : "Ctrl K";

function Keycap({ children }: { children: string }) {
  return (
    <span className="mono rounded border border-[var(--void-line)] px-2 py-1 text-xs text-[var(--void-text-muted)]">
      {children}
    </span>
  );
}

export function VaultScreen() {
  const status = useVaultStore((state) => state.status);
  const items = useVaultStore((state) => state.items);
  const query = useVaultStore((state) => state.query);
  const view = useVaultStore((state) => state.view);
  const busy = useVaultStore((state) => state.busy);
  const clipboardLeft = useVaultStore((state) => state.clipboardLeft);
  const setQuery = useVaultStore((state) => state.setQuery);
  const setView = useVaultStore((state) => state.setView);
  const refresh = useVaultStore((state) => state.refresh);
  const openNew = useVaultStore((state) => state.openNew);
  const openItem = useVaultStore((state) => state.openItem);
  const toggleFavorite = useVaultStore((state) => state.toggleFavorite);
  const lock = useVaultStore((state) => state.lock);
  const copy = useVaultStore((state) => state.copy);
  const notify = useVaultStore((state) => state.notify);

  const [category, setCategory] = useState<Category | "all">("all");
  const searchRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    function onKeyDown(event: KeyboardEvent) {
      const mod = event.ctrlKey || event.metaKey;
      if (!mod) return;
      const key = event.key.toLowerCase();
      if (key === "k") {
        event.preventDefault();
        setView("items");
        window.setTimeout(() => searchRef.current?.focus(), 0);
      } else if (key === "n") {
        event.preventDefault();
        openNew();
      } else if (key === "l") {
        event.preventDefault();
        void lock();
      }
    }
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [lock, openNew, setView]);

  useEffect(() => {
    if (!status?.unlocked) return;
    const timer = window.setTimeout(() => void refresh(true), 180);
    return () => window.clearTimeout(timer);
  }, [query, refresh, status?.unlocked]);

  useEffect(() => {
    if (view !== "items" || !status?.unlocked) return;
    const timer = window.setInterval(() => void refresh(true), 1000);
    return () => window.clearInterval(timer);
  }, [view, refresh, status?.unlocked]);

  const filtered = useMemo(
    () => (category === "all" ? items : items.filter((item) => item.category === category)),
    [items, category],
  );

  async function copyPassword(id: string) {
    try {
      const item = await api.getItem(id);
      if (item.password) await copy(item.password);
      else notify("Пароль не задан", "error");
    } catch (error) {
      notify(errorMessage(error), "error");
    }
  }

  async function copyTotp(id: string) {
    const item = items.find((entry) => entry.id === id);
    if (item?.totp?.code) await copy(item.totp.code);
  }

  return (
    <div className="safe-pad mx-auto min-h-screen max-w-[1500px] pb-28 lg:pb-12">
      <header className="flex items-center justify-between gap-4 border-b border-[var(--void-line)] pb-4">
        <div className="flex items-center gap-3">
          <img src="/logo.png" alt="" className="h-8 w-8 rounded-lg border border-[var(--void-line)] object-cover" />
          <div className="flex flex-col gap-1">
            <span className="text-sm text-[var(--void-text)]">VoidVault</span>
            <span className="mono text-xs text-[var(--void-text-muted)]">офлайн · зашифровано</span>
          </div>
        </div>
        <div className="flex items-center gap-3">
          {clipboardLeft > 0 && (
            <span className="mono hidden text-xs text-[var(--void-text-muted)] sm:inline">
              буфер очистится через {clipboardLeft}с
            </span>
          )}
          <div className="hidden items-center gap-2 sm:flex">
            <Keycap>{isMac ? "⌘ L" : "Ctrl L"}</Keycap>
          </div>
          <IconButton label="Заблокировать" onClick={() => void lock()}>
            <Icon name="lock" />
          </IconButton>
        </div>
      </header>

      <div className="mt-6 grid gap-6 lg:grid-cols-[240px_minmax(0,1fr)]">
        <aside className="hidden flex-col gap-8 lg:flex">
          <nav className="flex flex-col gap-1">
            {NAV.map((entry) => (
              <button
                key={entry.key}
                onClick={() => setView(entry.key)}
                className={cx(
                  "flex h-9 items-center gap-3 rounded-lg px-3 text-left text-sm transition-colors",
                  view === entry.key
                    ? "bg-[var(--void-raised)] text-[var(--void-text)]"
                    : "text-[var(--void-text-muted)] hover:bg-[var(--void-surface)] hover:text-[var(--void-text-secondary)]",
                )}
              >
                <Icon name={entry.icon} />
                {entry.label}
              </button>
            ))}
          </nav>

          <div className="flex flex-col gap-1">
            <div className="px-3 pb-2 text-xs text-[var(--void-text-muted)]">Категории</div>
            <button
              onClick={() => setCategory("all")}
              className={cx(
                "flex h-9 items-center rounded-lg px-3 text-left text-sm transition-colors",
                category === "all"
                  ? "bg-[var(--void-raised)] text-[var(--void-text)]"
                  : "text-[var(--void-text-muted)] hover:bg-[var(--void-surface)] hover:text-[var(--void-text-secondary)]",
              )}
            >
              Все записи · {items.length}
            </button>
            {(Object.keys(CATEGORY_META) as Category[]).map((key) => (
              <button
                key={key}
                onClick={() => setCategory(key)}
                className={cx(
                  "flex h-9 items-center rounded-lg px-3 text-left text-sm transition-colors",
                  category === key
                    ? "bg-[var(--void-raised)] text-[var(--void-text)]"
                    : "text-[var(--void-text-muted)] hover:bg-[var(--void-surface)] hover:text-[var(--void-text-secondary)]",
                )}
              >
                {CATEGORY_META[key].label}
              </button>
            ))}
          </div>

          <div className="mt-auto rounded-lg border border-[var(--void-line)] bg-[var(--void-inset)] p-4">
            <div className="flex items-center gap-2 text-xs text-[var(--void-text-secondary)]">
              <Icon name="shield" />
              Состояние хранилища
            </div>
            <div className="mono mt-2 text-xs text-[var(--void-text-muted)]">
              {status?.itemCount ?? 0} записей · блокировка через {status?.autoLockMinutes ?? 5} мин
            </div>
          </div>
        </aside>

        <main className="min-w-0">
          {view === "items" && (
            <>
              <div className="flex flex-col gap-3 sm:flex-row sm:items-center">
                <div className="relative flex-1">
                  <Icon
                    name="search"
                    className="pointer-events-none absolute left-3 top-1/2 -translate-y-1/2 text-[var(--void-text-muted)]"
                  />
                  <input
                    ref={searchRef}
                    value={query}
                    onChange={(event) => setQuery(event.target.value)}
                    placeholder="Поиск"
                    className="h-10 w-full rounded-lg border border-[var(--void-line)] bg-[var(--void-inset)] pl-10 pr-20 text-sm text-[var(--void-text)] placeholder:text-[var(--void-text-muted)] focus:border-[var(--void-line-strong)] focus:outline-none"
                  />
                  <span className="pointer-events-none absolute right-3 top-1/2 -translate-y-1/2">
                    <Keycap>{searchShortcut}</Keycap>
                  </span>
                </div>
                <Button variant="primary" onClick={openNew}>
                  <Icon name="plus" />
                  Новая запись
                </Button>
              </div>

              <div className="mt-4 flex gap-2 overflow-x-auto pb-1 lg:hidden">
                <button
                  onClick={() => setCategory("all")}
                  className={cx(
                    "h-8 shrink-0 rounded-lg border px-3 text-xs transition-colors",
                    category === "all"
                      ? "border-[var(--void-line-strong)] bg-[var(--void-raised)] text-[var(--void-text)]"
                      : "border-[var(--void-line)] text-[var(--void-text-muted)]",
                  )}
                >
                  Все
                </button>
                {(Object.keys(CATEGORY_META) as Category[]).map((key) => (
                  <button
                    key={key}
                    onClick={() => setCategory(key)}
                    className={cx(
                      "h-8 shrink-0 rounded-lg border px-3 text-xs transition-colors",
                      category === key
                        ? "border-[var(--void-line-strong)] bg-[var(--void-raised)] text-[var(--void-text)]"
                        : "border-[var(--void-line)] text-[var(--void-text-muted)]",
                    )}
                  >
                    {CATEGORY_META[key].label}
                  </button>
                ))}
              </div>

              <div className="mt-6 grid gap-4 sm:grid-cols-2 xl:grid-cols-3">
                <AnimatePresence>
                  {filtered.map((item) => (
                    <ItemCard
                      key={item.id}
                      item={item}
                      onOpen={() => void openItem(item.id)}
                      onCopyPassword={() => void copyPassword(item.id)}
                      onCopyTotp={() => void copyTotp(item.id)}
                      onToggleFavorite={() => void toggleFavorite(item.id)}
                    />
                  ))}
                </AnimatePresence>
              </div>

              {!busy && filtered.length === 0 && (
                <div className="glass mt-6 flex flex-col items-center rounded-xl px-6 py-12 text-center">
                  <Icon name="key" className="text-[var(--void-text-muted)]" />
                  <p className="mt-4 text-sm text-[var(--void-text)]">Записей не найдено</p>
                  <p className="mt-2 text-xs text-[var(--void-text-muted)]">
                    Создайте первую запись или измените запрос
                  </p>
                  <Button size="sm" className="mt-4" onClick={openNew}>
                    <Icon name="plus" />
                    Создать запись
                  </Button>
                </div>
              )}
            </>
          )}

          {view !== "items" && (
            <AnimatePresence mode="wait">
              <motion.div
                key={view}
                initial={{ opacity: 0 }}
                animate={{ opacity: 1 }}
                exit={{ opacity: 0 }}
                transition={{ duration: 0.16, ease: "easeOut" }}
              >
                {view === "generator" && <GeneratorPanel />}
                {view === "backup" && <BackupPanel />}
                {view === "settings" && <SettingsPanel />}
              </motion.div>
            </AnimatePresence>
          )}
        </main>
      </div>

      <nav className="glass fixed bottom-[calc(var(--safe-bottom)+0.75rem)] left-1/2 z-40 flex w-[min(92vw,28rem)] -translate-x-1/2 justify-between gap-1 rounded-xl px-2 py-2 lg:hidden">
        {NAV.map((entry) => (
          <button
            key={entry.key}
            onClick={() => setView(entry.key)}
            className={cx(
              "flex h-12 flex-1 flex-col items-center justify-center gap-1 rounded-lg text-xs transition-colors",
              view === entry.key
                ? "bg-[var(--void-raised)] text-[var(--void-text)]"
                : "text-[var(--void-text-muted)]",
            )}
          >
            <Icon name={entry.icon} />
            {entry.label}
          </button>
        ))}
      </nav>
    </div>
  );
}
