import { TextField } from "react-aria-components";
import { FieldGroup, Input } from "@/components/ui/Field";
import { RadioCards } from "@/components/ui/RadioCards";
import { fileName } from "@/features/compress/imageApi";
import { pickPdfs } from "@/features/compress/pdfApi";
import { resultFiles, type TaskResult, type ToolOptions } from "@/features/tool/taskApi";
import { PageStrip } from "@/features/pages/PageStrip";
import { formatRanges, parseRanges } from "@/features/split/pageRanges";
import type { FileInfo } from "@/features/tool/fileInfo";
import { NumberInput } from "@/features/tool/NumberInput";
import { OptionField } from "@/features/tool/OptionField";
import { defineTool, type OptionsProps } from "@/features/tool/toolDef";
import { useT, type Translator } from "@/i18n";
import { isInt, useStoredState } from "@/lib/storedState";

type PagesMode = "all" | "take";
type Sharpness = "standard" | "sharp";

/** Dots per inch of the paper for each "Ketajaman". */
const DPI: Record<Sharpness, number> = { standard: 150, sharp: 300 };
const MIN_DPI = 72;
const MAX_DPI = 600;

export interface PdfToJpgSettings {
  /** "Semua halaman" or "Pilih halaman". */
  mode: PagesMode;
  /** "Halaman", such as "1-3, 5". Never kept: pages belong to a PDF, so each visit starts empty. */
  ranges: string;
  /** "Ketajaman". */
  sharpness: Sharpness;
  /** "Resolusi (DPI)" from Advanced options; wins over `sharpness`. */
  dpi: number | null;
}

function parse(stored: unknown): PdfToJpgSettings {
  const s = (stored ?? {}) as Partial<Record<keyof PdfToJpgSettings, unknown>>;
  return {
    mode: s.mode === "take" ? "take" : "all",
    ranges: "",
    sharpness: s.sharpness === "sharp" ? "sharp" : "standard",
    dpi: isInt(s.dpi, MIN_DPI, MAX_DPI) ? s.dpi : null,
  };
}

function usePdfToJpgSettings() {
  return useStoredState("rantakit.pdfKeJpg", parse);
}

function toRequest(settings: PdfToJpgSettings): ToolOptions {
  const dpi = settings.dpi ?? DPI[settings.sharpness];
  return settings.mode === "take"
    ? { tool: "pdf-ke-jpg", pages: parseRanges(settings.ranges) ?? [], dpi }
    : { tool: "pdf-ke-jpg", dpi };
}

/** A Preset keeps how sharp the photos are, not which pages: those belong to a PDF. */
function presetOptions({ sharpness, dpi }: PdfToJpgSettings): Partial<PdfToJpgSettings> {
  return { sharpness, dpi };
}

/** The one PDF's page count, when exactly one is chosen and has been read. */
function onlyPdfPages(files: FileInfo[], chosen: number) {
  return chosen === 1 ? (files[0]?.pages ?? null) : null;
}

function blocked(settings: PdfToJpgSettings, files: FileInfo[], chosen: number, { t, count }: Translator) {
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
  if (options.tool !== "pdf-ke-jpg" || !options.pages || options.pages.length === 0 || file.pages === null) return null;
  const has = file.pages;
  const missing = options.pages.filter((page) => page > has);
  if (missing.length === 0) return null;
  return missing.length === options.pages.length ? t("split.none") : t("split.skipped", { pages: formatRanges(missing) });
}

/** "12 foto JPG dibuat dari ijazah.pdf", or "20 foto JPG dibuat dari 3 PDF". */
function summary(results: TaskResult[], { t, count }: Translator) {
  const photos = count("pdfJpg.photos", results.reduce((sum, result) => sum + resultFiles(result).length, 0));
  const from = results.length === 1 ? fileName(results[0].inputPath) : count("files.pdf", results.length);
  return t("pdfJpg.result", { photos, from });
}

const MODES: PagesMode[] = ["all", "take"];
const SHARPNESS: Sharpness[] = ["standard", "sharp"];

function Main({ settings, onChange, files, paths }: OptionsProps<PdfToJpgSettings>) {
  const { t } = useT();
  const pages = onlyPdfPages(files, paths.length);
  return (
    <>
      <OptionField label={t("pdfJpg.pages")}>
        <RadioCards
          label={t("pdfJpg.pages")}
          value={settings.mode}
          options={MODES.map((mode) => ({
            id: mode,
            label: t(`pdfJpg.${mode}`),
            description: t(`pdfJpg.${mode}.description`),
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
      <OptionField label={t("pdfJpg.sharpness")}>
        <RadioCards
          label={t("pdfJpg.sharpness")}
          value={settings.sharpness}
          options={SHARPNESS.map((sharpness) => ({
            id: sharpness,
            label: t(`pdfJpg.${sharpness}`),
            description: t(`pdfJpg.${sharpness}.description`),
          }))}
          onChange={(sharpness) => onChange({ sharpness })}
        />
      </OptionField>
    </>
  );
}

function Advanced({ settings, onChange }: OptionsProps<PdfToJpgSettings>) {
  const { t } = useT();
  return (
    <OptionField label={t("pdfJpg.dpi")} help={t("pdfJpg.dpi.help")} description={t("resize.side.overrides")}>
      <NumberInput
        label={t("pdfJpg.dpi")}
        minValue={MIN_DPI}
        maxValue={MAX_DPI}
        placeholder={String(DPI[settings.sharpness])}
        value={settings.dpi}
        onChange={(dpi) =>
          onChange({ dpi: dpi === null ? null : Math.min(MAX_DPI, Math.max(MIN_DPI, Math.round(dpi))) })
        }
      />
    </OptionField>
  );
}

export const pdfKeJpg = defineTool({
  noun: "pdf",
  pick: pickPdfs,
  useSettings: usePdfToJpgSettings,
  Main,
  Advanced,
  toRequest,
  presetOptions,
  blocked,
  fileNote,
  summary,
});
