import { useEffect, useState } from "react";
import { open, save } from "@tauri-apps/plugin-dialog";
import { Button, Field, Icon, TextInput, selectClass, textareaClass, cx } from "../../components/ui";
import { api, errorMessage } from "../../lib/ipc";
import { useVaultStore } from "../../store/vaultStore";
import type { ImportResult } from "../../types";

export function BackupPanel() {
  const copy = useVaultStore((state) => state.copy);
  const notify = useVaultStore((state) => state.notify);
  const refresh = useVaultStore((state) => state.refresh);

  const [exported, setExported] = useState("");
  const [incoming, setIncoming] = useState("");
  const [mode, setMode] = useState<"merge" | "replace">("merge");
  const [backupPassword, setBackupPassword] = useState("");
  const [authPassword, setAuthPassword] = useState("");
  const [authorizedUntil, setAuthorizedUntil] = useState(0);
  const [now, setNow] = useState(() => Date.now());
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    if (!authorizedUntil) return;
    const timer = window.setInterval(() => setNow(Date.now()), 1000);
    return () => window.clearInterval(timer);
  }, [authorizedUntil]);

  const authorized = authorizedUntil > now;
  const secondsLeft = Math.max(0, Math.ceil((authorizedUntil - now) / 1000));

  async function authorize() {
    setBusy(true);
    try {
      const seconds = await api.authorizeBackup(authPassword);
      setAuthorizedUntil(Date.now() + seconds * 1000);
      setNow(Date.now());
      setAuthPassword("");
      notify(`Бэкапы разрешены на ${seconds} секунд`);
    } catch (error) {
      notify(errorMessage(error), "error");
    } finally {
      setBusy(false);
    }
  }

  function showResult(result: ImportResult) {
    notify(`Импортировано: ${result.imported}, пропущено: ${result.skipped}`);
    void refresh(true);
  }

  async function doExport() {
    setBusy(true);
    try {
      setExported(await api.exportBackup());
      notify("Бэкап сформирован");
    } catch (error) {
      notify(errorMessage(error), "error");
    } finally {
      setBusy(false);
    }
  }

  async function saveFile() {
    setBusy(true);
    try {
      const path = await save({
        title: "Сохранить VoidVault backup",
        defaultPath: "voidvault-backup.vault",
        filters: [{ name: "VoidVault backup", extensions: ["vault"] }],
      });
      if (!path) return;
      if (!exported) setExported(await api.exportBackup());
      await api.exportBackupFile(path);
      notify("Файл бэкапа сохранён");
    } catch (error) {
      notify(errorMessage(error), "error");
    } finally {
      setBusy(false);
    }
  }

  async function importText() {
    setBusy(true);
    try {
      showResult(await api.importBackup(incoming, mode, backupPassword));
      setIncoming("");
    } catch (error) {
      notify(errorMessage(error), "error");
    } finally {
      setBusy(false);
    }
  }

  async function importFile() {
    setBusy(true);
    try {
      const path = await open({
        title: "Выбрать VoidVault backup",
        multiple: false,
        filters: [{ name: "VoidVault backup", extensions: ["vault", "txt"] }],
      });
      if (!path) return;
      showResult(await api.importBackupFile(path, mode, backupPassword));
    } catch (error) {
      notify(errorMessage(error), "error");
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="mx-auto w-full max-w-4xl">
      <div className="flex flex-col gap-1">
        <h2 className="text-xl text-[var(--void-text)]">Бэкапы</h2>
        <p className="text-sm text-[var(--void-text-muted)]">
          Формат VOIDVAULT 1 — зашифрованный JSON, только локально
        </p>
      </div>

      <section className="glass mt-6 rounded-xl p-6">
        <div className="flex items-center gap-2">
          <Icon name="lock" className="text-[var(--void-text-muted)]" />
          <h3 className="text-sm text-[var(--void-text)]">Подтверждение</h3>
        </div>
        <p className="mt-2 text-xs leading-relaxed text-[var(--void-text-secondary)]">
          Экспорт и импорт требуют повторного мастер-пароля. Разрешение действует две минуты и сбрасывается при блокировке.
        </p>
        <div className="mt-4 flex flex-col gap-4 sm:flex-row sm:items-end">
          <div className="flex-1">
            <Field label="Мастер-пароль">
              <TextInput
                type="password"
                value={authPassword}
                onChange={(event) => setAuthPassword(event.target.value)}
                onKeyDown={(event) => {
                  if (event.key === "Enter" && !busy) void authorize();
                }}
                placeholder="Введите мастер-пароль"
                autoComplete="current-password"
              />
            </Field>
          </div>
          <Button variant="primary" onClick={() => void authorize()} disabled={busy || !authPassword}>
            Подтвердить
          </Button>
        </div>
        {authorized && (
          <p className="mono mt-4 text-xs text-[var(--void-text-secondary)]">
            Разрешено ещё {secondsLeft}с
          </p>
        )}
      </section>

      <div className="mt-4 grid gap-4 lg:grid-cols-2">
        <section className="glass rounded-xl p-6">
          <div className="flex items-center gap-2">
            <Icon name="download" className="text-[var(--void-text-muted)]" />
            <h3 className="text-sm text-[var(--void-text)]">Экспорт</h3>
          </div>
          <p className="mt-2 text-xs leading-relaxed text-[var(--void-text-secondary)]">
            Весь хранимый контент шифруется вашим мастер-ключом и упаковывается в текстовый файл.
          </p>
          <div className="mt-4 flex flex-wrap gap-2">
            <Button variant="primary" size="sm" onClick={() => void doExport()} disabled={busy || !authorized}>
              Сформировать
            </Button>
            <Button size="sm" onClick={() => void saveFile()} disabled={busy || !authorized}>
              <Icon name="download" />
              В файл
            </Button>
            <Button size="sm" onClick={() => void copy(exported)} disabled={!exported || busy || !authorized}>
              <Icon name="copy" />
              В буфер
            </Button>
          </div>
          <textarea
            readOnly
            value={exported}
            placeholder="VOIDVAULT 1"
            className={cx(textareaClass, "mono mt-4 h-48 resize-none text-xs")}
          />
        </section>

        <section className="glass rounded-xl p-6">
          <div className="flex items-center gap-2">
            <Icon name="upload" className="text-[var(--void-text-muted)]" />
            <h3 className="text-sm text-[var(--void-text)]">Импорт</h3>
          </div>
          <p className="mt-2 text-xs leading-relaxed text-[var(--void-text-secondary)]">
            При импорте секреты расшифровываются бэкап-ключом и повторно шифруются текущим ключом.
          </p>

          <div className="mt-4 grid gap-4 sm:grid-cols-2">
            <Field label="Режим">
              <select
                value={mode}
                onChange={(event) => setMode(event.target.value as "merge" | "replace")}
                className={selectClass}
              >
                <option value="merge">Объединить</option>
                <option value="replace">Заменить всё</option>
              </select>
            </Field>
            <Field label="Пароль бэкапа" hint="Если отличается от текущего">
              <TextInput
                type="password"
                value={backupPassword}
                onChange={(event) => setBackupPassword(event.target.value)}
                placeholder="Необязательно"
              />
            </Field>
          </div>

          <textarea
            value={incoming}
            onChange={(event) => setIncoming(event.target.value)}
            placeholder="Вставьте содержимое .vault файла"
            className={cx(textareaClass, "mono mt-4 h-32 resize-none text-xs")}
          />

          <div className="mt-4 flex flex-wrap gap-2">
            <Button size="sm" onClick={() => void importFile()} disabled={busy || !authorized}>
              <Icon name="upload" />
              Из файла
            </Button>
            <Button size="sm" onClick={() => void importText()} disabled={!incoming.trim() || busy || !authorized}>
              Импортировать текст
            </Button>
          </div>
        </section>
      </div>
    </div>
  );
}
