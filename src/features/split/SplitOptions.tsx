import { Check } from "@keyline-icons/react";
import { TextField, ToggleButton } from "react-aria-components";
import { FieldGroup, Input } from "@/components/ui/Field";
import { RadioCards } from "@/components/ui/RadioCards";
import { fileName } from "@/features/compress/imageApi";
import { pickPdfs } from "@/features/compress/pdfApi";
import type { TaskResult, ToolOptions } from "@/features/compress/taskApi";
import { PageThumbnail } from "@/features/pages/PageThumbnail";
import type { FileInfo } from "@/features/tool/fileInfo";
import { NumberInput } from "@/features/tool/NumberInput";
import { OptionField } from "@/features/tool/OptionField";
import { defineTool, type OptionsProps } from "@/features/tool/toolDef";
import { useT, type Translator } from "@/i18n";
import { focusRing, tv } from "@/lib/react-aria-utils";
import { isInt, useStoredState } from "@/lib/storedState";
import { formatRanges, parseRanges, togglePage } from "./pageRanges";

type SplitMode = "take" | "all";

export interface SplitSettings {
  /** "Ambil halaman" or "Pisah semua halaman". */
  mode: SplitMode;
  /** "Halaman", such as "1-3, 5". Kept in a Preset, but each visit starts empty: pages belong to a PDF. */
  ranges: string;
  /** "Setiap N halaman" for "Pisah semua halaman"; null is every page. */
  every: number | null;
}

function parse(stored: unknown): SplitSettings {
  const s = (stored ?? {}) as Partial<Record<keyof SplitSettings, unknown>>;
  return {
    mode: s.mode === "all" ? "all" : "take",
    ranges: "",
    every: isInt(s.every, 1, 10_000) ? s.every : null,
  };
}

function useSplitSettings() {
  return useStoredState("rantakit.pisahPdf", parse);
}

function toRequest(settings: SplitSettings): ToolOptions {
  return settings.mode === "take"
    ? { tool: "pisah-pdf", pages: parseRanges(settings.ranges) ?? [] }
    : { tool: "pisah-pdf", every: settings.every ?? 1 };
}

/** The one PDF's page count, when exactly one is chosen and has been read. */
function onlyPdfPages(files: FileInfo[], chosen: number) {
  return chosen === 1 ? (files[0]?.pages ?? null) : null;
}

function blocked(settings: SplitSettings, files: FileInfo[], chosen: number, { t, count }: Translator) {
  if (settings.mode === "all") return null;
  const pages = parseRanges(settings.ranges);
  if (pages === null) return t("split.range.invalid");
  if (pages.length === 0) return t("split.range.empty");
  const has = onlyPdfPages(files, chosen);
  if (has !== null && pages[pages.length - 1] > has) return t("split.range.outside", { pages: count("pages", has) });
  return null;
}

/** With several PDFs, a page one of them doesn't have is skipped; its row says so first. */
function fileNote(options: ToolOptions, file: FileInfo, { t }: Translator) {
  if (options.tool !== "pisah-pdf" || !options.pages || options.pages.length === 0 || file.pages === null) return null;
  const has = file.pages;
  const missing = options.pages.filter((page) => page > has);
  if (missing.length === 0) return null;
  return missing.length === options.pages.length ? t("split.none") : t("split.skipped", { pages: formatRanges(missing) });
}

function summary(results: TaskResult[], { t, count }: Translator) {
  if (results.some((result) => result.files)) return t("split.each");
  return results.length === 1
    ? t("split.taken", { pages: count("pages", results[0].pages ?? 0) })
    : t("split.taken.many");
}

const MODES: SplitMode[] = ["take", "all"];

function Main({ settings, onChange, files, paths }: OptionsProps<SplitSettings>) {
  const { t } = useT();
  const pages = onlyPdfPages(files, paths.length);
  return (
    <>
      <OptionField label={t("split.mode")}>
        <RadioCards
          label={t("split.mode")}
          value={settings.mode}
          options={MODES.map((mode) => ({
            id: mode,
            label: t(`split.${mode}`),
            description: t(`split.${mode}.description`),
          }))}
          onChange={(mode) => onChange({ mode })}
        />
      </OptionField>
      {settings.mode === "take" && (
        <>
          <OptionField label={t("split.range")} description={t("split.range.help")}>
            <TextField
              aria-label={t("split.range")}
              value={settings.ranges}
              onChange={(ranges) => onChange({ ranges })}
              className="min-w-0"
            >
              <FieldGroup className="h-11">
                <Input placeholder={t("split.range.placeholder")} inputMode="numeric" className="min-h-11 w-full" />
              </FieldGroup>
            </TextField>
          </OptionField>
          {pages !== null && (
            <PageStrip
              path={paths[0]}
              pages={pages}
              ranges={settings.ranges}
              onChange={(ranges) => onChange({ ranges })}
            />
          )}
        </>
      )}
    </>
  );
}

const stripPage = tv({
  extend: focusRing,
  base: "relative w-16 shrink-0 snap-start rounded-md p-0.5 cursor-default transition-opacity [-webkit-tap-highlight-color:transparent]",
  variants: {
    dimmed: { true: "opacity-55 hover:opacity-80" },
  },
});

interface PageStripProps {
  path: string;
  pages: number;
  ranges: string;
  onChange: (ranges: string) => void;
}

/**
 * The one PDF's pages under the range field. Tapping a page adds it to the
 * range or takes it out; typing a range picks the pages. Both always agree,
 * because the pages shown as picked are read from the range itself.
 */
function PageStrip({ path, pages, ranges, onChange }: PageStripProps) {
  const { t } = useT();
  const picked = parseRanges(ranges) ?? [];
  const name = fileName(path);
  return (
    <div className="flex flex-col gap-2">
      <div
        role="group"
        aria-label={t("split.strip", { name })}
        className="-mx-2 flex snap-x scroll-px-2 gap-2.5 overflow-x-auto px-2 pt-2 pb-2"
      >
        {Array.from({ length: pages }, (_, index) => {
          const page = index + 1;
          const isPicked = picked.includes(page);
          return (
            <ToggleButton
              key={page}
              aria-label={t("page.label", { page, pages })}
              isSelected={isPicked}
              onChange={() => onChange(formatRanges(togglePage(picked, page)))}
              className={(renderProps) => stripPage({ ...renderProps, dimmed: picked.length > 0 && !isPicked })}
            >
              <PageThumbnail path={path} page={index} pages={pages} size={128} selected={isPicked} />
              {isPicked && (
                <span
                  aria-hidden
                  className="absolute end-0 top-0 flex size-5 items-center justify-center rounded-full bg-brand-strong text-on-primary"
                >
                  <Check className="size-3.5" />
                </span>
              )}
              <span aria-hidden className="mt-1 block text-center text-small text-text-muted">
                {page}
              </span>
            </ToggleButton>
          );
        })}
      </div>
      <p className="text-small text-text-muted">{t("split.strip.hint")}</p>
    </div>
  );
}

function Advanced({ settings, onChange }: OptionsProps<SplitSettings>) {
  const { t } = useT();
  return (
    <OptionField label={t("split.every")} help={t("split.every.help")}>
      <NumberInput
        label={t("split.every")}
        minValue={1}
        maxValue={10_000}
        placeholder="1"
        value={settings.every}
        onChange={(every) => onChange({ every: every === null ? null : Math.max(1, Math.round(every)) })}
      />
    </OptionField>
  );
}

export const pisahPdf = defineTool({
  noun: "pdf",
  pick: pickPdfs,
  useSettings: useSplitSettings,
  Main,
  Advanced,
  // "Setiap N halaman" only changes how "Pisah semua halaman" cuts.
  showAdvanced: (settings) => settings.mode === "all",
  toRequest,
  blocked,
  fileNote,
  summary,
});
