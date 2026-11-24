import { AnimatePresence, motion } from "motion/react";
import { useEffect, useState } from "react";
import {
  Button,
  CATEGORY_META,
  Field,
  Icon,
  IconButton,
  Modal,
  TextInput,
  Toggle,
  selectClass,
  textareaClass,
  cx,
} from "../../components/ui";
import { api, errorMessage } from "../../lib/ipc";
import { useVaultStore } from "../../store/vaultStore";
import type { Category, ItemInput, VaultItem } from "../../types";

const empty = (): ItemInput => ({
  id: null,
  category: "login",
  title: "",
  service: "",
  username: "",
  password: "",
  totpSecret: "",
  note: "",
  favorite: false,
  tags: [],
});

export function ItemEditor() {
  const editor = useVaultStore((state) => state.editor);
  const closeEditor = useVaultStore((state) => state.closeEditor);
  const saveItem = useVaultStore((state) => state.saveItem);
  const deleteItem = useVaultStore((state) => state.deleteItem);
  const notify = useVaultStore((state) => state.notify);

  const [form, setForm] = useState<ItemInput>(empty);
  const [tagsText, setTagsText] = useState("");
  const [visible, setVisible] = useState(false);
  const [busy, setBusy] = useState(false);
  const [confirmDelete, setConfirmDelete] = useState(false);

  useEffect(() => {
    if (!editor.open) return;
    const item: VaultItem | null = editor.item;
    setForm(
      item
        ? {
            id: item.id,
            category: item.category,
            title: item.title,
            service: item.service,
            username: item.username,
            password: item.password,
            totpSecret: item.totpSecret,
            note: item.note,
            favorite: item.favorite,
            tags: [...item.tags],
          }
        : empty(),
    );
    setTagsText(item?.tags.join(", ") ?? "");
    setVisible(false);
    setConfirmDelete(false);
  }, [editor.open, editor.item]);

  useEffect(() => {
    if (!editor.open) return;
    function onKey(event: KeyboardEvent) {
      if (event.key === "Escape") closeEditor();
    }
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [editor.open, closeEditor]);

  function patch(values: Partial<ItemInput>) {
    setForm((current) => ({ ...current, ...values }));
  }

  async function generate() {
    try {
      const secret = await api.generatePassword({
        length: 24,
        uppercase: true,
        lowercase: true,
        digits: true,
        symbols: true,
        excludeSimilar: true,
      });
      patch({ password: secret.value });
      setVisible(true);
    } catch (error) {
      notify(errorMessage(error), "error");
    }
  }

  async function submit() {
    if (!form.title.trim()) {
      notify("Введите название", "error");
      return;
    }
    setBusy(true);
    try {
      await saveItem({
        ...form,
        title: form.title.trim(),
        tags: tagsText
          .split(",")
          .map((tag) => tag.trim())
          .filter(Boolean),
      });
    } catch {
      // toast is emitted by the store
    } finally {
      setBusy(false);
    }
  }

  return (
    <Modal open={editor.open} onClose={closeEditor} wide>
      <div className="flex items-center justify-between gap-4">
        <div className="flex flex-col gap-1">
          <h2 className="text-xl text-[var(--void-text)]">
            {form.id ? "Редактирование записи" : "Новая запись"}
          </h2>
          <p className="text-xs text-[var(--void-text-muted)]">
            Поля шифруются в Rust перед сохранением в SQLite
          </p>
        </div>
        <IconButton label="Закрыть" onClick={closeEditor}>
          <Icon name="x" />
        </IconButton>
      </div>

      <div className="mt-6 grid gap-4 sm:grid-cols-2">
        <Field label="Категория">
          <select
            value={form.category}
            onChange={(event) => patch({ category: event.target.value as Category })}
            className={selectClass}
          >
            {(Object.keys(CATEGORY_META) as Category[]).map((key) => (
              <option key={key} value={key}>
                {CATEGORY_META[key].label}
              </option>
            ))}
          </select>
        </Field>

        <div className="flex flex-col gap-2">
          <span className="text-xs text-[var(--void-text-secondary)]">Избранное</span>
          <div className="flex h-10 items-center rounded-lg border border-[var(--void-line)] bg-[var(--void-inset)] px-3">
            <Toggle
              checked={form.favorite}
              onChange={(value) => patch({ favorite: value })}
              label="Показывать первым"
            />
          </div>
        </div>

        <Field label="Название">
          <TextInput value={form.title} onChange={(event) => patch({ title: event.target.value })} placeholder="GitHub" autoFocus />
        </Field>

        <Field label="URL / сервис">
          <TextInput value={form.service} onChange={(event) => patch({ service: event.target.value })} placeholder="https://github.com" />
        </Field>

        <Field label="Логин / email">
          <TextInput value={form.username} onChange={(event) => patch({ username: event.target.value })} placeholder="user@example.com" />
        </Field>

        <Field label="Теги" hint="Через запятую">
          <TextInput value={tagsText} onChange={(event) => setTagsText(event.target.value)} placeholder="работа, основное" />
        </Field>

        <div className="sm:col-span-2">
          <Field label="Пароль / секрет">
            <div className="flex gap-2">
              <div className="relative flex-1">
                <TextInput
                  type={visible ? "text" : "password"}
                  value={form.password}
                  onChange={(event) => patch({ password: event.target.value })}
                  className="mono pr-10"
                  placeholder="••••••••••••"
                />
                <button
                  type="button"
                  onClick={() => setVisible((value) => !value)}
                  className="absolute right-2 top-1/2 flex h-8 w-8 -translate-y-1/2 items-center justify-center text-[var(--void-text-muted)] transition-colors hover:text-[var(--void-text)]"
                  aria-label={visible ? "Скрыть пароль" : "Показать пароль"}
                >
                  <Icon name={visible ? "eyeOff" : "eye"} />
                </button>
              </div>
              <Button onClick={() => void generate()} aria-label="Сгенерировать пароль">
                <Icon name="generator" />
              </Button>
            </div>
          </Field>
        </div>

        <div className="sm:col-span-2">
          <Field label="TOTP-секрет (Base32)" hint="Оставьте пустым, если 2FA не используется">
            <TextInput
              value={form.totpSecret}
              onChange={(event) => patch({ totpSecret: event.target.value })}
              className="mono"
              placeholder="JBSWY3DPEHPK3PXP"
              autoComplete="off"
            />
          </Field>
        </div>

        <div className="sm:col-span-2">
          <Field label="Заметка">
            <textarea
              value={form.note}
              onChange={(event) => patch({ note: event.target.value })}
              rows={3}
              className={cx(textareaClass, "resize-y")}
              placeholder="Секретная информация"
            />
          </Field>
        </div>
      </div>

      <AnimatePresence>
        {confirmDelete && (
          <motion.div
            initial={{ opacity: 0, y: 8 }}
            animate={{ opacity: 1, y: 0 }}
            exit={{ opacity: 0, y: 8 }}
            transition={{ duration: 0.16, ease: "easeOut" }}
            className="mt-4"
          >
            <div className="flex flex-wrap items-center justify-between gap-3 rounded-lg border border-[var(--void-line-strong)] bg-[var(--void-raised)] px-4 py-3">
              <span className="text-xs text-[var(--void-text)]">
                Удалить запись без возможности восстановления?
              </span>
              <div className="flex gap-2">
                <Button size="sm" onClick={() => setConfirmDelete(false)}>Отмена</Button>
                <Button size="sm" variant="danger" onClick={() => void deleteItem(form.id!)}>Удалить</Button>
              </div>
            </div>
          </motion.div>
        )}
      </AnimatePresence>

      <div className="mt-6 flex items-center justify-between gap-3 border-t border-[var(--void-line)] pt-4">
        <div>
          {form.id && !confirmDelete && (
            <Button variant="subtle" size="sm" onClick={() => setConfirmDelete(true)}>
              <Icon name="trash" />
              Удалить
            </Button>
          )}
        </div>
        <div className="flex gap-2">
          <Button onClick={closeEditor}>Отмена</Button>
          <Button variant="primary" onClick={() => void submit()} disabled={busy}>
            {busy ? "Сохранение..." : "Сохранить"}
          </Button>
        </div>
      </div>
    </Modal>
  );
}
