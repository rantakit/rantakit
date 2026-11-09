import React from "react";
import { isTauri } from "@tauri-apps/api/core";
import ReactDOM from "react-dom/client";
import App from "./App";
import { syncPreferences } from "./features/settings/preferences";
import "./index.css";

async function start() {
  // Theme and text size before the first render, so the screen doesn't flash.
  syncPreferences();
  // In a plain browser during development, a stand-in backend answers commands.
  if (import.meta.env.DEV && !isTauri()) (await import("./lib/devMocks")).installDevMocks();
  ReactDOM.createRoot(document.getElementById("root")!).render(
    <React.StrictMode>
      <App />
    </React.StrictMode>,
  );
}

// index.html already shows the "needs a newer WebView" message.
if (!window.__RANTAKIT_UNSUPPORTED__) void start();
