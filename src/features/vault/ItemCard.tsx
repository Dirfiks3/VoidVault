import { motion } from "motion/react";
import { CATEGORY_META, Icon, IconButton, cx } from "../../components/ui";
import type { ItemSummary } from "../../types";

const categoryIcon = {
  login: "key",
  card: "card",
  note: "note",
  apiKey: "api",
  ssh: "ssh",
} as const;

interface Props {
  item: ItemSummary;
  onOpen: () => void;
  onCopyPassword: () => void;
  onCopyTotp: () => void;
  onToggleFavorite: () => void;
}

export function ItemCard({ item, onOpen, onCopyPassword, onCopyTotp, onToggleFavorite }: Props) {
  const period = item.totp?.period ?? 30;
  const remaining = item.totp?.secondsRemaining ?? 0;
  const countdown = period > 0 ? remaining / period : 0;
  const code = item.totp?.code ?? "";

  return (
    <motion.article
      initial={{ opacity: 0, y: 8 }}
      animate={{ opacity: 1, y: 0 }}
      exit={{ opacity: 0 }}
      transition={{ duration: 0.16, ease: "easeOut" }}
      className="glass group relative flex min-h-48 flex-col rounded-xl p-4"
    >
      <div className="flex items-start gap-3">
        <div className="flex h-10 w-10 shrink-0 items-center justify-center rounded-lg border border-[var(--void-line)] bg-[var(--void-inset)] text-[var(--void-text-secondary)]">
          <Icon name={categoryIcon[item.category]} />
        </div>
        <div className="min-w-0 flex-1">
          <button onClick={onOpen} className="block max-w-full truncate text-left text-sm text-[var(--void-text)]">
            {item.title || "Без названия"}
          </button>
          <p className="mt-1 truncate text-xs text-[var(--void-text-muted)]">
            {item.service || CATEGORY_META[item.category].label}
          </p>
        </div>
        <IconButton
          label={item.favorite ? "Убрать из избранного" : "В избранное"}
          onClick={onToggleFavorite}
          className={cx(item.favorite && "text-[var(--void-text)]")}
        >
          <Icon name="star" className={cx(item.favorite && "fill-current")} />
        </IconButton>
      </div>

      {item.username && (
        <p className="mono mt-4 truncate text-xs text-[var(--void-text-secondary)]">{item.username}</p>
      )}

      {item.tags.length > 0 && (
        <div className="mt-3 flex flex-wrap gap-2">
          {item.tags.slice(0, 3).map((tag) => (
            <span
              key={tag}
              className="rounded border border-[var(--void-line)] px-2 py-1 text-xs text-[var(--void-text-muted)]"
            >
              {tag}
            </span>
          ))}
        </div>
      )}

      <div className="mt-auto pt-4">
        {item.totp ? (
          <div className="rounded-lg border border-[var(--void-line)] bg-[var(--void-inset)] p-4">
            <div className="flex items-center justify-between gap-2">
              <div>
                <div className="text-xs text-[var(--void-text-muted)]">2FA-код</div>
                <div className="mono mt-1 text-xl text-[var(--void-text)]">
                  {code.slice(0, 3)} {code.slice(3)}
                </div>
              </div>
              <IconButton label="Копировать 2FA-код" onClick={onCopyTotp} disabled={!code}>
                <Icon name="copy" />
              </IconButton>
            </div>
            <div className="mt-3 h-1 w-full overflow-hidden rounded-full bg-[var(--void-raised)]">
              <div
                className="h-full w-full origin-left rounded-full bg-[var(--void-accent)] transition-transform duration-200 ease-linear"
                style={{ transform: `scaleX(${countdown})` }}
              />
            </div>
            <div className="mono mt-2 text-right text-xs text-[var(--void-text-muted)]">
              {remaining}с
            </div>
          </div>
        ) : (
          <div className="flex gap-2">
            {item.hasPassword && (
              <button
                onClick={onCopyPassword}
                className="flex h-9 flex-1 items-center justify-center gap-2 rounded-lg border border-[var(--void-line)] bg-[var(--void-inset)] text-xs text-[var(--void-text-secondary)] transition-colors hover:border-[var(--void-line-strong)] hover:text-[var(--void-text)]"
              >
                <Icon name="copy" />
                Пароль
              </button>
            )}
            <button
              onClick={onOpen}
              className="flex h-9 flex-1 items-center justify-center gap-2 rounded-lg border border-[var(--void-line)] bg-[var(--void-inset)] text-xs text-[var(--void-text-secondary)] transition-colors hover:border-[var(--void-line-strong)] hover:text-[var(--void-text)]"
            >
              <Icon name="edit" />
              Открыть
            </button>
          </div>
        )}
      </div>

      {item.totp && item.hasPassword && (
        <button
          onClick={onCopyPassword}
          className="mono mt-3 flex items-center justify-center gap-2 text-xs text-[var(--void-text-muted)] transition-colors hover:text-[var(--void-text-secondary)]"
        >
          <Icon name="copy" />
          Пароль
        </button>
      )}
    </motion.article>
  );
}
