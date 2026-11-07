import { useLayoutEffect, useRef, type ReactNode } from "react";
import { isDesktopApp } from "@/lib/platform";
import { Sidebar } from "./Sidebar";
import { TabBar } from "./TabBar";
import type { NavTarget } from "./navigation";
import { TitleBar, type TitleBarNav } from "./TitleBar";

interface AppShellProps {
  /** The navigation item for what is on screen; "settings" while Settings is a page. */
  current: NavTarget;
  onNavigate: (target: NavTarget) => void;
  /** The sidebar's "Buka" button: choose files, then a Tool. */
  onOpen: () => void;
  /** Back, Forward and the sidebar toggle, shown in the desktop title bar. */
  titleBar: TitleBarNav;
  /** Shown at the title bar's right end, before the window controls. */
  titleBarEnd?: ReactNode;
  /** The phone's bottom tab bar; a Tool page has its own back button and main button there instead. */
  tabBar: boolean;
  /** What is on screen, such as a page or a Tool; a new one starts at the top. */
  screen: string;
  children: ReactNode;
}

/**
 * Desktop: a title bar, then the sidebar and the content as two cards on the
 * window background; the sidebar can be hidden. Android from 768 px wide:
 * the same without the title bar, so the sidebar always shows. Narrower:
 * full-width content with a bottom tab bar.
 */
export function AppShell({ current, onNavigate, onOpen, titleBar, titleBarEnd, tabBar, screen, children }: AppShellProps) {
  const main = useRef<HTMLElement>(null);
  // Otherwise a Tool opened from far down Home starts as far down.
  useLayoutEffect(() => {
    main.current?.scrollTo(0, 0);
  }, [screen]);
  // Only the desktop title bar can show a hidden sidebar again.
  const sidebar = titleBar.sidebarOpen || !isDesktopApp;
  return (
    <div className="flex h-full flex-col md:bg-canvas">
      {isDesktopApp && <TitleBar {...titleBar}>{titleBarEnd}</TitleBar>}
      <div className={`flex min-h-0 flex-1 flex-col md:flex-row md:gap-1.5 md:p-1.5 ${isDesktopApp ? "md:pt-0" : ""}`}>
        {sidebar && <Sidebar current={current} onNavigate={onNavigate} onOpen={onOpen} />}
        <main ref={main} className="flex min-h-0 min-w-0 flex-1 flex-col overflow-auto md:rounded-xl md:border md:border-border md:bg-surface">
          {children}
        </main>
        {tabBar && <TabBar current={current} onNavigate={onNavigate} />}
      </div>
    </div>
  );
}
