import { motion } from "motion/react";
import { useMemo, useState } from "react";
import { Button, Icon, TextInput } from "../../components/ui";
import { errorMessage } from "../../lib/ipc";
import { useVaultStore } from "../../store/vaultStore";

function entropyBits(password: string): number {
  if (!password) return 0;
  let pool = 0;
  if (/[a-z]/.test(password)) pool += 26;
  if (/[A-Z]/.test(password)) pool += 26;
  if (/[0-9]/.test(password)) pool += 10;
  if (/[^A-Za-z0-9]/.test(password)) pool += 32;
  return password.length * (pool > 0 ? Math.log2(pool) : 0);
}

function strengthLabel(bits: number): string {
  if (bits < 45) return "Слабый";
  if (bits < 65) return "Достаточный";
  if (bits < 90) return "Сильный";
  return "Очень сильный";
}

export function LockScreen() {
  const status = useVaultStore((state) => state.status);
  const createVault = useVaultStore((state) => state.createVault);
  const unlock = useVaultStore((state) => state.unlock);

  const [password, setPassword] = useState("");
  const [confirm, setConfirm] = useState("");
  const [visible, setVisible] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");

  const bits = useMemo(() => entropyBits(password), [password]);
  const isCreate = !status?.exists;
  const strength = Math.min(100, bits);

  async function submit() {
    setError("");
    if (password.length < 8) {
      setError("Минимум 8 символов");
      return;
    }
    if (isCreate && password !== confirm) {
      setError("Пароли не совпадают");
      return;
    }
    setBusy(true);
    try {
      if (isCreate) await createVault(password);
      else await unlock(password);
      setPassword("");
      setConfirm("");
    } catch (caught) {
      setError(errorMessage(caught));
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="flex min-h-full items-center justify-center px-5 py-16 sm:px-8">
      <motion.div
        initial={{ opacity: 0, y: 8 }}
        animate={{ opacity: 1, y: 0 }}
        transition={{ duration: 0.2, ease: "easeOut" }}
        className="glass w-full max-w-md rounded-xl p-6 sm:p-8"
      >
        <div className="flex flex-col items-center text-center">
          <img src="/logo.png" alt="VoidVault" className="h-12 w-12 rounded-lg border border-[var(--void-line)] object-cover" />
          <h1 className="mt-4 text-2xl text-[var(--void-text)]">VoidVault</h1>
          <p className="mt-2 max-w-xs text-sm leading-relaxed text-[var(--void-text-secondary)]">
            Локальное хранилище паролей и 2FA. Без облака, аккаунтов и телеметрии.
          </p>
        </div>

        <div className="mt-6 flex items-center gap-2 border-b border-[var(--void-line)] pb-3">
          <Icon name="shield" className="text-[var(--void-text-muted)]" />
          <span className="text-sm text-[var(--void-text-secondary)]">
            {isCreate ? "Новое хранилище" : "Разблокировка"}
          </span>
        </div>

        <div className="mt-6 flex flex-col gap-4">
          <div className="flex flex-col gap-2">
            <span className="text-xs text-[var(--void-text-secondary)]">Мастер-пароль</span>
            <div className="relative">
              <TextInput
                type={visible ? "text" : "password"}
                value={password}
                onChange={(event) => setPassword(event.target.value)}
                onKeyDown={(event) => {
                  if (event.key === "Enter") void submit();
                }}
                placeholder={isCreate ? "Минимум 8 символов" : "Введите мастер-пароль"}
                autoFocus
                autoComplete={isCreate ? "new-password" : "current-password"}
                className="pr-11"
              />
              <button
                type="button"
                aria-label={visible ? "Скрыть пароль" : "Показать пароль"}
                onClick={() => setVisible((value) => !value)}
                className="absolute right-2 top-1/2 flex h-8 w-8 -translate-y-1/2 items-center justify-center text-[var(--void-text-muted)] transition-colors hover:text-[var(--void-text)]"
              >
                <Icon name={visible ? "eyeOff" : "eye"} />
              </button>
            </div>
          </div>

          {isCreate && (
            <>
              <div className="flex flex-col gap-2">
                <span className="text-xs text-[var(--void-text-secondary)]">Подтверждение</span>
                <TextInput
                  type={visible ? "text" : "password"}
                  value={confirm}
                  onChange={(event) => setConfirm(event.target.value)}
                  onKeyDown={(event) => {
                    if (event.key === "Enter") void submit();
                  }}
                  placeholder="Повторите мастер-пароль"
                  autoComplete="new-password"
                />
              </div>

              <div className="flex flex-col gap-2">
                <div className="flex items-center justify-between text-xs">
                  <span className="text-[var(--void-text-muted)]">Энтропия</span>
                  <span className="mono text-[var(--void-text-secondary)]">
                    {Math.round(bits)} бит · {strengthLabel(bits)}
                  </span>
                </div>
                <div className="h-1 w-full overflow-hidden rounded-full bg-[var(--void-raised)]">
                  <div
                    className="h-full w-full origin-left rounded-full bg-[var(--void-text-secondary)] transition-transform duration-200 ease-linear"
                    style={{ transform: `scaleX(${strength / 100})` }}
                  />
                </div>
              </div>
            </>
          )}

          {error && (
            <p className="flex items-center gap-2 text-sm text-[var(--void-text)]">
              <Icon name="x" />
              {error}
            </p>
          )}

          <Button variant="primary" onClick={() => void submit()} disabled={busy} className="w-full">
            {busy
              ? isCreate
                ? "Создание..."
                : "Разблокировка..."
              : isCreate
                ? "Создать хранилище"
                : "Разблокировать"}
          </Button>
        </div>

        <div className="mt-6 flex items-center justify-center gap-2 text-xs text-[var(--void-text-muted)]">
          <Icon name="offline" />
          Argon2id · AES-256-GCM · локально
        </div>
      </motion.div>
    </div>
  );
}
