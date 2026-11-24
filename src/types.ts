export type Category = "login" | "card" | "note" | "apiKey" | "ssh";

export interface KdfParams {
  algorithm: string;
  memoryKib: number;
  iterations: number;
  parallelism: number;
  hashLen: number;
  salt: string;
}

export interface TotpInfo {
  code: string;
  secondsRemaining: number;
  period: number;
}

export interface ItemFields {
  category: Category;
  title: string;
  service: string;
  username: string;
  password: string;
  totpSecret: string;
  note: string;
  favorite: boolean;
  tags: string[];
}

export interface ItemInput extends ItemFields {
  id?: string | null;
}

export interface VaultItem extends ItemFields {
  id: string;
  createdAt: string;
  updatedAt: string;
}

export interface ItemSummary {
  id: string;
  category: Category;
  title: string;
  service: string;
  username: string;
  favorite: boolean;
  tags: string[];
  hasPassword: boolean;
  hasTotp: boolean;
  totp: TotpInfo | null;
  createdAt: string;
  updatedAt: string;
}

export interface VaultStatus {
  exists: boolean;
  unlocked: boolean;
  autoLockMinutes: number;
  itemCount: number;
  kdf: KdfParams | null;
}

export interface ImportResult {
  imported: number;
  skipped: number;
  mode: "merge" | "replace";
}

export interface PasswordOptions {
  length: number;
  uppercase: boolean;
  lowercase: boolean;
  digits: boolean;
  symbols: boolean;
  excludeSimilar: boolean;
}

export interface PassphraseOptions {
  words: number;
  separator: string;
  capitalize: boolean;
  appendDigit: boolean;
}

export interface GeneratedSecret {
  value: string;
  entropyBits: number;
  kind: "password" | "passphrase" | string;
}

export interface ClipboardStatus {
  expiresInSeconds: number;
}

export interface PasswordAssessment {
  entropyBits: number;
  score: number;
  label: string;
  suggestions: string[];
}

export type View = "items" | "generator" | "backup" | "settings";
