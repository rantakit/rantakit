import { useEffect, useState, type ReactNode } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { ChevronLeft, ChevronRight, Copy, Minus, PanelLeft, Square, X } from "@keyline-icons/react";
import { Button } from "react-aria-components";
import { useT } from "@/i18n";
import { isMacApp } from "@/lib/platform";
import { focusRing, tv } from "@/lib/react-aria-utils";
import type { AppCommand } from "@/lib/appCommands";
import { AppMenu } from "./AppMenu";

/** Title bar button, 28 px square. */
const control = tv({
  extend: focusRing,
  base: [
    "flex size-7 rounded-lg items-center justify-center text-text-muted",
    "cursor-default select-none transition-colors",
    "hover:bg-badge-bg hover:text-text pressed:bg-badge-bg disabled:opacity-40",
  ],
  variants: {
    close: { true: "hover:bg-danger hover:text-on-danger pressed:bg-danger-hover" },
  },
});

export interface TitleBarNav {
  canBack: boolean;
  canForward: boolean;
  onBack: () => void;
  onForward: () => void;
  sidebarOpen: boolean;
  onToggleSidebar: () => void;
  /** Runs what is chosen in the hamburger menu. */
  onCommand: (command: AppCommand) => void;
}

/**
 * The desktop title bar: the hamburger menu (not on macOS, which has its menu bar),
 * the sidebar toggle, Back and Forward on the left,
 * `children` (the "Versi baru" notice) and the window controls on the right.
 * macOS keeps its own traffic lights, so the buttons start clear of them and
 * the window controls are left out. The empty strip moves the window; a
 * double-click maximizes it.
 */
export function TitleBar({
  canBack,
  canForward,
  onBack,
  onForward,
  sidebarOpen,
  onToggleSidebar,
  onCommand,
  children,
}: TitleBarNav & { children?: ReactNode }) {
  const { t } = useT();

  return (
    <header
      data-tauri-drag-region
      className={`flex h-9 shrink-0 items-center gap-2 px-1.5 ${isMacApp ? "ps-20" : ""}`}
    >
      <div className="flex shrink-0 items-center gap-1">
        {!isMacApp && (
          <AppMenu
            canBack={canBack}
            canForward={canForward}
            sidebarOpen={sidebarOpen}
            onCommand={onCommand}
            className={(rp) => control(rp)}
          />
        )}
        {/* The sidebar only exists from md up. */}
        <Button
          aria-label={t(sidebarOpen ? "nav.sidebar.hide" : "nav.sidebar.show")}
          onPress={onToggleSidebar}
          className={(rp) => `${control(rp)} max-md:hidden`}
        >
          <PanelLeft aria-hidden className="size-4" />
        </Button>
        <Button aria-label={t("nav.back")} isDisabled={!canBack} onPress={onBack} className={(rp) => control(rp)}>
          <ChevronLeft aria-hidden className="size-4" />
        </Button>
        <Button aria-label={t("nav.forward")} isDisabled={!canForward} onPress={onForward} className={(rp) => control(rp)}>
          <ChevronRight aria-hidden className="size-4" />
        </Button>
      </div>
      <div data-tauri-drag-region className="h-full flex-1" />
      {children}
      {!isMacApp && <WindowControls />}
    </header>
  );
}

/** Minimize, maximize or restore, and close, for Windows and Linux. */
function WindowControls() {
  const { t } = useT();
  // getCurrentWindow() returns a new object on every call: keep one, or the
  // effect below would re-run after every render.
  const [appWindow] = useState(getCurrentWindow);
  const [isMaximized, setIsMaximized] = useState(false);

  useEffect(() => {
    const sync = () => appWindow.isMaximized().then(setIsMaximized);
    sync();
    const unlisten = appWindow.onResized(sync);
    return () => {
      unlisten.then((fn) => fn());
    };
  }, [appWindow]);

  return (
    <div className="flex shrink-0 items-center gap-1">
      <Button aria-label={t("window.minimize")} onPress={() => appWindow.minimize()} className={(rp) => control(rp)}>
        <Minus aria-hidden className="size-4" />
      </Button>
      <Button
        aria-label={t(isMaximized ? "window.restore" : "window.maximize")}
        onPress={() => appWindow.toggleMaximize()}
        className={(rp) => control(rp)}
      >
        {isMaximized ? <Copy aria-hidden className="size-3.5" /> : <Square aria-hidden className="size-3.5" />}
      </Button>
      <Button
        aria-label={t("window.close")}
        onPress={() => appWindow.close()}
        className={(rp) => control({ ...rp, close: true })}
      >
        <X aria-hidden className="size-4" />
      </Button>
    </div>
  );
}
