/// <reference types="vite/client" />

interface ImportMetaEnv {
  /** Set by the Tauri CLI during `tauri dev` / `tauri build`: windows, darwin, linux, android, ios. */
  readonly TAURI_ENV_PLATFORM?: string;
}

interface Window {
  /** Set by index.html when the web view is too old for Rantakit's styles. */
  __RANTAKIT_UNSUPPORTED__?: boolean;
}