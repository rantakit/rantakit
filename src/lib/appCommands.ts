import { useEffect, useRef } from "react";

/**
 * Everything the shortcuts, the title bar menu, and the macOS menu bar can
 * do. The menu bar sends these as `menu` events (see src-tauri/src/menu.rs).
 */
export type AppCommand =
  | "open_files"
  | "open_settings"
  | "quit"
  | "undo"
  | "cut"
  | "copy"
  | "paste"
  | "select_all"
  | "toggle_sidebar"
  | "go_home"
  | "go_history"
  | "go_back"
  | "go_forward"
  | "open_source"
  | "report_problem"
  | "open_license"
  | "open_about";

interface Shortcut {
  /** `KeyboardEvent.key`, lower-cased. */
  key: string;
  shift?: boolean;
  /** Alt in place of Ctrl. */
  alt?: boolean;
}

/**
 * Shortcuts for Windows and Linux; on macOS the menu bar owns these as Cmd.
 * Ctrl+Z belongs to Arrange PDF Pages, and text fields keep the system's.
 */
const SHORTCUTS: Partial<Record<AppCommand, Shortcut>> = {
  open_files: { key: "o" }, // Ctrl+O
  open_settings: { key: "," }, // Ctrl+,
  toggle_sidebar: { key: "b" }, // Ctrl+B
  go_home: { key: "1" }, // Ctrl+1
  go_history: { key: "2" }, // Ctrl+2
  go_back: { key: "arrowleft", alt: true }, // Alt+←
  go_forward: { key: "arrowright", alt: true }, // Alt+→
};

const KEY_NAMES: Record<string, string> = { arrowleft: "←", arrowright: "→" };

/** How the title bar menu shows a command's shortcut, such as "Ctrl+," or "Alt+←"; null for none. */
export function shortcutLabel(command: AppCommand): string | null {
  if (command === "undo") return "Ctrl+Z";
  const shortcut = SHORTCUTS[command];
  if (!shortcut) return null;
  const key = KEY_NAMES[shortcut.key] ?? shortcut.key.toUpperCase();
  return [shortcut.alt ? "Alt" : "Ctrl", shortcut.shift && "Shift", key].filter(Boolean).join("+");
}

/** Run `onCommand` for the shortcuts above. Pass `enabled: false` to skip. */
export function useShortcuts(onCommand: (command: AppCommand) => void, enabled: boolean) {
  const onCommandRef = useRef(onCommand);
  onCommandRef.current = onCommand;

  useEffect(() => {
    if (!enabled) return;

    const onKeyDown = (event: KeyboardEvent) => {
      if (event.ctrlKey === event.altKey || event.metaKey || event.repeat) return;
      const key = event.key.toLowerCase();
      const match = Object.entries(SHORTCUTS).find(
        ([, s]) => s.key === key && Boolean(s.shift) === event.shiftKey && Boolean(s.alt) === event.altKey,
      );
      if (!match) return;
      event.preventDefault();
      onCommandRef.current(match[0] as AppCommand);
    };

    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [enabled]);
}
