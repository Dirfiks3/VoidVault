import { useEffect, useState } from "react";
import { Button, Field, Icon, TextInput, Toggle, cx } from "../../components/ui";
import { api, errorMessage } from "../../lib/ipc";
import { useVaultStore } from "../../store/vaultStore";
import type { GeneratedSecret, PassphraseOptions, PasswordOptions } from "../../types";

export function GeneratorPanel() {
  const copy = useVaultStore((state) => state.copy);
  const notify = useVaultStore((state) => state.notify);

  const [mode, setMode] = useState<"password" | "passphrase">("password");
  const [passwordOptions, setPasswordOptions] = useState<PasswordOptions>({
    length: 24,
    uppercase: true,
    lowercase: true,
    digits: true,
    symbols: true,
    excludeSimilar: true,
  });
  const [passphraseOptions, setPassphraseOptions] = useState<PassphraseOptions>({
    words: 6,
    separator: "-",
    capitalize: false,
    appendDigit: true,
  });
  const [result, setResult] = useState<GeneratedSecret | null>(null);
  const [busy, setBusy] = useState(false);

  async function generate() {
    setBusy(true);
    try {
      const value =
        mode === "password"
          ? await api.generatePassword(passwordOptions)
          : await api.generatePassphrase(passphraseOptions);
      setResult(value);
    } catch (error) {
      notify(errorMessage(error), "error");
    } finally {
      setBusy(false);
    }
  }

  useEffect(() => {
    void generate();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [mode, passwordOptions, passphraseOptions]);

  const strength = result ? Math.min(100, (result.entropyBits / 128) * 100) : 0;

  return (
    <div className="mx-auto w-full max-w-3xl">
      <div className="flex flex-col gap-1">
        <h2 className="text-xl text-[var(--void-text)]">Генератор</h2>
        <p className="text-sm text-[var(--void-text-muted)]">
          Криптографически стойкие секреты, создаваемые в Rust
        </p>
      </div>

      <div className="glass mt-6 rounded-xl p-4 sm:p-6">
        <div className="flex gap-1 rounded-lg border border-[var(--void-line)] bg-[var(--void-inset)] p-1">
          {(["password", "passphrase"] as const).map((value) => (
            <button
              key={value}
              onClick={() => setMode(value)}
              className={cx(
                "h-9 flex-1 rounded-lg text-sm transition-colors",
                mode === value
                  ? "bg-[var(--void-raised)] text-[var(--void-text)]"
                  : "text-[var(--void-text-muted)] hover:text-[var(--void-text-secondary)]",
              )}
            >
              {value === "password" ? "Пароль" : "Пассфраза"}
            </button>
          ))}
        </div>

        <div className="mt-6 rounded-lg border border-[var(--void-line)] bg-[var(--void-inset)] p-4">
          <div className="flex items-center justify-between gap-3">
            <div className="mono min-w-0 flex-1 break-all text-base text-[var(--void-text)] sm:text-xl">
              {result?.value ?? "—"}
            </div>
            <div className="flex shrink-0 gap-2">
              <Button size="sm" onClick={() => void generate()} aria-label="Сгенерировать заново">
                <Icon name="refresh" />
              </Button>
              <Button
                size="sm"
                variant="primary"
                disabled={!result}
                onClick={() => void copy(result!.value)}
                aria-label="Скопировать секрет"
              >
                <Icon name="copy" />
              </Button>
            </div>
          </div>
          <div className="mt-4 flex items-center gap-3">
            <div className="h-1 flex-1 overflow-hidden rounded-full bg-[var(--void-raised)]">
              <div
                className="h-full w-full origin-left rounded-full bg-[var(--void-text-secondary)] transition-transform duration-200 ease-linear"
                style={{ transform: `scaleX(${strength / 100})` }}
              />
            </div>
            <span className="mono whitespace-nowrap text-xs text-[var(--void-text-muted)]">
              {result ? `${Math.round(result.entropyBits)} бит` : "—"}
            </span>
          </div>
        </div>

        <div className="mt-6 grid gap-4 sm:grid-cols-2">
          {mode === "password" ? (
            <>
              <Field label="Длина" hint={`${passwordOptions.length} символов`}>
                <input
                  type="range"
                  min={8}
                  max={64}
                  value={passwordOptions.length}
                  onChange={(event) =>
                    setPasswordOptions((current) => ({ ...current, length: Number(event.target.value) }))
                  }
                  className="h-10 w-full"
                />
              </Field>
              <div className="flex flex-col gap-3 rounded-lg border border-[var(--void-line)] bg-[var(--void-inset)] p-4">
                <Toggle label="Верхний регистр" checked={passwordOptions.uppercase} onChange={(v) => setPasswordOptions((c) => ({ ...c, uppercase: v }))} />
                <Toggle label="Нижний регистр" checked={passwordOptions.lowercase} onChange={(v) => setPasswordOptions((c) => ({ ...c, lowercase: v }))} />
                <Toggle label="Цифры" checked={passwordOptions.digits} onChange={(v) => setPasswordOptions((c) => ({ ...c, digits: v }))} />
                <Toggle label="Символы" checked={passwordOptions.symbols} onChange={(v) => setPasswordOptions((c) => ({ ...c, symbols: v }))} />
                <Toggle label="Исключать похожие (0/O, 1/l)" checked={passwordOptions.excludeSimilar} onChange={(v) => setPasswordOptions((c) => ({ ...c, excludeSimilar: v }))} />
              </div>
            </>
          ) : (
            <>
              <Field label="Количество слов" hint={`${passphraseOptions.words} слов`}>
                <input
                  type="range"
                  min={3}
                  max={12}
                  value={passphraseOptions.words}
                  onChange={(event) =>
                    setPassphraseOptions((current) => ({ ...current, words: Number(event.target.value) }))
                  }
                  className="h-10 w-full"
                />
              </Field>
              <Field label="Разделитель">
                <TextInput
                  value={passphraseOptions.separator}
                  onChange={(event) => setPassphraseOptions((current) => ({ ...current, separator: event.target.value }))}
                  placeholder="-"
                />
              </Field>
              <div className="flex flex-col gap-3 rounded-lg border border-[var(--void-line)] bg-[var(--void-inset)] p-4 sm:col-span-2">
                <Toggle label="С заглавной буквы" checked={passphraseOptions.capitalize} onChange={(v) => setPassphraseOptions((c) => ({ ...c, capitalize: v }))} />
                <Toggle label="Добавить цифру" checked={passphraseOptions.appendDigit} onChange={(v) => setPassphraseOptions((c) => ({ ...c, appendDigit: v }))} />
              </div>
            </>
          )}
        </div>

        {busy && <p className="mono mt-4 text-xs text-[var(--void-text-muted)]">Генерация...</p>}
      </div>
    </div>
  );
}
