import { RadioCards } from "@/components/ui/RadioCards";
import { pickImages } from "@/features/compress/imageApi";
import { PHOTO_SIZES, type PhotoSize } from "@/features/compress/imageSettings";
import type { ToolOptions } from "@/features/tool/taskApi";
import type { FileInfo } from "@/features/tool/fileInfo";
import { NumberInput } from "@/features/tool/NumberInput";
import { OptionField } from "@/features/tool/OptionField";
import { defineTool, type OptionsProps } from "@/features/tool/toolDef";
import { useT } from "@/i18n";
import { isInt, useStoredState } from "@/lib/storedState";

export interface ResizeSettings {
  /** "Ukuran baru". */
  size: PhotoSize;
  /** "Sisi terpanjang (px)"; wins over `size` when set. */
  maxSide: number | null;
}

const SIZES = Object.keys(PHOTO_SIZES) as PhotoSize[];

function parse(stored: unknown): ResizeSettings {
  const s = (stored ?? {}) as Partial<Record<keyof ResizeSettings, unknown>>;
  return {
    size: SIZES.find((size) => size === s.size) ?? "medium",
    maxSide: isInt(s.maxSide, 1, 20_000) ? s.maxSide : null,
  };
}

function useResizeSettings() {
  return useStoredState("rantakit.ubahUkuranFoto", parse);
}

function toResizeOptions(settings: ResizeSettings): ToolOptions {
  return { tool: "ubah-ukuran-foto", size: settings.size, maxSide: settings.maxSide ?? undefined };
}

/** Photos are never enlarged: one already within the size is skipped. */
function leavesUnchanged(options: ToolOptions, file: FileInfo) {
  if (options.tool !== "ubah-ukuran-foto" || file.longestSide === null) return false;
  return file.longestSide <= (options.maxSide ?? PHOTO_SIZES[options.size ?? "medium"]);
}

function Main({ settings, onChange }: OptionsProps<ResizeSettings>) {
  const { t } = useT();
  return (
    <OptionField label={t("resize.size")}>
      <RadioCards
        label={t("resize.size")}
        value={settings.size}
        options={SIZES.map((size) => ({
          id: size,
          label: t(`photoSize.${size}`),
          description: t(`resize.${size}.description`),
        }))}
        onChange={(size) => onChange({ size })}
      />
    </OptionField>
  );
}

function Advanced({ settings, onChange }: OptionsProps<ResizeSettings>) {
  const { t } = useT();
  return (
    <OptionField label={t("resize.side")} help={t("resize.side.help")} description={t("resize.side.overrides")}>
      <NumberInput
        label={t("resize.side")}
        minValue={1}
        maxValue={20_000}
        placeholder={t("resize.side.placeholder")}
        value={settings.maxSide}
        onChange={(side) => onChange({ maxSide: side === null ? null : Math.round(side) })}
      />
    </OptionField>
  );
}

export const ubahUkuranFoto = defineTool({
  noun: "photo",
  pick: pickImages,
  useSettings: useResizeSettings,
  Main,
  Advanced,
  toRequest: toResizeOptions,
  leavesUnchanged,
});
