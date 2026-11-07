import { useCallback, useEffect, useState, type ReactNode } from "react";
import { revealItemInDir } from "@tauri-apps/plugin-opener";
import { ChevronRight, FolderOpen, History, Share2, SquareArrowOutUpRight, TriangleAlert } from "@keyline-icons/react";
import { DialogTrigger } from "react-aria-components";
import { AlertDialog } from "@/components/ui/AlertDialog";
import { Button } from "@/components/ui/Button";
import { Modal } from "@/components/ui/Modal";
import { fileName } from "@/features/compress/imageApi";
import { TOOL_ICONS, type ToolId } from "@/features/home/tools";
import { setKeepHistory, useAppSettings } from "@/features/settings/appSettings";
import { useT } from "@/i18n";
import { errorMessage } from "@/lib/errorMessages";
import { openPath, shareFiles } from "@/lib/files";
import { formatBytes } from "@/lib/format";
import { canShare, canShowInFolder } from "@/lib/platform";
import { clearHistory, listHistory, type HistoryFile, type HistoryItem } from "./historyApi";

/** "Riwayat": every Task with a Result, grouped by day, newest first. */
export function HistoryPage() {
  const tr = useT();
  const { t } = tr;
  const [items, setItems] = useState<HistoryItem[] | null>(null);
  const [error, setError] = useState<string>();
  const settings = useAppSettings();
  const off = settings?.keepHistory === false;

  const load = useCallback(() => {
    listHistory()
      .then((items) => {
        setItems(items);
        setError(undefined);
      })
      .catch((err) => setError(errorMessage(tr, err)));
  }, [tr]);

  useEffect(load, [load]);

  // Results may have been moved or deleted while Rantakit was out of sight.
  useEffect(() => {
    const onVisible = () => document.visibilityState === "visible" && load();
    document.addEventListener("visibilitychange", onVisible);
    return () => document.removeEventListener("visibilitychange", onVisible);
  }, [load]);

  const clear = () => {
    setItems([]);
    clearHistory().catch(load);
  };

  if (error) {
    return (
      <Message icon={TriangleAlert} title={t("history.loadFailed")}>
        {error}
      </Message>
    );
  }
  if (items === null) return null;

  return (
    <div className="mx-auto flex w-full max-w-4xl flex-1 flex-col gap-6 px-5 pt-6 pb-10 md:px-10">
      <header className="flex flex-wrap items-start justify-between gap-4">
        <div className="flex flex-col gap-1">
          <h1 className="font-display text-title">{t("history.title")}</h1>
          <p className="text-text-muted">{t("history.kept")}</p>
        </div>
        {items.length > 0 && !off && (
          <DialogTrigger>
            <Button variant="secondary" size="row">
              {t("history.clear")}
            </Button>
            <Modal>
              <AlertDialog
                title={t("history.clear.title")}
                variant="danger"
                actionLabel={t("history.clear")}
                cancelLabel={t("history.clear.cancel")}
                onAction={clear}
              >
                {t("history.clear.body")}
              </AlertDialog>
            </Modal>
          </DialogTrigger>
        )}
      </header>

      {off ? (
        <Message
          icon={History}
          title={t("history.off")}
          action={<Button onPress={() => setKeepHistory(true).then(load).catch(() => {})}>{t("history.turnOn")}</Button>}
        >
          {t("history.off.body")}
        </Message>
      ) : items.length === 0 ? (
        <Message icon={History} title={t("history.empty")}>
          {t("history.empty.hint")}
        </Message>
      ) : (
        byDay(items).map(([day, entries]) => (
          <section key={day} className="flex flex-col gap-3">
            <h2 className="text-label font-semibold text-text-muted">{dayLabel(tr, entries[0].createdAt)}</h2>
            <ul className="flex flex-col divide-y divide-border overflow-hidden rounded-2xl border border-border bg-surface">
              {entries.map((entry) => (
                <HistoryEntry key={entry.id} entry={entry} />
              ))}
            </ul>
          </section>
        ))
      )}
    </div>
  );
}

/** Entries grouped by calendar day, keeping their order. */
function byDay(items: HistoryItem[]): [string, HistoryItem[]][] {
  const days = new Map<string, HistoryItem[]>();
  for (const item of items) {
    const day = new Date(item.createdAt).toDateString();
    days.set(day, [...(days.get(day) ?? []), item]);
  }
  return [...days];
}

/** "Hari ini", "Kemarin", then "Senin, 26 Oktober". */
function dayLabel({ t, locale }: ReturnType<typeof useT>, time: number) {
  const date = new Date(time);
  const today = new Date();
  const yesterday = new Date();
  yesterday.setDate(today.getDate() - 1);
  if (date.toDateString() === today.toDateString()) return t("history.today");
  if (date.toDateString() === yesterday.toDateString()) return t("history.yesterday");
  return new Intl.DateTimeFormat(locale, {
    weekday: "long",
    day: "numeric",
    month: "long",
    year: date.getFullYear() === today.getFullYear() ? undefined : "numeric",
  }).format(date);
}

const isToolId = (tool: string): tool is ToolId => tool in TOOL_ICONS;

/** One Task: its Tool, time, Results, and sizes before and after. */
function HistoryEntry({ entry }: { entry: HistoryItem }) {
  const { t, count, locale } = useT();
  const [expanded, setExpanded] = useState(false);
  const known = isToolId(entry.tool);
  const Icon = known ? TOOL_ICONS[entry.tool as ToolId] : History;
  const existing = entry.files.filter((file) => file.exists);
  const first = entry.files[0];
  // Several Results from one Original, such as its pages, sit in one folder (X3).
  const fromOne = entry.files.length > 1 && entry.files.every((file) => file.originalPath === first.originalPath);
  // Several Originals merged into one Result, as in Gabung PDF: it shows as one.
  const toOne = entry.files.length > 1 && entry.files.every((file) => file.resultPath === first.resultPath);
  const results = toOne ? [first] : entry.files;
  const names = toOne
    ? t("history.mergedFrom", {
        files: count(
          entry.tool === "jpg-ke-pdf" ? "files.photo" : entry.tool === "scan-dokumen" ? "pages" : "files.pdf",
          entry.files.length,
        ),
        name: fileName(first.resultPath),
      })
    : fromOne
    ? t("history.madeFrom", { name: fileName(first.originalPath), files: count("files.any", entry.files.length) })
    : entry.files.length === 1
      ? fileName(first.resultPath)
      : t("history.andMore", { name: fileName(first.resultPath), count: entry.files.length - 1 });
  const before = fromOne ? first.originalBytes : entry.files.reduce((sum, file) => sum + file.originalBytes, 0);
  const after = results.reduce((sum, file) => sum + file.resultBytes, 0);
  const time = new Intl.DateTimeFormat(locale, { hour: "2-digit", minute: "2-digit" }).format(entry.createdAt);
  const missing = existing.length === 0;

  return (
    <li className="flex flex-col">
      <div className={`flex flex-wrap items-center gap-4 px-4 py-3.5 ${missing ? "text-text-muted" : ""}`}>
        <span
          aria-hidden
          className={`flex size-12 shrink-0 items-center justify-center rounded-xl ${
            missing ? "bg-badge-bg" : "bg-brand-soft text-brand-strong"
          }`}
        >
          <Icon className="size-6" />
        </span>
        <span className="flex min-w-0 flex-1 flex-col gap-0.5">
          <span className="flex items-baseline gap-2">
            <span className="text-label font-semibold">{known ? t(`tool.${entry.tool as ToolId}`) : entry.tool}</span>
            <span className="text-small whitespace-nowrap text-text-muted">{time}</span>
          </span>
          <span className="truncate" title={first.resultPath}>
            {names}
          </span>
          <span className="text-small text-text-muted">
            {missing ? (
              t("history.missing")
            ) : fromOne || toOne ? (
              formatBytes(after, locale)
            ) : (
              <>
                {formatBytes(before, locale)} → {formatBytes(after, locale)}
              </>
            )}
          </span>
        </span>
        {!missing && (
          // On a phone the buttons go on their own line, under the file names.
          <div className="flex w-full shrink-0 flex-wrap items-center gap-2 ps-16 md:w-auto md:ps-0">
            {results.length === 1 && <OpenButton file={first} />}
            {canShowInFolder &&
              (fromOne ? (
                <Button variant="secondary" size="row" onPress={() => openPath(parentOf(existing[0].resultPath))}>
                  <FolderOpen aria-hidden className="size-[18px]" /> {t("result.openFolder")}
                </Button>
              ) : (
                <Button variant="secondary" size="row" onPress={() => revealItemInDir(existing[0].resultPath)}>
                  <FolderOpen aria-hidden className="size-[18px]" /> {t("task.showInFolder")}
                </Button>
              ))}
            {canShare && (
              <Button
                variant="secondary"
                size="row"
                onPress={() => shareFiles(toOne ? [first.resultPath] : existing.map((file) => file.resultPath))}
              >
                <Share2 aria-hidden className="size-[18px]" />{" "}
                {t(results.length === 1 ? "result.share" : "result.shareAll")}
              </Button>
            )}
            {results.length > 1 && (
              <Button variant="secondary" size="row" aria-expanded={expanded} onPress={() => setExpanded(!expanded)}>
                {t("history.seeFiles", { count: entry.files.length })}
                <ChevronRight
                  aria-hidden
                  className={`size-4 transition-transform ${expanded ? "rotate-90" : ""}`}
                />
              </Button>
            )}
          </div>
        )}
      </div>
      {expanded && !missing && (
        <ul className="flex flex-col divide-y divide-border border-t border-border">
          {entry.files.map((file) => (
            <FileLine key={file.resultPath} file={file} fromOne={fromOne} />
          ))}
        </ul>
      )}
    </li>
  );
}

function OpenButton({ file }: { file: HistoryFile }) {
  const { t } = useT();
  const name = fileName(file.resultPath);
  return (
    <Button
      variant="secondary"
      size="row"
      aria-label={t("result.openName", { name })}
      onPress={() => openPath(file.resultPath)}
    >
      <SquareArrowOutUpRight aria-hidden className="size-[18px]" /> {t("result.open")}
    </Button>
  );
}

/** The folder a Result sits in. */
const parentOf = (path: string) => path.slice(0, Math.max(path.lastIndexOf("/"), path.lastIndexOf("\\")));

/** One Result of an expanded entry; one of several from one Original shows only its own size. */
function FileLine({ file, fromOne }: { file: HistoryFile; fromOne: boolean }) {
  const { t, locale } = useT();
  const sizes = fromOne
    ? formatBytes(file.resultBytes, locale)
    : `${formatBytes(file.originalBytes, locale)} → ${formatBytes(file.resultBytes, locale)}`;
  return (
    <li className={`flex items-center gap-4 py-2.5 ps-20 pe-4 ${file.exists ? "" : "text-text-muted"}`}>
      <span className="flex min-w-0 flex-1 flex-col">
        <span className="truncate" title={file.resultPath}>
          {fileName(file.resultPath)}
        </span>
        <span className="text-small text-text-muted">
          {file.exists ? sizes : t("history.missing")}
        </span>
      </span>
      {file.exists && <OpenButton file={file} />}
    </li>
  );
}

interface MessageProps {
  icon: typeof History;
  title: string;
  children: ReactNode;
  /** A button under the message. */
  action?: ReactNode;
}

function Message({ icon: Icon, title, children, action }: MessageProps) {
  return (
    <div className="flex flex-1 flex-col items-center justify-center gap-2 p-6 text-center">
      <Icon aria-hidden className="size-8 text-text-muted" />
      <h2 className="font-medium">{title}</h2>
      <p className="max-w-sm text-small text-text-muted">{children}</p>
      {action && <div className="mt-3">{action}</div>}
    </div>
  );
}
