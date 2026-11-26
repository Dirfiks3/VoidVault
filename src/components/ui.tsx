import { AnimatePresence, motion } from "motion/react";
import type { ReactNode, ButtonHTMLAttributes, InputHTMLAttributes } from "react";
import type { Category } from "../types";

export function cx(...classes: Array<string | false | null | undefined>): string {
  return classes.filter(Boolean).join(" ");
}

export const CATEGORY_META: Record<Category, { label: string; short: string }> = {
  login: { label: "Логины", short: "LOG" },
  card: { label: "Карты", short: "CRD" },
  note: { label: "Защищённые заметки", short: "NOTE" },
  apiKey: { label: "API-ключи", short: "API" },
  ssh: { label: "SSH-ключи", short: "SSH" },
};

type IconName =
  | "search"
  | "plus"
  | "lock"
  | "copy"
  | "star"
  | "edit"
  | "trash"
  | "key"
  | "card"
  | "note"
  | "api"
  | "ssh"
  | "eye"
  | "eyeOff"
  | "refresh"
  | "download"
  | "upload"
  | "settings"
  | "shield"
  | "x"
  | "check"
  | "clock"
  | "offline"
  | "chevron"
  | "generator";

const paths: Record<IconName, ReactNode> = {
  search: <><circle cx="11" cy="11" r="7" /><path d="m20 20-3.5-3.5" /></>,
  plus: <><path d="M12 5v14" /><path d="M5 12h14" /></>,
  lock: <><rect x="4" y="10" width="16" height="10" rx="2" /><path d="M8 10V7a4 4 0 0 1 8 0v3" /></>,
  copy: <><rect x="9" y="9" width="11" height="11" rx="2" /><path d="M5 15H4a1 1 0 0 1-1-1V4a1 1 0 0 1 1-1h10a1 1 0 0 1 1 1v1" /></>,
  star: <path d="m12 3 2.8 5.7 6.2.9-4.5 4.4 1 6.2-5.5-2.9-5.5 2.9 1-6.2L3 9.6l6.2-.9L12 3Z" />,
  edit: <><path d="M12 20h9" /><path d="M16.5 3.5a2.1 2.1 0 0 1 3 3L7 19l-4 1 1-4Z" /></>,
  trash: <><path d="M3 6h18" /><path d="M8 6V4h8v2" /><path d="M19 6l-1 14H6L5 6" /></>,
  key: <><circle cx="8" cy="15" r="4" /><path d="m10.8 12.2 8-8" /><path d="m17 5 2 2" /><path d="m14 8 2 2" /></>,
  card: <><rect x="2" y="5" width="20" height="14" rx="2" /><path d="M2 10h20" /><path d="M6 15h4" /></>,
  note: <><path d="M6 3h9l4 4v14H6z" /><path d="M14 3v5h5" /><path d="M9 12h7" /><path d="M9 16h7" /></>,
  api: <><path d="m8 8-4 4 4 4" /><path d="m16 8 4 4-4 4" /><path d="m14 5-4 14" /></>,
  ssh: <><rect x="3" y="4" width="18" height="16" rx="2" /><path d="m7 9 3 3-3 3" /><path d="M13 15h4" /></>,
  eye: <><path d="M2 12s3.6-6 10-6 10 6 10 6-3.6 6-10 6-10-6-10-6Z" /><circle cx="12" cy="12" r="3" /></>,
  eyeOff: <><path d="m3 3 18 18" /><path d="M10.6 6.2A10.9 10.9 0 0 1 12 6c6.4 0 10 6 10 6a17 17 0 0 1-3.3 3.7" /><path d="M6.2 7.8A17 17 0 0 0 2 12s3.6 6 10 6a10.7 10.7 0 0 0 4.2-.8" /><path d="M9.9 9.9a3 3 0 0 0 4.2 4.2" /></>,
  refresh: <><path d="M20 7v5h-5" /><path d="M4 17v-5h5" /><path d="M6.1 9a7 7 0 0 1 11.6-2L20 9" /><path d="M17.9 15a7 7 0 0 1-11.6 2L4 15" /></>,
  download: <><path d="M12 3v12" /><path d="m7 11 5 5 5-5" /><path d="M4 20h16" /></>,
  upload: <><path d="M12 17V5" /><path d="m7 9 5-5 5 5" /><path d="M4 20h16" /></>,
  settings: <><circle cx="12" cy="12" r="3" /><path d="M19.4 15a1.7 1.7 0 0 0 .3 1.9l.1.1-2.8 2.8-.1-.1a1.7 1.7 0 0 0-1.9-.3 1.7 1.7 0 0 0-1 1.5V21h-4v-.1a1.7 1.7 0 0 0-1-1.5 1.7 1.7 0 0 0-1.9.3l-.1.1L4.2 17l.1-.1a1.7 1.7 0 0 0 .3-1.9 1.7 1.7 0 0 0-1.5-1H3v-4h.1a1.7 1.7 0 0 0 1.5-1 1.7 1.7 0 0 0-.3-1.9L4.2 7l2.8-2.8.1.1a1.7 1.7 0 0 0 1.9.3 1.7 1.7 0 0 0 1-1.5V3h4v.1a1.7 1.7 0 0 0 1 1.5 1.7 1.7 0 0 0 1.9-.3l.1-.1L19.8 7l-.1.1a1.7 1.7 0 0 0-.3 1.9 1.7 1.7 0 0 0 1.5 1H21v4h-.1a1.7 1.7 0 0 0-1.5 1Z" /></>,
  shield: <><path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10Z" /><path d="m9 12 2 2 4-4" /></>,
  x: <><path d="M18 6 6 18" /><path d="m6 6 12 12" /></>,
  check: <path d="m5 12 5 5L20 7" />,
  clock: <><circle cx="12" cy="12" r="9" /><path d="M12 7v5l3 2" /></>,
  offline: <><path d="m2 2 20 20" /><path d="M8.5 16.5a5 5 0 0 1 7 0" /><path d="M5 12a10 10 0 0 1 3-2" /><path d="M16 8a10 10 0 0 1 3 2" /><path d="M12 20h.01" /></>,
  chevron: <path d="m9 18 6-6-6-6" />,
  generator: <><path d="M9 3 4 12l5 9" /><path d="m15 3 5 9-5 9" /><path d="M12 8v8" /></>,
};

export function Icon({ name, className = "h-4 w-4" }: { name: IconName; className?: string }) {
  return (
    <svg
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.5"
      strokeLinecap="round"
      strokeLinejoin="round"
      className={className}
      aria-hidden="true"
    >
      {paths[name]}
    </svg>
  );
}

type ButtonProps = ButtonHTMLAttributes<HTMLButtonElement> & {
  variant?: "primary" | "ghost" | "danger" | "subtle";
  size?: "sm" | "md";
};

export function Button({ variant = "ghost", size = "md", className, ...props }: ButtonProps) {
  return (
    <button
      {...props}
      className={cx(
        "inline-flex items-center justify-center gap-2 rounded-lg border transition-colors disabled:cursor-not-allowed disabled:opacity-40",
        size === "sm" ? "h-8 px-3 text-xs" : "h-10 px-4 text-sm",
        variant === "primary" && "border-transparent bg-[var(--void-accent)] text-[#080a11] hover:bg-[#a094ff]",
        variant === "ghost" && "border-[var(--void-line)] bg-[var(--void-surface)] text-[var(--void-text)] hover:border-[var(--void-line-strong)] hover:bg-[var(--void-raised)]",
        variant === "danger" && "border-[var(--void-line-strong)] bg-transparent text-[var(--void-text)] hover:bg-white/10",
        variant === "subtle" && "border-transparent bg-transparent text-[var(--void-text-secondary)] hover:bg-white/5 hover:text-[var(--void-text)]",
        className,
      )}
    />
  );
}

export function IconButton({ label, className, ...props }: ButtonHTMLAttributes<HTMLButtonElement> & { label: string }) {
  return (
    <button
      aria-label={label}
      title={label}
      {...props}
      className={cx(
        "inline-flex h-8 w-8 items-center justify-center rounded-lg border border-transparent text-[var(--void-text-muted)] transition-colors hover:border-[var(--void-line)] hover:bg-white/5 hover:text-[var(--void-text)] disabled:opacity-30",
        className,
      )}
    />
  );
}

export function Field({ label, hint, children }: { label: string; hint?: string; children: ReactNode }) {
  return (
    <label className="flex flex-col gap-2">
      <span className="text-xs text-[var(--void-text-secondary)]">{label}</span>
      {children}
      {hint && <span className="text-xs text-[var(--void-text-muted)]">{hint}</span>}
    </label>
  );
}

export const inputClass =
  "h-10 w-full rounded-lg border border-[var(--void-line)] bg-[var(--void-inset)] px-3 text-sm text-[var(--void-text)] placeholder:text-[var(--void-text-muted)] focus:border-[var(--void-accent)] focus:outline-none";

export function SecurityRibbon() {
  const items = [
    { label: "LOCAL ONLY", tone: "accent" },
    { label: "AES-256-GCM", tone: "plain" },
    { label: "CLIPBOARD 30S", tone: "plain" },
    { label: "LOCK ON BLUR", tone: "good" },
  ] as const;

  return (
    <div className="glass mt-5 flex flex-wrap items-center gap-2 rounded-xl px-3 py-3">
      <span className="eyebrow mr-1">SECURITY</span>
      {items.map((item) => (
        <span key={item.label} className={cx("chip", item.tone === "accent" && "chip-accent")}>
          <span
            className={cx(
              "h-1.5 w-1.5 rounded-full",
              item.tone === "accent"
                ? "bg-[var(--void-accent)]"
                : item.tone === "good"
                  ? "bg-[var(--void-good)]"
                  : "bg-[var(--void-text-muted)]",
            )}
          />
          {item.label}
        </span>
      ))}
      <span className="mono ml-auto text-[10px] text-[var(--void-text-muted)]">v0.2.0</span>
    </div>
  );
}

export const selectClass =
  "h-10 w-full rounded-lg border border-[var(--void-line)] bg-[var(--void-inset)] px-3 text-sm text-[var(--void-text)] focus:border-[var(--void-accent)] focus:outline-none";

export const textareaClass =
  "w-full rounded-lg border border-[var(--void-line)] bg-[var(--void-inset)] px-3 py-2 text-sm text-[var(--void-text)] placeholder:text-[var(--void-text-muted)] focus:border-[var(--void-accent)] focus:outline-none";

export function TextInput(props: InputHTMLAttributes<HTMLInputElement>) {
  return <input {...props} className={cx(inputClass, props.className)} />;
}

export function Toggle({ checked, onChange, label }: { checked: boolean; onChange: (value: boolean) => void; label: string }) {
  return (
    <button
      type="button"
      onClick={() => onChange(!checked)}
      className="flex w-full items-center justify-between gap-3 text-left"
    >
      <span className="text-sm text-[var(--void-text-secondary)]">{label}</span>
      <span
        className={cx(
          "relative h-5 w-9 shrink-0 rounded-full border transition-colors",
          checked ? "border-transparent bg-[var(--void-accent)]" : "border-[var(--void-line)] bg-[var(--void-raised)]",
        )}
      >
        <span
          className={cx(
            "absolute left-0.5 top-0.5 h-4 w-4 rounded-full transition-transform",
            checked ? "translate-x-4 bg-[var(--void-bg)]" : "translate-x-0 bg-[var(--void-text-muted)]",
          )}
        />
      </span>
    </button>
  );
}

export function Modal({ open, onClose, children, wide }: { open: boolean; onClose: () => void; children: ReactNode; wide?: boolean }) {
  return (
    <AnimatePresence>
      {open && (
        <motion.div
          className="fixed inset-0 z-50 flex items-end justify-center bg-black/85 p-0 sm:items-center sm:p-6"
          initial={{ opacity: 0 }}
          animate={{ opacity: 1 }}
          exit={{ opacity: 0 }}
          onMouseDown={(event) => {
            if (event.target === event.currentTarget) onClose();
          }}
        >
          <motion.div
            initial={{ opacity: 0, y: 8 }}
            animate={{ opacity: 1, y: 0 }}
            exit={{ opacity: 0, y: 8 }}
            transition={{ duration: 0.2, ease: "easeOut" }}
            className={cx(
              "glass max-h-[92vh] w-full overflow-y-auto rounded-xl safe-pad",
              wide ? "sm:max-w-2xl" : "sm:max-w-lg",
            )}
          >
            {children}
          </motion.div>
        </motion.div>
      )}
    </AnimatePresence>
  );
}
