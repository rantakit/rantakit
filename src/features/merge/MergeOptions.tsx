import { Layers } from "@keyline-icons/react";
import { pickPdfs } from "@/features/compress/pdfApi";
import type { TaskResult } from "@/features/tool/taskApi";
import type { FileInfo } from "@/features/tool/fileInfo";
import { defineTool, type OptionsProps } from "@/features/tool/toolDef";
import { useT, type Translator } from "@/i18n";
import { today } from "@/lib/format";

/** Gabung PDF has no options. */
type MergeSettings = Record<string, never>;

const NO_SETTINGS: MergeSettings = {};
const useMergeSettings = () => [NO_SETTINGS, () => {}] as const;

/** Every page of the chosen PDFs, once each has been read. */
function totalPages(files: FileInfo[], count: number) {
  if (files.length < count || files.some((file) => file.pages === null)) return null;
  return files.reduce((sum, file) => sum + (file.pages ?? 0), 0);
}

/** "5 PDF · 23 halaman": what goes into the one Result. */
function Main({ files, paths }: OptionsProps<MergeSettings>) {
  const { t, count } = useT();
  const pages = totalPages(files, paths.length);
  if (paths.length === 0) return null;
  return (
    <div className="flex items-center gap-3 rounded-2xl border border-border bg-surface p-4">
      <span className="flex size-10 shrink-0 items-center justify-center rounded-xl bg-brand-soft text-brand-strong">
        <Layers aria-hidden className="size-5" />
      </span>
      <span className="text-label font-semibold">
        {pages === null
          ? count("files.pdf", paths.length)
          : t("merge.summary", { files: count("files.pdf", paths.length), pages: count("pages", pages) })}
      </span>
    </div>
  );
}

function blocked(_settings: MergeSettings, _files: FileInfo[], chosen: number, { t }: Translator) {
  return chosen < 2 ? t("merge.needTwo") : null;
}

/** "5 PDF → 1 PDF, 23 halaman." Every file's outcome points at the same Result. */
function summary(results: TaskResult[], { t, count }: Translator) {
  const pages = results[0]?.pages ?? 0;
  return t("merge.result", { files: count("files.pdf", results.length), pages: count("pages", pages) });
}

export const gabungPdf = defineTool({
  noun: "pdf",
  ordered: true,
  presets: false,
  pick: pickPdfs,
  useSettings: useMergeSettings,
  Main,
  toRequest: () => ({ tool: "gabung-pdf", date: today() }),
  blocked,
  summary,
});
