import { invoke } from "@tauri-apps/api/core";
import type {
  Category,
  ClipboardStatus,
  GeneratedSecret,
  ImportResult,
  ItemInput,
  ItemSummary,
  PassphraseOptions,
  PasswordOptions,
  PasswordAssessment,
  VaultItem,
  VaultStatus,
} from "../types";

export const api = {
  vaultStatus: () => invoke<VaultStatus>("vault_status"),

  createVault: (masterPassword: string) =>
    invoke<VaultStatus>("create_vault", { masterPassword }),

  unlockVault: (masterPassword: string) =>
    invoke<VaultStatus>("unlock_vault", { masterPassword }),

  lockVault: () => invoke<VaultStatus>("lock_vault"),

  setAutoLock: (minutes: number) =>
    invoke<VaultStatus>("set_auto_lock", { minutes }),

  touchVault: () => invoke<void>("touch_vault"),

  listItems: (
    query?: string,
    category?: Category | "all",
    favoritesOnly = false,
  ) =>
    invoke<ItemSummary[]>("list_items", {
      query: query || null,
      category: category && category !== "all" ? category : null,
      favoritesOnly,
      sort: null,
    }),

  getItem: (id: string) => invoke<VaultItem>("get_item", { id }),

  saveItem: (item: ItemInput) => invoke<string>("save_item", { payload: item }),

  deleteItem: (id: string) => invoke<void>("delete_item", { id }),

  toggleFavorite: (id: string) => invoke<void>("toggle_favorite", { id }),

  passwordStrength: (password: string) =>
    invoke<PasswordAssessment>("password_strength", { password }),

  generatePassword: (options: PasswordOptions) =>
    invoke<GeneratedSecret>("generate_password", { options }),

  generatePassphrase: (options: PassphraseOptions) =>
    invoke<GeneratedSecret>("generate_passphrase", { options }),

  copySecret: (text: string) =>
    invoke<ClipboardStatus>("copy_secret", { text }),

  clearClipboard: () => invoke<void>("clear_clipboard"),

  authorizeBackup: (masterPassword: string) =>
    invoke<number>("authorize_backup", { masterPassword }),

  exportBackup: () => invoke<string>("export_backup"),

  importBackup: (
    backupText: string,
    mode: "merge" | "replace",
    backupPassword?: string,
  ) =>
    invoke<ImportResult>("import_backup", {
      backupText,
      mode,
      backupPassword: backupPassword || null,
    }),

  exportBackupFile: (path: string) =>
    invoke<void>("export_backup_file", { path }),

  importBackupFile: (
    path: string,
    mode: "merge" | "replace",
    backupPassword?: string,
  ) =>
    invoke<ImportResult>("import_backup_file", {
      path,
      mode,
      backupPassword: backupPassword || null,
    }),

  otpauthUri: (id: string) => invoke<string>("otpauth_uri", { id }),
};

export function errorMessage(error: unknown): string {
  if (typeof error === "string") return error;
  if (error instanceof Error) return error.message;
  if (error && typeof error === "object" && "message" in error) {
    return String((error as { message: unknown }).message);
  }
  return "Неизвестная ошибка";
}
