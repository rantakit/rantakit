import { File, Image, Plus } from "@keyline-icons/react";
import { FileArrowUp } from "@keyline-icons/react/two-tone";
import { FileRow } from "@/components/blocks/FileRow";
import { Button } from "@/components/ui/Button";
import { fileName } from "@/features/compress/imageApi";
import { useT } from "@/i18n";
import type { JobItem } from "@/features/compress/useCompressJob";
import { formatBytes } from "@/lib/format";
import { isAndroidApp } from "@/lib/platform";
import { useFileInfo, useFileTile, type FileInfo } from "./fileInfo";
import { OrderedFiles } from "./OrderedFiles";
import type { RowExtras } from "./toolDef";
import { unchangedNote } from "./unchanged";

export const NOUN_ICONS = { photo: Image, pdf: File };

interface FilesColumnProps {
  noun: "photo" | "pdf";
  paths: string[];
  /** While a Task runs: each file's progress, in place of the remove buttons. */
  items?: JobItem[];
  /** "Sudah JPG, tidak diubah" for a file the chosen options would leave as it is. */
  skipNote?: (info: FileInfo) => string | null;
  onPick: () => void;
  onRemove: (path: string) => void;
  /** Tools that make one Result from several files: the user sets the order (X2). */
  onReorder?: (paths: string[]) => void;
  /** What an ordered row shows in place of the size (D1). */
  useRow?: (path: string, paths: string[]) => RowExtras;
}

/**
 * The Files step: one drop zone while empty, then a row per file and "Tambah
 * foto". On desktop the count and "Tambah foto" stay put while the list scrolls.
 */
export function FilesColumn({ noun, paths, items, skipNote, onPick, onRemove, onReorder, useRow }: FilesColumnProps) {
  const tr = useT();
  const { t, count, locale } = tr;
  const infos = useFileInfo(paths);

  if (paths.length === 0) {
    return (
      <div className="mx-5 mb-5 flex min-h-80 flex-1 flex-col items-center justify-center gap-4 rounded-[20px] border-2 border-dashed border-border-strong p-8 text-center md:min-h-0">
        <span className="flex size-18 items-center justify-center rounded-[20px] bg-brand-soft text-brand-strong">
          <FileArrowUp aria-hidden className="size-9" />
        </span>
        <Button onPress={onPick}>{t(`files.pick.${noun}`)}</Button>
        <div className="flex flex-col gap-1">
          {/* Nothing is dragged into an app on a phone. */}
          {!isAndroidApp && <p>{t("files.dropHere")}</p>}
          <p className="text-small text-text-muted">{t(`formats.${noun}`)}</p>
        </div>
      </div>
    );
  }

  const total = paths.reduce((sum, path) => sum + (infos[path]?.size ?? 0), 0);
  return (
    <div className="flex flex-1 flex-col md:min-h-0">
      <div className="flex items-baseline justify-between gap-3 px-5 pb-3">
        <h2 className="text-section">{count(`files.${noun}`, paths.length)}</h2>
        {total > 0 && (
          <span className="text-small text-text-muted">{t("files.total", { size: formatBytes(total, locale) })}</span>
        )}
      </div>

      <div className="px-5 md:min-h-0 md:flex-1 md:overflow-y-auto">
        {onReorder && !items ? (
          <OrderedFiles
            noun={noun}
            paths={paths}
            infos={infos}
            onReorder={onReorder}
            onRemove={onRemove}
            useRow={useRow}
          />
        ) : (
          <ul
            aria-label={t("files.list")}
            className="flex flex-col divide-y divide-border rounded-2xl border border-border bg-surface"
          >
            {items
              ? items.map((item) => <WorkingRow key={item.path} item={item} noun={noun} />)
              : paths.map((path) => (
                  <ReadyRow
                    key={path}
                    path={path}
                    noun={noun}
                    info={infos[path]}
                    skipNote={infos[path] ? skipNote?.(infos[path]) : null}
                    onRemove={() => onRemove(path)}
                  />
                ))}
          </ul>
        )}
      </div>

      <div className={`flex flex-wrap items-center gap-3 px-5 py-4 ${items ? "invisible" : ""}`}>
        <Button variant="secondary" size="row" onPress={onPick}>
          <Plus aria-hidden className="size-5" /> {t(`files.add.${noun}`)}
        </Button>
        {!isAndroidApp && <span className="text-small text-text-muted">{t(`files.dropMore.${noun}`)}</span>}
      </div>
    </div>
  );
}

function ReadyRow({
  path,
  noun,
  info,
  skipNote,
  onRemove,
}: {
  path: string;
  noun: "photo" | "pdf";
  info?: FileInfo;
  skipNote?: string | null;
  onRemove: () => void;
}) {
  const { t, count, locale } = useT();
  const tile = useFileTile(path, noun, 96);
  const name = fileName(path);
  const meta = [
    info?.pages != null ? count("pages", info.pages) : null,
    info?.size != null ? formatBytes(info.size, locale) : null,
    skipNote,
  ].filter(Boolean);

  return (
    <FileRow
      name={name}
      title={path}
      icon={NOUN_ICONS[noun]}
      {...tile}
      status="ready"
      note={meta.join(" · ")}
      onRemove={onRemove}
      removeLabel={t("setup.remove", { name })}
    />
  );
}

/** A file while its Task runs: waiting, a progress bar, or done with its new size. */
function WorkingRow({ item, noun }: { item: JobItem; noun: "photo" | "pdf" }) {
  const { t, locale } = useT();
  const tile = useFileTile(item.path, noun, 96);
  const common = { name: item.name, title: item.path, icon: NOUN_ICONS[noun], ...tile };
  switch (item.status) {
    case "pending":
      return <FileRow {...common} status="waiting" note={t("task.file.waiting")} />;
    case "working":
      return (
        <FileRow
          {...common}
          status="working"
          fraction={item.fraction}
          note={t("task.file.working", { percent: Math.round(item.fraction * 100) })}
        />
      );
    case "done":
      return (
        <FileRow
          {...common}
          status="done"
          note={t("task.file.saved", { size: formatBytes(item.result.compressedBytes, locale) })}
        />
      );
    case "cancelled":
      return <FileRow {...common} status="skipped" note={t("task.file.cancelled")} />;
    case "unchanged":
      return <FileRow {...common} status="skipped" note={unchangedNote(t, item.options)} />;
    case "failed":
      return <FileRow {...common} status="failed" note={t("task.file.failed")} />;
  }
}
