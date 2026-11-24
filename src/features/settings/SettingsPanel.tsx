import { Button, Icon, cx } from "../../components/ui";
import { api } from "../../lib/ipc";
import { useVaultStore } from "../../store/vaultStore";

const OPTIONS = [1, 5, 15];

export function SettingsPanel() {
  const status = useVaultStore((state) => state.status);
  const setAutoLock = useVaultStore((state) => state.setAutoLock);
  const lock = useVaultStore((state) => state.lock);
  const notify = useVaultStore((state) => state.notify);
  const clipboardLeft = useVaultStore((state) => state.clipboardLeft);

  return (
    <div className="mx-auto w-full max-w-3xl">
      <div className="flex flex-col gap-1">
        <h2 className="text-xl text-[var(--void-text)]">Настройки</h2>
        <p className="text-sm text-[var(--void-text-muted)]">Локальные параметры хранилища</p>
      </div>

      <div className="mt-6 grid gap-4">
        <section className="glass rounded-xl p-6">
          <div className="flex items-center gap-2">
            <Icon name="clock" className="text-[var(--void-text-muted)]" />
            <h3 className="text-sm text-[var(--void-text)]">Автоблокировка</h3>
          </div>
          <p className="mt-2 text-xs text-[var(--void-text-secondary)]">
            Хранилище блокируется при бездействии и при сворачивании окна.
          </p>
          <div className="mt-4 grid grid-cols-3 gap-2">
            {OPTIONS.map((minutes) => (
              <button
                key={minutes}
                onClick={() => void setAutoLock(minutes)}
                className={cx(
                  "h-10 rounded-lg border text-sm transition-colors",
                  status?.autoLockMinutes === minutes
                    ? "border-[var(--void-line-strong)] bg-[var(--void-raised)] text-[var(--void-text)]"
                    : "border-[var(--void-line)] bg-[var(--void-inset)] text-[var(--void-text-muted)] hover:text-[var(--void-text-secondary)]",
                )}
              >
                {minutes} мин
              </button>
            ))}
          </div>
        </section>

        <section className="glass rounded-xl p-6">
          <div className="flex items-center gap-2">
            <Icon name="shield" className="text-[var(--void-text-muted)]" />
            <h3 className="text-sm text-[var(--void-text)]">Безопасность</h3>
          </div>
          <dl className="mono mt-4 grid grid-cols-2 gap-y-3 text-xs">
            <dt className="text-[var(--void-text-muted)]">KDF</dt>
            <dd className="text-right text-[var(--void-text-secondary)]">{status?.kdf?.algorithm ?? "—"}</dd>
            <dt className="text-[var(--void-text-muted)]">Память</dt>
            <dd className="text-right text-[var(--void-text-secondary)]">
              {status?.kdf ? `${Math.round(status.kdf.memoryKib / 1024)} MiB` : "—"}
            </dd>
            <dt className="text-[var(--void-text-muted)]">Итерации</dt>
            <dd className="text-right text-[var(--void-text-secondary)]">{status?.kdf?.iterations ?? "—"}</dd>
            <dt className="text-[var(--void-text-muted)]">Шифрование</dt>
            <dd className="text-right text-[var(--void-text-secondary)]">AES-256-GCM</dd>
            <dt className="text-[var(--void-text-muted)]">Записей</dt>
            <dd className="text-right text-[var(--void-text-secondary)]">{status?.itemCount ?? 0}</dd>
          </dl>
          <div className="mt-4 flex flex-wrap gap-2">
            <Button variant="danger" size="sm" onClick={() => void lock()}>
              <Icon name="lock" />
              Заблокировать сейчас
            </Button>
            <Button
              size="sm"
              onClick={async () => {
                await api.clearClipboard();
                notify("Буфер очищен");
              }}
              disabled={!clipboardLeft}
            >
              Очистить буфер {clipboardLeft ? `(${clipboardLeft}с)` : ""}
            </Button>
          </div>
        </section>

        <section className="glass rounded-xl p-6">
          <div className="flex items-center gap-2">
            <Icon name="offline" className="text-[var(--void-text-muted)]" />
            <h3 className="text-sm text-[var(--void-text)]">О приложении</h3>
          </div>
          <p className="mt-3 text-xs leading-relaxed text-[var(--void-text-secondary)]">
            VoidVault 0.1.0. Данные хранятся только в локальной SQLite-базе приложения.
            Никаких аккаунтов, синхронизации, облака и телеметрии.
          </p>
          <div className="mt-4 flex items-center gap-3 border-t border-[var(--void-line)] pt-4">
            <img src="/logo.png" alt="" className="h-8 w-8 rounded-lg border border-[var(--void-line)] object-cover" />
            <span className="mono text-xs text-[var(--void-text-muted)]">VoidVault · только локально</span>
          </div>
        </section>
      </div>
    </div>
  );
}
