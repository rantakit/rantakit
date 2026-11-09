import { useRef, useState, type ComponentType } from "react";
import { flushSync } from "react-dom";
import { revealItemInDir } from "@tauri-apps/plugin-opener";
import {
  CircleCheck,
  CircleStop,
  FolderOpen,
  Share2,
  SquareArrowOutUpRight,
  TriangleAlert,
  type IconProps,
} from "@keyline-icons/react";
import { ProblemRow } from "@/components/blocks/ProblemRow";
import { Button } from "@/components/ui/Button";
import { fileName } from "@/features/compress/imageApi";
import { withSmallerImages } from "@/features/compress/imageSettings";
import { limitOf, resultFiles, type ResultPart, type TaskResult } from "./taskApi";
import type { CompressJob, JobItem } from "./useCompressJob";
import { useT, type Translator } from "@/i18n";
import { errorMessage } from "@/lib/errorMessages";
import { openPath, shareFiles } from "@/lib/files";
import { formatBytes } from "@/lib/format";
import { canShare, canShowInFolder, isAndroidApp } from "@/lib/platform";
import { tv } from "@/lib/react-aria-utils";
import { nounOf, useFileTile, type Noun } from "./fileInfo";
import { iconFor } from "./FilesColumn";
import { unchangedNote } from "./unchanged";

interface ResultSummaryProps {
  job: CompressJob;
  noun: Noun;
  /** The Tool's own line for what happened (X4); left out by the compress Tools. */
  summary?: (results: TaskResult[], tr: Translator) => string | null;
  /** Whether every file went into one Result, as in Gabung PDF. */
  merges: boolean;
  /** Where Results were saved; null next to each Original. */
  resultsFolder: string | null;
}

const card = tv({
  base: "flex items-start gap-4 rounded-[20px] border p-5",
  variants: {
    tone: {
      done: "border-border bg-surface",
      partial: "border-transparent bg-warn-soft",
      cancelled: "border-border bg-surface",
    },
  },
});

const cardIcon = tv({
  base: "mt-0.5 size-7 shrink-0",
  variants: { tone: { done: "text-success", partial: "text-warn", cancelled: "text-text-muted" } },
});

const TONE_ICONS: Record<"done" | "partial" | "cancelled", ComponentType<IconProps>> = {
  done: CircleCheck,
  partial: TriangleAlert,
  cancelled: CircleStop,
};

/** How a Task went, in the Tool page's side panel; "Proses lagi" sits below it. */
export function ResultSummary({ job, noun, summary, merges, resultsFolder }: ResultSummaryProps) {
  const tr = useT();
  const { t, count, locale } = tr;
  const { items } = job;
  const done = items.flatMap((item) => (item.status === "done" ? [item] : []));
  const cancelled = items.some((item) => item.status === "cancelled");
  const failed = items.filter((item) => item.status === "failed").length;
  const unchanged = items.filter((item) => item.status === "unchanged").length;
  const finished = done.length + unchanged;
  const tone = cancelled ? "cancelled" : failed > 0 ? "partial" : "done";
  const files = count(`files.${noun}`, items.length);
  const limit = done.length > 0 ? limitOf(done[0].options) : undefined;
  // One Original that made several Results counts the files made (X3).
  const made = done.flatMap((item) => resultFiles(item.result)).length;
  const many = done.some((item) => item.result.files);

  // A merge that stopped names the file that stopped it; nothing else was made.
  const stopper = merges ? items.find((item) => item.status === "failed") : undefined;
  const said = done.length > 0 ? (summary?.(done.map((item) => item.result), tr) ?? null) : null;

  // A merge makes one PDF, whatever went into it.
  const one = count("files.pdf", 1);
  const title = {
    done: many ? count("result.made", made) : t("result.done", { files: merges ? one : files }),
    partial: t("result.partial", { done: finished, files }),
    cancelled: t("result.cancelled", { done: merges ? 0 : finished, files: merges ? one : files }),
  }[tone];
  // Android always saves to Download/Rantakit/, whatever the desktop setting says.
  const folder = isAndroidApp ? "Download/Rantakit" : resultsFolder;
  const saved = folder ? t("result.savedIn", { folder }) : t("result.savedNext");
  if (stopper?.status === "failed") {
    const error = errorMessage(tr, stopper.error, { accepted: t(`formats.${noun}`), canShrink: false });
    return (
      <div className={card({ tone: "partial" })}>
        <TriangleAlert aria-hidden className={cardIcon({ tone: "partial" })} />
        <div className="flex min-w-0 flex-1 flex-col gap-1">
          <h2 className="font-display text-section">{t(noun === "photo" ? "photoPdf.failed" : "merge.failed")}</h2>
          <p className="text-text-muted">{t("merge.failed.body", { name: stopper.name, error })}</p>
        </div>
      </div>
    );
  }
  const body = {
    done:
      done.length === 0
        ? t("result.nothingChanged")
        : said
          ? `${said} ${saved}`
          : limit
            ? `${t("result.underLimit.all", { limit: formatBytes(limit, locale) })} ${saved}`
            : saved,
    partial: t("result.partial.body", { failed: count(`files.${noun}`, failed) }),
    cancelled: t(`result.cancelled.body.${noun}`),
  }[tone];
  const ToneIcon = TONE_ICONS[tone];

  return (
    <div className={card({ tone })}>
      <ToneIcon aria-hidden className={cardIcon({ tone })} />
      <div className="flex min-w-0 flex-1 flex-col gap-1">
        <h2 className="font-display text-section">{title}</h2>
        <p className="text-text-muted">{body}</p>
      </div>
    </div>
  );
}


interface ResultListProps {
  job: CompressJob;
  noun: Noun;
  /** Whether sizes show as before → after; false when the Tool is not about size (X4). */
  comparesSizes: boolean;
  /** Whether every file went into one Result, as in Gabung PDF: it shows once. */
  merges: boolean;
}

/** Each Result with its sizes and actions, in the Tool page's main column; scrolls on its own on desktop. */
export function ResultList({ job, noun, comparesSizes, merges }: ResultListProps) {
  const tr = useT();
  const { t } = tr;
  const done = job.items.flatMap((item) => (item.status === "done" ? [item] : []));
  // A merge made one Result from all of them, or nothing: then the file that stopped it.
  const items = merges ? (done.length > 0 ? done.slice(0, 1) : job.items.filter((item) => item.status === "failed")) : job.items;
  const made = merges ? done.slice(0, 1).flatMap((item) => resultFiles(item.result)) : done.flatMap((item) => resultFiles(item.result));
  // One Original that made several Results: list the files in its new folder (X3).
  const one = items.length === 1 && items[0].status === "done" ? items[0] : null;
  const folder = one?.result.files ? one.result : null;

  return (
    <div className="flex flex-1 flex-col md:min-h-0">
      <div className="flex flex-wrap items-center justify-between gap-3 px-5 pb-3">
        <h2 className="text-section">{t("result.list")}</h2>
        <div className="flex flex-wrap items-center gap-2">
          {folder && canShowInFolder && (
            <Button variant="secondary" size="row" onPress={() => openPath(folder.outputPath)}>
              <FolderOpen aria-hidden className="size-[18px]" /> {t("result.openFolder")}
            </Button>
          )}
          {canShare && made.length > 1 && (
            <Button variant="secondary" size="row" onPress={() => shareFiles(made.map((part) => part.path))}>
              <Share2 aria-hidden className="size-[18px]" /> {t("result.shareAll")}
            </Button>
          )}
        </div>
      </div>
      <div className="px-5 pb-5 md:min-h-0 md:flex-1 md:overflow-y-auto">
        {folder ? (
          <FolderResults parts={resultFiles(folder)} />
        ) : one ? (
          // A merge's one Result is a PDF, even from photos.
          <OneResult item={one} noun={merges ? "pdf" : noun} comparesSizes={comparesSizes} />
        ) : (
          <ul
            aria-label={t("result.list")}
            className="flex flex-col divide-y divide-border overflow-hidden rounded-2xl border border-border bg-surface"
          >
            {items.map((item, index) => (
              <ResultRow
                key={item.path}
                tr={tr}
                item={item}
                noun={noun}
                comparesSizes={comparesSizes}
                canRetry={!job.isRunning && !merges}
                onRetry={(options) => job.retry(index, options)}
              />
            ))}
          </ul>
        )}
      </div>
    </div>
  );
}

type DoneItem = Extract<JobItem, { status: "done" }>;

/** Buka, then Tampilkan di folder on desktop and Bagikan on Windows and Android. */
function ResultActions({ path, name }: { path: string; name: string }) {
  const { t } = useT();
  return (
    <div className="flex shrink-0 flex-wrap items-center gap-2">
      <Button variant="secondary" size="row" aria-label={t("result.openName", { name })} onPress={() => openPath(path)}>
        <SquareArrowOutUpRight aria-hidden className="size-[18px]" /> {t("result.open")}
      </Button>
      {canShowInFolder && (
        <Button variant="secondary" size="row" onPress={() => revealItemInDir(path)}>
          <FolderOpen aria-hidden className="size-[18px]" /> {t("task.showInFolder")}
        </Button>
      )}
      {canShare && (
        <Button variant="secondary" size="row" onPress={() => shareFiles([path])}>
          <Share2 aria-hidden className="size-[18px]" /> {t("result.share")}
        </Button>
      )}
    </div>
  );
}

/** Buka folder on desktop and Bagikan semua, for one Original that made several Results. */
function FolderActions({ result }: { result: TaskResult }) {
  const { t } = useT();
  return (
    <div className="flex shrink-0 flex-wrap items-center gap-2">
      {canShowInFolder && (
        <Button variant="secondary" size="row" onPress={() => openPath(result.outputPath)}>
          <FolderOpen aria-hidden className="size-[18px]" /> {t("result.openFolder")}
        </Button>
      )}
      {canShare && (
        <Button
          variant="secondary"
          size="row"
          onPress={() => shareFiles(resultFiles(result).map((part) => part.path))}
        >
          <Share2 aria-hidden className="size-[18px]" /> {t("result.shareAll")}
        </Button>
      )}
    </div>
  );
}

function UnderLimit({ item }: { item: DoneItem }) {
  const { t, locale } = useT();
  const limit = limitOf(item.options);
  if (!limit || item.result.compressedBytes > limit) return null;
  return (
    <span className="inline-flex items-center gap-1 rounded-full bg-success-soft px-2.5 py-0.5 text-small font-semibold text-success">
      <CircleCheck aria-hidden className="size-4" />
      {t("result.underLimit", { limit: formatBytes(limit, locale) })}
    </span>
  );
}

function Sizes({ result, comparesSizes }: { result: TaskResult; comparesSizes: boolean }) {
  const { locale } = useT();
  return (
    <span className="text-small text-text-muted">
      {comparesSizes && `${formatBytes(result.originalBytes, locale)} → `}
      {formatBytes(result.compressedBytes, locale)}
    </span>
  );
}

function ResultTile({ path, noun, size }: { path: string; noun: Noun; size: number }) {
  const { thumbnail, thumbnailKind } = useFileTile(path, noun, size);
  const Icon = iconFor(noun, path);
  return (
    <span
      aria-hidden
      className="flex size-14 shrink-0 items-center justify-center overflow-hidden rounded-xl bg-badge-bg text-text-muted"
    >
      {!thumbnail ? (
        <Icon className="size-6" />
      ) : thumbnailKind === "page" ? (
        <img src={thumbnail} alt="" className="max-h-12 max-w-12 rounded-[2px] border border-page-border bg-page" />
      ) : (
        <img src={thumbnail} alt="" className="size-full object-cover" />
      )}
    </span>
  );
}

/** How many made files show before "Tampilkan N file lagi". */
const FIRST_PARTS = 5;

/** Every file one Original made, in order, each with its own actions. */
function FolderResults({ parts }: { parts: ResultPart[] }) {
  const { t, locale } = useT();
  // A short list shows whole; one more hidden file is not worth a button.
  const [expanded, setExpanded] = useState(parts.length <= FIRST_PARTS + 1);
  const list = useRef<HTMLUListElement>(null);
  const shown = expanded ? parts : parts.slice(0, FIRST_PARTS);
  // The button goes away, so focus moves on to the first file it showed.
  const showAll = () => {
    flushSync(() => setExpanded(true));
    list.current?.children[FIRST_PARTS]?.querySelector("button")?.focus();
  };

  return (
    <div className="flex flex-col gap-3">
      <ul
        ref={list}
        aria-label={t("result.list")}
        className="flex flex-col divide-y divide-border overflow-hidden rounded-2xl border border-border bg-surface"
      >
        {shown.map((part) => {
          const name = fileName(part.path);
          return (
            <li key={part.path} className="flex flex-wrap items-center gap-4 px-4 py-3.5">
              <ResultTile path={part.path} noun={nounOf(part.path)} size={112} />
              <span className="flex min-w-0 flex-1 basis-40 flex-col items-start gap-1">
                <span className="max-w-full truncate text-label" title={part.path}>
                  {name}
                </span>
                <span className="text-small text-text-muted">{formatBytes(part.bytes, locale)}</span>
              </span>
              <ResultActions path={part.path} name={name} />
            </li>
          );
        })}
      </ul>
      {!expanded && (
        <Button variant="link" className="self-start" onPress={showAll}>
          {t("result.showMore", { count: parts.length - FIRST_PARTS })}
        </Button>
      )}
    </div>
  );
}

interface ResultRowProps {
  tr: Translator;
  item: JobItem;
  noun: Noun;
  comparesSizes: boolean;
  canRetry: boolean;
  onRetry: (options?: JobItem["options"]) => void;
}

function ResultRow({ tr, item, noun, comparesSizes, canRetry, onRetry }: ResultRowProps) {
  const { t, count } = tr;
  const common = { name: item.name, title: item.path, icon: iconFor(noun, item.path) };

  switch (item.status) {
    case "done": {
      const { result } = item;
      const name = fileName(result.outputPath);
      const parts = result.files;
      return (
        <li className="flex flex-wrap items-center gap-4 px-4 py-3.5">
          <ResultTile path={parts ? parts[0].path : result.outputPath} noun={parts ? nounOf(parts[0].path) : noun} size={112} />
          {/* At least 10rem for the name; on a phone the actions wrap below it. */}
          <span className="flex min-w-0 flex-1 basis-40 flex-col items-start gap-1">
            <span className="max-w-full truncate text-label" title={result.outputPath}>
              {name}
            </span>
            <span className="flex flex-wrap items-center gap-2">
              {parts ? (
                <span className="text-small text-text-muted">{count("files.any", parts.length)}</span>
              ) : (
                <Sizes result={result} comparesSizes={comparesSizes} />
              )}
              <UnderLimit item={item} />
            </span>
          </span>
          {parts ? <FolderActions result={result} /> : <ResultActions path={result.outputPath} name={name} />}
        </li>
      );
    }
    case "unchanged":
      return <ProblemRow {...common} tone="unchanged" message={unchangedNote(t, item.options)} />;
    case "cancelled":
    case "pending":
    case "working":
      return <ProblemRow {...common} tone="cancelled" message={t("task.file.cancelled")} />;
    case "failed": {
      const smaller = item.error.code === "size-limit-unreachable" ? withSmallerImages(item.options) : null;
      return (
        <ProblemRow
          {...common}
          tone={smaller || item.error.code === "size-limit-unreachable" ? "limit" : "failed"}
          message={errorMessage(tr, item.error, {
            accepted: t(`formats.${noun}`),
            canShrink: smaller !== null && canRetry,
          })}
          action={
            !canRetry
              ? undefined
              : smaller
                ? { label: t("task.shrink"), onPress: () => onRetry(smaller) }
                : { label: t("task.retry"), onPress: () => onRetry() }
          }
        />
      );
    }
  }
}

interface OneResultProps {
  item: DoneItem;
  noun: Noun;
  comparesSizes: boolean;
}

/** A Task with one file: a large preview, the sizes before and after, and the actions. */
function OneResult({ item, noun, comparesSizes }: OneResultProps) {
  const { t, locale } = useT();
  const { thumbnail: preview, thumbnailKind } = useFileTile(item.result.outputPath, noun, 800);
  const Icon = iconFor(noun, item.result.outputPath);
  const name = fileName(item.result.outputPath);

  return (
    <div className="flex flex-col gap-6 rounded-[20px] border border-border bg-surface p-5 xl:flex-row xl:items-center">
      <div className="flex h-[280px] w-full shrink-0 items-center justify-center overflow-hidden rounded-2xl bg-badge-bg text-text-muted xl:w-[400px]">
        {preview && thumbnailKind === "page" ? (
          <img src={preview} alt="" className="max-h-[248px] max-w-[90%] rounded-[3px] border border-page-border bg-page" />
        ) : preview ? (
          <img src={preview} alt="" className="size-full object-contain" />
        ) : (
          <Icon aria-hidden className="size-12" />
        )}
      </div>
      <div className="flex min-w-0 flex-1 flex-col items-start gap-4">
        <div className="flex max-w-full flex-col gap-1">
          <span className="text-small font-semibold text-text-muted">{t("result.label")}</span>
          <span className="truncate text-label" title={item.result.outputPath}>
            {name}
          </span>
          {!comparesSizes && <Sizes result={item.result} comparesSizes={false} />}
        </div>
        {comparesSizes && (
          <dl className="flex gap-8">
            <div className="flex flex-col gap-0.5">
              <dt className="text-small text-text-muted">{t("result.before")}</dt>
              <dd className="text-section">{formatBytes(item.result.originalBytes, locale)}</dd>
            </div>
            <div className="flex flex-col gap-0.5">
              <dt className="text-small text-text-muted">{t("result.after")}</dt>
              <dd className="text-section text-brand-strong">{formatBytes(item.result.compressedBytes, locale)}</dd>
            </div>
          </dl>
        )}
        <UnderLimit item={item} />
        <ResultActions path={item.result.outputPath} name={name} />
      </div>
    </div>
  );
}
