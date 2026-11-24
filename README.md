# VoidVault

Offline, local-first password and 2FA manager. No cloud, accounts, sync, or telemetry.

Built with Tauri v2, Rust, React 19, TypeScript, Vite, Tailwind CSS v4, Zustand, and Motion. All secrets stay in a local SQLite database; the master key exists only in Rust process memory and is zeroized on lock.

## Features

- Vault categories: логины, карты, защищённые заметки, API-ключи, SSH.
- Per-field AES-256-GCM encryption with Argon2id (`64 MiB`, `3` iterations, `parallelism 4`, `32`-byte key).
- RFC 6238 TOTP codes generated in Rust with a live 30-second progress indicator.
- Password/passphrase generator with entropy estimate and similar-character exclusion.
- Instant search and hotkeys: `Ctrl/Cmd+K` search, `Ctrl/Cmd+N` new item, `Ctrl/Cmd+L` lock.
- Clipboard copy with automatic clear after 30 seconds.
- Auto-lock after 1, 5, or 15 minutes of inactivity; also locks when the window loses focus or the page becomes hidden.
- Encrypted `VOIDVAULT 1` text backups with file, clipboard, and text import/export.
- AMOLED-black UI with liquid-glass cards, mono secrets, and Android safe-area padding.

## Requirements

- Node.js + pnpm
- Rust stable
- Tauri v2 desktop prerequisites for Windows/macOS/Linux
- For Android: Android SDK, NDK, Java JDK, and the `aarch64-linux-android` Rust target

## Install

```bash
pnpm install
pnpm approve-builds --all
```

`approve-builds` is needed once if pnpm blocks esbuild's postinstall script.

## Development

```bash
pnpm dev            # frontend only, Vite dev server
pnpm tauri dev      # full Tauri app
```

## Desktop build

```bash
pnpm tauri build
```

Outputs:

- `src-tauri/target/release/bundle/msi/VoidVault_0.1.0_x64_en-US.msi`
- `src-tauri/target/release/bundle/nsis/VoidVault_0.1.0_x64-setup.exe`

## Android build

Set the Android SDK paths first (PowerShell):

```powershell
$env:ANDROID_HOME = "$env:LOCALAPPDATA\Android\Sdk"
$env:ANDROID_SDK_ROOT = $env:ANDROID_HOME
```

Generate the Android project if it is missing:

```bash
pnpm tauri android init
```

The normal command is:

```bash
pnpm tauri android build --apk
```

On Windows, Tauri CLI 2.12.6 creates a symlink into `jniLibs`. Without Developer Mode or `SeCreateSymbolicLinkPrivilege`, that step fails with `Creation symbolic link is not allowed for this system`. A copy-based workaround is already applied to:

`src-tauri/gen/android/buildSrc/src/main/java/com/voidvault/vault/kotlin/BuildTask.kt`

It copies the prebuilt Rust `.so` instead of asking Tauri to symlink it. To build with that workaround:

```powershell
$env:ANDROID_HOME = "$env:LOCALAPPDATA\Android\Sdk"
$env:ANDROID_SDK_ROOT = $env:ANDROID_HOME
$env:CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER = "$env:ANDROID_HOME\ndk\27.1.12297006\toolchains\llvm\prebuilt\windows-x86_64\bin\aarch64-linux-android24-clang.cmd"

cargo build --manifest-path src-tauri/Cargo.toml --target aarch64-linux-android --release
```

Then, from `src-tauri/gen/android`:

```powershell
.\gradlew.bat assembleArm64Release
```

Finally, align and sign the APK with the standard Android debug keystore:

```powershell
$bt = "$env:ANDROID_HOME\build-tools\37.0.0"
$dir = ".\app\build\outputs\apk\arm64\release"
$unsigned = Join-Path $dir 'app-arm64-release-unsigned.apk'
$aligned = Join-Path $dir 'app-arm64-release-aligned.apk'
$signed = Join-Path $dir 'VoidVault_0.1.0_arm64-release.apk'

& "$bt\zipalign.exe" -p -f 4 $unsigned $aligned
& "$bt\apksigner.bat" sign --ks "$env:USERPROFILE\.android\debug.keystore" --ks-pass pass:android --key-pass pass:android --out $signed $aligned
& "$bt\apksigner.bat" verify --print-certs $signed
```

The resulting APK is:

`src-tauri/gen/android/app/build/outputs/apk/arm64/release/VoidVault_0.1.0_arm64-release.apk`

This is an arm64-only APK, signed with the Android debug key. Regenerating the Android project (`pnpm tauri android init`) overwrites the patched `BuildTask.kt`; reapply the copy-based task or enable Windows Developer Mode if you want to use `pnpm tauri android build --apk` directly.

## Backups

Exports use the text format:

```text
VOIDVAULT 1
<base64-wrapped encrypted JSON envelope>
```

The envelope contains the KDF parameters and a payload encrypted with the current vault key. The payload holds all encrypted item fields, the verifier, and the auto-lock setting.

- **Объединить**: imports new IDs and skips IDs that already exist.
- **Заменить всё**: deletes all current items before importing. The current vault master password and KDF are kept; imported secrets are re-encrypted with the current vault key.
- If the backup was made with a different master password, enter that password in the **Пароль бэкапа** field.

Backups can be generated, saved to a file, copied to the clipboard, or pasted as text. Treat exported backup text as a secret. Export and import require repeating the master password; the permission lives for 120 seconds in Rust memory only and is revoked on lock.

## Безопасность: что защищено, а что нет

VoidVault имеет открытый исходный код. Любой может прочитать код и увидеть все проверки, поэтому безопасность не строится на скрытности реализации. Она держится на мастер-ключе и на границах, которые навязывает операционная система.

### Модель угроз

| Угроза | Что делает злоумышленник | Что делает VoidVault |
| --- | --- | --- |
| Стиллер | Читает файлы, буфер обмена, базы браузеров | Нет браузерного хранилища и облака; каждое поле шифруется отдельно AES-256-GCM; буфер очищается через 30 с; мастер-ключ не покидает Rust |
| RAT с правами пользователя | Читает память процесса, перехватывает клавиатуру, внедряется в webview | Само приложение эту угрозу не останавливает. Граница здесь — ОС: не работать под администратором, Secure Boot + HVCI + Credential Guard, Defender/EDR |
| RAT с правами администратора или ядра | Полный контроль над машиной | Не помогает ничего. Если машина скомпрометирована на этом уровне, секреты доступны злоумышленнику |
| Подмена бинарника — главная угроза опенсорса | Форк репозитория, троян под видом официального релиза | Сборка в GitHub Actions, аттестации происхождения, `SHA256SUMS`, проверка хэша перед установкой |

### Что реально реализовано

- Мастер-ключ выделяется в куче, фиксируется в физической памяти (`mlock` на Unix, `VirtualLock` на Windows) и обнуляется перед освобождением. Каждая копия ключа фиксируется отдельно; `Drop` сначала снимает фиксацию, затем `zeroize` затирает байты.
- Строгий CSP: `connect-src` разрешает только IPC, внешние адреса запрещены; `object-src`, `base-uri`, `frame-src` и `form-action` равны `'none'`. Devtools отключены, глобальный мост `window.__TAURI__` не предоставляется.
- Capabilities ограничены: `core:default`, запись и очистка буфера, диалоги открытия и сохранения. Сетевого плагина нет — внедрённому скрипту некуда отправить данные.
- Экспорт и импорт требуют повторного мастер-пароля. Разрешение живёт 120 секунд в памяти Rust и сбрасывается при блокировке: RAT не вытащит всё хранилище тихо из уже разблокированной сессии.
- Автоблокировка через 1, 5 или 15 минут, блокировка при потере фокуса и скрытии окна.

### Чего намеренно нет

`IsDebuggerPresent`, anti-VM, тайминговые ловушки и прочая обфускация. В открытом коде это задержка на десять минут, а не защита. Мы не выдаём её за безопасность.

### Проверка сборки

1. Скачай установщик и соответствующий `SHA256SUMS-*.txt` из релиза.
2. Проверь хэш:

   ```powershell
   Get-FileHash .\VoidVault_0.1.0_x64-setup.exe -Algorithm SHA256
   ```

3. В разделе Actions найди нужный run и проверь attestation: бинарник должен быть собран из того же коммита, что и релиз.
4. При желании собери из исходника локально (`pnpm install && pnpm tauri build`) и сравни конфигурацию и аттестацию. Побайтовое совпадение MSI/NSIS не гарантировано из-за меток времени в установщиках.

### Требования к системе

Не используй административную учётную запись для ежедневной работы. Включай Secure Boot, HVCI и Credential Guard в Windows. Если на машине уже есть RAT с правами ядра, никакое приложение не спасёт секреты.

## Storage and crypto notes

- SQLite lives in Tauri's `app_data_dir()`.
- Each sensitive field is encrypted separately with AES-256-GCM. AAD binds ciphertext to the item ID and field name (`voidvault/field/v1/{id}/{field}`).
- The master key is derived with Argon2id and never sent to the frontend.
- Locking zeroizes the in-memory key and cancels pending clipboard clears.

## Logo

The supplied `logo.png` was actually JPEG data. It was converted in place to a real PNG, and the original bytes are preserved as `logo.original.jpg`. App icons were generated with:

```bash
pnpm tauri icon logo.png
```
