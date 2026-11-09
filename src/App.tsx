import { useState } from "react";
import { AppShell } from "@/components/layout/AppShell";
import type { NavTarget, Page } from "@/components/layout/navigation";
import { PermissionDialogs } from "@/features/compress/PermissionDialogs";
import { usePermissionGate } from "@/features/compress/usePermissionGate";
import { IncomingFilesDialogs } from "@/features/drop/IncomingFilesDialogs";
import { useIncomingFiles } from "@/features/drop/useIncomingFiles";
import { HomePage } from "@/features/home/HomePage";
import { HistoryPage } from "@/features/history/HistoryPage";
import { SettingsDialog, SettingsPage, type SettingsTab } from "@/features/settings/SettingsDialog";
import { ToolSessionDialogs } from "@/features/tool/LeaveGuard";
import { ToolPage } from "@/features/tool/ToolPage";
import { isShipped, type ShippedTool } from "@/features/tool/tools";
import { useToolSession } from "@/features/tool/useToolSession";
import { UpdateNotice } from "@/features/update/UpdateNotice";
import { useShortcuts, type AppCommand } from "@/lib/appCommands";
import { useBackButton } from "@/lib/backButton";
import { useMenuEvents, useNativeLanguage } from "@/lib/menuEvents";
import { useNavHistory } from "@/lib/navHistory";
import { isAndroidApp, isMacApp } from "@/lib/platform";
import { useStoredState } from "@/lib/storedState";
import { useBackForwardInputs } from "@/lib/useBackForwardInputs";
import { useWideLayout } from "@/lib/wideLayout";

/** Whether the sidebar is shown, remembered between launches; shown unless the user hid it. */
const parseLayout = (stored: unknown) => ({
  sidebar: !(stored && typeof stored === "object" && (stored as { sidebar?: unknown }).sidebar === false),
});

function App() {
  const [page, setPage] = useState<Page>("home");
  /**
   * Settings while open: a dialog over the page, or on a phone a page of its
   * own; "list" is the phone's list of parts, which the dialog shows as its
   * first tab.
   */
  const [settingsTab, setSettingsTab] = useState<SettingsTab | "list" | null>(null);
  const wide = useWideLayout();
  const settingsPage = settingsTab !== null && !wide;

  const session = useToolSession();
  const { tool, job } = session;
  /** The Tool page on screen; a Tool page sits over Home. */
  const pageTool = page === "home" ? tool : null;
  const gate = usePermissionGate(job.start);

  const openTool = (id: ShippedTool, withPaths: string[] = []) => {
    setPage("home");
    session.open(id, withPaths);
  };

  const incoming = useIncomingFiles({
    busy: job.isRunning,
    openTool: pageTool,
    addToTool: session.addFiles,
    openWith: openTool,
    leaveFor: (then) =>
      session.guard(() => {
        setSettingsTab(null);
        session.close();
        then();
      }),
  });

  const navigate = (next: NavTarget) => {
    // Settings opens over the page; the Tool page stays as it is.
    if (next === "settings") return setSettingsTab("list");
    session.guard(() => {
      setSettingsTab(null);
      session.close();
      setPage(next satisfies Page);
    });
  };

  const pickFiles = async (id: ShippedTool) => incoming.add(await session.pick(id));

  /** From the menu: open the Tool's page, then the picker. */
  const startTool = (id: ShippedTool) => {
    if (tool === id && page === "home") return pickFiles(id);
    session.guard(() => {
      openTool(id);
      pickFiles(id);
    });
  };

  // A Tool page sits over Home, so the Tool alone names where the user is.
  const history = useNavHistory<Page | ShippedTool>(pageTool ?? page);

  /** Back or Forward; leaving a Tool page asks first, and staying keeps the history where it is. */
  const goHistory = (delta: -1 | 1) => {
    const target = history.target(delta);
    if (!target) return;
    session.guard(() => {
      history.go(delta);
      if (target === "home" || target === "history") {
        session.close();
        setPage(target);
      } else {
        openTool(target);
      }
    });
  };

  const [layout, updateLayout] = useStoredState("rantakit.layout", parseLayout);
  const toggleSidebar = () => updateLayout({ sidebar: !layout.sidebar });

  // Escape on a Tool page is Back, asking first when files would be lost.
  const asking = session.leaving !== null || incoming.dropped !== null || settingsTab !== null;
  useBackForwardInputs(goHistory, { escape: pageTool !== null && !asking, mouse: !asking });

  // Android's back gesture: Settings' parts, then the same steps as Back. With
  // none left, Android closes the app itself. Open dialogs close first.
  useBackButton(() => {
    if (settingsPage) setSettingsTab(settingsTab === "list" ? null : "list");
    else goHistory(-1);
  }, settingsPage || history.canBack);

  /** Commands from the macOS menu bar and the keyboard shortcuts. */
  const runCommand = (command: AppCommand) => {
    switch (command) {
      case "go_home":
        return navigate("home");
      case "go_history":
        return navigate("history");
      case "open_settings":
        return navigate("settings");
      case "compress_image":
        return startTool("kompres-foto");
      case "compress_pdf":
        return startTool("kompres-pdf");
      case "go_back":
        return goHistory(-1);
      case "go_forward":
        return goHistory(1);
      case "toggle_sidebar":
        return toggleSidebar();
    }
  };

  useMenuEvents(runCommand);
  useNativeLanguage();
  // On macOS the menu bar handles these as Cmd shortcuts; Android has none.
  useShortcuts(runCommand, !isMacApp && !isAndroidApp);

  return (
    <AppShell
      current={settingsPage ? "settings" : page}
      onNavigate={navigate}
      onOpen={incoming.openFiles}
      tabBar={!pageTool || settingsPage}
      screen={settingsPage ? "settings" : (pageTool ?? page)}
      titleBar={{
        canBack: history.canBack,
        canForward: history.canForward,
        onBack: () => goHistory(-1),
        onForward: () => goHistory(1),
        sidebarOpen: layout.sidebar,
        onToggleSidebar: toggleSidebar,
      }}
      titleBarEnd={<UpdateNotice busy={job.isRunning} />}
    >
      {settingsPage && (
        <SettingsPage
          tab={settingsTab === "list" ? null : settingsTab}
          onTabChange={(tab) => setSettingsTab(tab ?? "list")}
        />
      )}
      {/* Hidden, not closed, under the Settings page, so a Tool page keeps its files and options. */}
      <div className={settingsPage ? "hidden" : "contents"}>
        {page === "home" &&
          (tool ? (
            <ToolPage
              key={tool}
              tool={tool}
              paths={session.paths}
              job={job}
              onPick={() => pickFiles(tool)}
              onAdd={incoming.add}
              onRemove={session.remove}
              onReorder={session.reorder}
              onBack={() => session.guard(session.close)}
              onStart={(options) => gate.request(session.paths, options)}
              onUnsavedChange={session.setUnsaved}
              onProcessAnother={session.processAnother}
            />
          ) : (
            <HomePage
              onOpenTool={(id) => isShipped(id) && openTool(id)}
              notice={isAndroidApp && <UpdateNotice busy={false} className="-mb-5 h-9 self-end px-3" />}
            />
          ))}
        {page === "history" && <HistoryPage />}
      </div>
      {settingsTab && wide && (
        <SettingsDialog
          tab={settingsTab === "list" ? "appearance" : settingsTab}
          onTabChange={setSettingsTab}
          onClose={() => setSettingsTab(null)}
        />
      )}
      <IncomingFilesDialogs incoming={incoming} />
      <PermissionDialogs gate={gate} />
      <ToolSessionDialogs session={session} />
    </AppShell>
  );
}

export default App;
