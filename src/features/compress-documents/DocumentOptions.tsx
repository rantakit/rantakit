import { Select, SelectItem } from "@/components/ui/Select";
import { MAX_DPIS } from "@/features/compress/pdfSettings";
import { levelOf, limitBytes, parseShrink, type ShrinkSettings } from "@/features/compress/shrink";
import { ShrinkOptions } from "@/features/compress/ShrinkOptions";
import type { ToolOptions } from "@/features/tool/taskApi";
import type { FileInfo } from "@/features/tool/fileInfo";
import { NumberInput } from "@/features/tool/NumberInput";
import { OptionField } from "@/features/tool/OptionField";
import { defineTool, type OptionsProps } from "@/features/tool/toolDef";
import { useT, type Translator } from "@/i18n";
import { isInt, useStoredState } from "@/lib/storedState";
import { pickDocuments } from "./documentApi";

export interface DocumentSettings extends ShrinkSettings {
  /** "Kualitas gambar (1–100)"; null lets Rantakit choose. */
  quality: number | null;
  /** "Resolusi gambar maksimum"; 0 leaves the pictures' size as it is. */
  maxDpi: number;
}

function parse(stored: unknown): DocumentSettings {
  const s = (stored ?? {}) as Partial<Record<keyof DocumentSettings, unknown>>;
  return {
    ...parseShrink(stored),
    quality: isInt(s.quality, 1, 100) ? s.quality : null,
    maxDpi: MAX_DPIS.includes(s.maxDpi as number) ? (s.maxDpi as number) : 0,
  };
}

/** Kompres Dokumen's options, remembered for the next visit. */
function useDocumentSettings() {
  return useStoredState("rantakit.kompresDokumen", parse);
}

function toRequest(settings: DocumentSettings): ToolOptions {
  return {
    tool: "kompres-dokumen",
    limit: limitBytes(settings),
    level: levelOf(settings),
    quality: settings.quality ?? undefined,
    maxDpi: settings.maxDpi || undefined,
  };
}

/** Only pictures get smaller, so a document without any likely stays as it is. */
function fileNote(_options: ToolOptions, file: FileInfo, { t }: Translator) {
  return file.pictures === 0 ? t("document.noPictures") : null;
}

function Main({ settings, onChange }: OptionsProps<DocumentSettings>) {
  const { t } = useT();
  return (
    <>
      <ShrinkOptions noun="document" settings={settings} onChange={onChange} />
      <p className="text-small text-text-muted">{t("options.document.intro")}</p>
    </>
  );
}

function Advanced({ settings, onChange }: OptionsProps<DocumentSettings>) {
  const { t } = useT();
  return (
    <>
      <OptionField
        label={t("options.quality.pdf")}
        help={t("options.quality.document.help")}
        description={t("options.quality.overrides")}
      >
        <NumberInput
          label={t("options.quality.pdf")}
          minValue={1}
          maxValue={100}
          placeholder={t("options.quality.auto")}
          value={settings.quality}
          onChange={(quality) => onChange({ quality: quality === null ? null : Math.round(quality) })}
        />
      </OptionField>
      <OptionField label={t("options.dpi")} help={t("options.dpi.document.help")}>
        <Select
          aria-label={t("options.dpi")}
          value={String(settings.maxDpi)}
          onChange={(key) => onChange({ maxDpi: Number(key) })}
        >
          {MAX_DPIS.map((dpi) => (
            <SelectItem key={dpi} id={String(dpi)}>
              {dpi === 0 ? t("options.dimension.original") : `${dpi} dpi`}
            </SelectItem>
          ))}
        </Select>
      </OptionField>
    </>
  );
}

export const kompresDokumen = defineTool({
  noun: "document",
  pick: pickDocuments,
  useSettings: useDocumentSettings,
  Main,
  Advanced,
  toRequest,
  fileNote,
});
