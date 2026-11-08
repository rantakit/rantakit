import { useEffect, useState } from "react";
import { AppShell } from "@/components/layout/AppShell";
import { AlertDialog } from "@/components/ui/AlertDialog";
import { Modal } from "@/components/ui/Modal";
import type { NavTarget, Page } from "@/components/layout/navigation";
import {
  askNotificationPermission,
  askToSaveResults,
  canSaveResults,
  notificationPermission,
  type ToolOptions,
} from "@/features/compress/taskApi";
import { useCompressJob } from "@/features/compress/useCompressJob";
import { DropDialog } from "@/features/drop/DropDialog";
import { dropHint } from "@/features/drop/dropHint";
import { matchFiles, useDropMatch, type DropMatch } from "@/features/drop/dropMatch";
import { DropOverlay } from "@/features/drop/DropOverlay";
import { pickAnyFiles } from "@/features/drop/pickFiles";
import { useSharedFiles } from "@/features/drop/sharedFiles";
import { HomePage } from "@/features/home/HomePage";
import type { ToolId } from "@/features/home/tools";
import { HistoryPage } from "@/features/history/HistoryPage";
import { SettingsDialog, SettingsPage, type SettingsTab } from "@/features/settings/SettingsDialog";
import { fileName } from "@/features/compress/imageApi";
import { nounOf, pageNoun } from "@/features/tool/fileInfo";
import { LeaveDialog } from "@/features/tool/LeaveDialog";
import { ToolPage } from "@/features/tool/ToolPage";
import { UpdateNotice } from "@/features/update/UpdateNotice";
import { isShipped, toolDef, type ShippedTool } from "@/features/tool/tools";
import { useShortcuts, type AppCommand } from "@/lib/appCommands";
import { useBackButton } from "@/lib/backButton";
import { useFileDrop } from "@/lib/fileDrop";
import { useMenuEvents, useNativeLanguage } from "@/lib/menuEvents";
import { useNavHistory } from "@/lib/navHistory";
import { isAndroidApp, isMacApp } from "@/lib/platform";
import { useStoredState } from "@/lib/storedState";
import { useWideLayout } from "@/lib/wideLayout";
import { useT } from "@/i18n";

/** Dropped files waiting for the user to choose a Tool. */
interface Dropped {
  paths: string[];
  match: Extract<DropMatch, { status: "fits" }>;
}

const unique = (paths: string[]) => [...new Set(paths)];

/** Whether the sidebar is shown, remembered between launches; shown unless the user hid it. */
const parseLayout = (stored: unknown) => ({
  sidebar: !(stored && typeof stored === "object" && (stored as { sidebar?: unknown }).sidebar === false),
});

function App() {
  const tr = useT();
  const { t } = tr;
  const [page, setPage] = useState<Page>("home");
  /**
   * Settings while open: a dialog over the page, or on a phone a page of its
   * own; "list" is the phone's list of parts, which the dialog shows as its
   * first tab.
   */
  const [settingsTab, setSettingsTab] = useState<SettingsTab | "list" | null>(null);
  const wide = useWideLayout();
  const settingsPage = settingsTab !== null && !wide;
  /** The Tool page on screen, over Home. */
  const [tool, setTool] = useState<ShippedTool | null>(null);
  /** The Originals added to that page. */
  const [paths, setPaths] = useState<string[]>([]);
  const job = useCompressJob();
  /** What to do once the user agrees to leave the Tool page, and whether that stops a Task. */
  const [leaving, setLeaving] = useState<{ action: () => void; working: boolean } | null>(null);

  /** Clears the Tool page; a running Task is stopped, and its finished Results stay. */
  const closeTool = () => {
    if (tool) toolDef(tool).discard?.(paths);
    setTool(null);
    setPaths([]);
    job.reset();
  };

  /** A Tool with its own work area (Atur Halaman PDF) has changes that leaving would lose. */
  const [unsaved, setUnsaved] = useState(false);
  /** Another file chosen for such a Tool, waiting for "Ganti PDF?" to be answered. */
  const [replacing, setReplacing] = useState<string | null>(null);

  /** Run `action`, which leaves the Tool page, after asking if files would be lost. */
  const leaveTool = (action: () => void) => {
    // A work area loses nothing until it is changed.
    const hasFiles = paths.length > 0 && job.items.length === 0 && (!tool || !toolDef(tool).Workspace || unsaved);
    if (tool && (hasFiles || job.isRunning)) setLeaving({ action, working: job.isRunning });
    else action();
  };

  const navigate = (next: NavTarget) => {
    // Settings opens over the page; the Tool page stays as it is.
    if (next === "settings") return setSettingsTab("list");
    leaveTool(() => {
      setSettingsTab(null);
      closeTool();
      setPage(next satisfies Page);
    });
  };

  const openTool = (id: ShippedTool, withPaths: string[] = []) => {
    setPage("home");
    setTool(id);
    setPaths(unique(withPaths));
    job.reset();
  };

  /** Add Originals to the open Tool page, from the picker or a drop. */
  const addFiles = (added: string[]) => {
    if (job.isRunning || added.length === 0) return;
    // A Tool with its own work area takes one file per Task; another one replaces it.
    if (tool && toolDef(tool).Workspace) {
      const next = added[0];
      if (next === paths[0] && job.items.length === 0) return;
      if (unsaved && job.items.length === 0) return setReplacing(next);
      job.reset();
      setPaths([next]);
      return;
    }
    // Beri Watermark takes photos or PDFs, but not both on one page (X5).
    if (tool && toolDef(tool).noun === "file") {
      const kept = job.items.length > 0 ? [] : paths;
      if (new Set([...kept, ...added].map(nounOf)).size > 1) {
        return setRefused({ title: t("drop.mixed"), detail: t("drop.mixed.choose") });
      }
    }
    // After a Result, new files start over.
    if (job.items.length > 0) {
      job.reset();
      setPaths(unique(added));
    } else {
      setPaths((prev) => unique([...prev, ...added]));
    }
  };

  // A Tool just opened from elsewhere has no files yet.
  const pickFiles = async (id: ShippedTool) =>
    addFiles(await toolDef(id).pick(pageNoun(toolDef(id).noun, id === tool && job.items.length === 0 ? paths : [])));

  /** From the menu: open the Tool's page, then the picker. */
  const startTool = (id: ShippedTool) => {
    if (tool === id && page === "home") return pickFiles(id);
    leaveTool(() => {
      openTool(id);
      pickFiles(id);
    });
  };

  /** Android 8 and 9: why the storage permission is asked for, or that it was refused, before a Task. */
  const [saving, setSaving] = useState<{ status: "ask" | "refused"; options: ToolOptions } | null>(null);

  /** Android 13 and later: why notifications are asked for, once, before the first Task. */
  const [notifying, setNotifying] = useState<ToolOptions | null>(null);

  const startJob = async (options: ToolOptions) => {
    if (!(await canSaveResults())) setSaving({ status: "ask", options });
    else if ((await notificationPermission()) === "ask") setNotifying(options);
    else job.start(paths, options);
  };

  const askToSave = async (options: ToolOptions) => {
    if (await askToSaveResults()) job.start(paths, options);
    else setSaving({ status: "refused", options });
  };

  /** The Task starts whatever the answer; without the permission only its notification is hidden. */
  const askToNotify = async (options: ToolOptions) => {
    await askNotificationPermission();
    job.start(paths, options);
  };

  const [dropped, setDropped] = useState<Dropped | null>(null);
  const pageTool = page === "home" ? tool : null;

  // A Tool page sits over Home, so the Tool alone names where the user is.
  const history = useNavHistory<Page | ShippedTool>(pageTool ?? page);

  /** Back or Forward; leaving a Tool page asks first, and staying keeps the history where it is. */
  const goHistory = (delta: -1 | 1) => {
    const target = history.target(delta);
    if (!target) return;
    leaveTool(() => {
      history.go(delta);
      if (target === "home" || target === "history") {
        closeTool();
        setPage(target);
      } else {
        openTool(target);
      }
    });
  };

  const [layout, updateLayout] = useStoredState("rantakit.layout", parseLayout);
  const toggleSidebar = () => updateLayout({ sidebar: !layout.sidebar });

  /**
   * Files dropped on the window or chosen with "Buka": ask for a Tool, or add
   * them to the open one. Files that don't fit come back with the reason, in
   * the drop overlay's words.
   */
  const receiveFiles = async (received: string[]) => {
    if (job.isRunning || received.length === 0) return null;
    const match = await matchFiles(received);
    // Only "Buka" shows the hint; a drop was explained while dragging.
    const hint = dropHint(tr, { busy: false, openTool: pageTool, match, count: received.length, chosen: true });
    if (!hint.canDrop || match.status !== "fits") return hint;
    if (!pageTool) setDropped({ paths: received, match });
    else addFiles(received);
    return null;
  };

  /** A drop that doesn't fit was already explained by the overlay while dragging. */
  const dragging = useFileDrop((dropPaths) => void receiveFiles(dropPaths));

  /** Why the files chosen with "Buka" were not taken. */
  const [refused, setRefused] = useState<{ title: string; detail?: string } | null>(null);
  const openFiles = async () => setRefused(await receiveFiles(await pickAnyFiles()));
  const draggedMatch = useDropMatch(dragging);

  /** A share that came while a Task was running, and was not taken. */
  const [shareBusy, setShareBusy] = useState(false);

  /**
   * Files shared from another app on Android: the Tools that fit, as for
   * "Buka" on Home, after asking if files on a Tool page would be lost.
   */
  const shareFiles = (shared: string[]) => {
    if (job.isRunning) return setShareBusy(true);
    leaveTool(async () => {
      setSettingsTab(null);
      closeTool();
      const match = await matchFiles(shared);
      const hint = dropHint(tr, { busy: false, openTool: null, match, count: shared.length, chosen: true });
      if (!hint.canDrop || match.status !== "fits") setRefused(hint);
      else setDropped({ paths: shared, match });
    });
  };
  useSharedFiles(shareFiles);

  const chooseDropTool = (id: ToolId) => {
    if (isShipped(id) && dropped) openTool(id, dropped.paths);
    setDropped(null);
  };

  // Escape on a Tool page is Back, asking first when files would be lost.
  useEffect(() => {
    if (!pageTool || leaving || dropped || settingsTab) return;
    const onKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape" && !e.defaultPrevented) goHistory(-1);
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  });

  // The mouse's side buttons are Back and Forward. Cancelling them also keeps
  // the webview from trying a navigation of its own.
  useEffect(() => {
    if (leaving || dropped || settingsTab) return;
    const isSideButton = (e: MouseEvent) => e.button === 3 || e.button === 4;
    const onMouseDown = (e: MouseEvent) => {
      if (isSideButton(e)) e.preventDefault();
    };
    const onMouseUp = (e: MouseEvent) => {
      if (!isSideButton(e)) return;
      e.preventDefault();
      goHistory(e.button === 3 ? -1 : 1);
    };
    window.addEventListener("mousedown", onMouseDown);
    window.addEventListener("mouseup", onMouseUp);
    return () => {
      window.removeEventListener("mousedown", onMouseDown);
      window.removeEventListener("mouseup", onMouseUp);
    };
  });

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

  const noun = tool ? pageNoun(toolDef(tool).noun, paths) : "photo";

  return (
    <AppShell
      current={settingsPage ? "settings" : page}
      onNavigate={navigate}
      onOpen={openFiles}
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
            paths={paths}
            job={job}
            onPick={() => pickFiles(tool)}
            onAdd={addFiles}
            onRemove={(path) => {
              toolDef(tool).discard?.([path]);
              setPaths((prev) => prev.filter((p) => p !== path));
            }}
            onReorder={setPaths}
            onBack={() => leaveTool(closeTool)}
            onStart={startJob}
            onUnsavedChange={setUnsaved}
            onProcessAnother={() => {
              toolDef(tool).discard?.(paths);
              job.reset();
              setPaths([]);
            }}
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
      {dragging && (
        <DropOverlay
          {...dropHint(tr, { busy: job.isRunning, openTool: pageTool, match: draggedMatch, count: dragging.length })}
        />
      )}
      {dropped && (
        <DropDialog
          paths={dropped.paths}
          match={dropped.match}
          onChoose={chooseDropTool}
          onClose={() => setDropped(null)}
        />
      )}
      {refused && (
        <Modal isOpen isDismissable onOpenChange={(open) => !open && setRefused(null)}>
          <AlertDialog
            title={refused.title}
            actionLabel={t("drop.pickAgain")}
            cancelLabel={t("common.close")}
            onAction={openFiles}
          >
            {refused.detail}
          </AlertDialog>
        </Modal>
      )}
      {shareBusy && (
        <Modal isOpen isDismissable onOpenChange={(open) => !open && setShareBusy(false)}>
          <AlertDialog title={t("share.busy.title")} cancelLabel={t("common.close")}>
            {t("share.busy.body")}
          </AlertDialog>
        </Modal>
      )}
      {saving && (
        <Modal isOpen isDismissable onOpenChange={(open) => !open && setSaving(null)}>
          <AlertDialog
            title={t(`saving.${saving.status}.title`)}
            actionLabel={t(saving.status === "ask" ? "saving.ask.continue" : "saving.refused.retry")}
            cancelLabel={t(saving.status === "ask" ? "common.cancel" : "common.close")}
            onAction={() => askToSave(saving.options)}
          >
            {t(`saving.${saving.status}.body`)}
          </AlertDialog>
        </Modal>
      )}
      {notifying && (
        <Modal isOpen isDismissable onOpenChange={(open) => !open && setNotifying(null)}>
          <AlertDialog
            title={t("notifying.title")}
            actionLabel={t("notifying.continue")}
            cancelLabel={t("common.cancel")}
            onAction={() => askToNotify(notifying)}
          >
            {t("notifying.body")}
          </AlertDialog>
        </Modal>
      )}
      {leaving && (
        <LeaveDialog
          {...(leaving.working
            ? {
                title: t("leave.working.title"),
                body: t(`leave.working.body.${noun}`),
                leaveLabel: t("leave.working.stop"),
                stayLabel: t("leave.working.keep"),
              }
            : {
                title: t("leave.title"),
                body:
                  tool && toolDef(tool).Workspace && paths[0]
                    ? t("arrange.replace.body", { name: fileName(paths[0]) })
                    : tool && toolDef(tool).Source
                      ? t("leave.body.scan")
                      : t(`leave.body.${noun}`),
                leaveLabel: t("leave.leave"),
                stayLabel: t("leave.stay"),
              })}
          onStay={() => setLeaving(null)}
          onLeave={() => {
            setLeaving(null);
            leaving.action();
          }}
        />
      )}
      {replacing && paths[0] && (
        <LeaveDialog
          title={t("arrange.replace.title")}
          body={t("arrange.replace.body", { name: fileName(paths[0]) })}
          leaveLabel={t("arrange.replace")}
          stayLabel={t("arrange.replace.cancel")}
          onStay={() => setReplacing(null)}
          onLeave={() => {
            setReplacing(null);
            job.reset();
            setPaths([replacing]);
          }}
        />
      )}
    </AppShell>
  );
}

export default App;
