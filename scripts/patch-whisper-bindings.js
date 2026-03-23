/**
 * Patches whisper-rs-sys bundled bindings with correct Windows bindings.
 *
 * The bundled bindings in whisper-rs-sys v0.15.0 were generated for Linux,
 * causing struct size assertion failures on Windows. This script replaces
 * them with pre-generated Windows bindings.
 *
 * Run automatically via npm pretauri:dev / pretauri:build hooks.
 */
import { existsSync, readdirSync, copyFileSync } from "fs";
import { join, resolve } from "path";

const CRATE_NAME = "whisper-rs-sys-0.15.0";
const REGISTRY_BASE = join(
  process.env.CARGO_HOME || join(process.env.USERPROFILE || "", ".cargo"),
  "registry",
  "src"
);

if (process.platform !== "win32") {
  process.exit(0);
}

const bindingsSource = resolve("src-tauri", "whisper-bindings", "bindings_windows.rs");
if (!existsSync(bindingsSource)) {
  console.error("[patch-whisper-bindings] Windows bindings file not found:", bindingsSource);
  process.exit(1);
}

if (!existsSync(REGISTRY_BASE)) {
  console.log("[patch-whisper-bindings] Cargo registry not found, skipping (will be created on first build)");
  process.exit(0);
}

let patched = false;
for (const index of readdirSync(REGISTRY_BASE)) {
  const crateSrc = join(REGISTRY_BASE, index, CRATE_NAME, "src", "bindings.rs");
  if (existsSync(crateSrc)) {
    copyFileSync(bindingsSource, crateSrc);
    console.log("[patch-whisper-bindings] Patched:", crateSrc);
    patched = true;
  }
}

if (!patched) {
  console.log("[patch-whisper-bindings] whisper-rs-sys not yet in registry cache, skipping");
}
