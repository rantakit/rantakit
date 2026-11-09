import { useMemo } from "react";
import { Image } from "@keyline-icons/react";
import { ToggleButton } from "@/components/ui/ToggleButton";
import { ToggleButtonGroup } from "@/components/ui/ToggleButtonGroup";
import { pickImages } from "@/features/compress/imageApi";
import { NumberInput } from "@/features/tool/NumberInput";
import { OptionField } from "@/features/tool/OptionField";
import { defineTool, type OptionsProps } from "@/features/tool/toolDef";
import { useT } from "@/i18n";
import { leavesUnchanged, toConvertOptions, useConvertSettings, type Background, type ConvertSettings } from "./convertSettings";

/** What the Tool makes, said once above the button: "Setiap foto disimpan sebagai JPG." */
export function ConvertNote({ to }: { to: "jpg" | "png" | "any" }) {
  const { t } = useT();
  return (
    <div className="flex items-start gap-3 rounded-2xl border border-border bg-surface p-4">
      <span className="flex size-10 shrink-0 items-center justify-center rounded-xl bg-brand-soft text-brand-strong">
        <Image aria-hidden className="size-5" />
      </span>
      <p className="text-small text-text-muted">{t(`convert.note.${to}`)}</p>
    </div>
  );
}

function Swatch({ color }: { color: string }) {
  return (
    <span aria-hidden className="size-4 shrink-0 rounded-full border border-border-strong" style={{ background: color }} />
  );
}

/** Only for JPG: PNG has no quality to choose and keeps its transparency. */
function Advanced({ settings, onChange }: OptionsProps<ConvertSettings>) {
  const { t } = useT();
  return (
    <>
      <OptionField label={t("convert.quality")} help={t("convert.quality.help")}>
        <NumberInput
          label={t("convert.quality")}
          minValue={1}
          maxValue={100}
          value={settings.quality}
          onChange={(quality) => onChange({ quality: quality === null ? 90 : Math.round(quality) })}
        />
      </OptionField>
      <OptionField label={t("convert.background")} help={t("convert.background.help")}>
        <ToggleButtonGroup
          aria-label={t("convert.background")}
          selectionMode="single"
          disallowEmptySelection
          selectedKeys={[settings.background]}
          onSelectionChange={(keys) => onChange({ background: [...keys][0] as Background })}
          className="[&>*]:flex-auto [&>*]:gap-1.5 [&>*]:px-2"
        >
          <ToggleButton id="white">
            <Swatch color="#ffffff" /> {t("convert.background.white")}
          </ToggleButton>
          <ToggleButton id="black">
            <Swatch color="#000000" /> {t("convert.background.black")}
          </ToggleButton>
          <ToggleButton id="custom">
            <Swatch color={settings.customColor} /> {t("convert.background.custom")}
          </ToggleButton>
        </ToggleButtonGroup>
        {settings.background === "custom" && (
          <input
            type="color"
            aria-label={t("convert.background.custom")}
            value={settings.customColor}
            onChange={(event) => onChange({ customColor: event.target.value })}
            className="h-11 w-full cursor-default rounded-xl border border-border bg-surface px-1.5"
          />
        )}
      </OptionField>
    </>
  );
}

/** Ubah ke JPG for photos; the PDF half is PDF ke JPG's page (see `ubahKeJpg`). */
export const photosToJpg = defineTool({
  noun: "photo",
  pick: pickImages,
  useSettings: useConvertSettings,
  Main: () => <ConvertNote to="jpg" />,
  Advanced,
  toRequest: (settings) => toConvertOptions("jpeg", settings),
  leavesUnchanged,
});

/** Nothing to choose: a PNG keeps every pixel and its transparency. */
const NO_SETTINGS: ConvertSettings = { quality: 90, background: "white", customColor: "#f2f2f2" };

export const ubahKePng = defineTool({
  noun: "photo",
  presets: false,
  pick: pickImages,
  useSettings: () => useMemo(() => [NO_SETTINGS, () => {}] as const, []),
  Main: () => <ConvertNote to="png" />,
  toRequest: (settings) => toConvertOptions("png", settings),
  leavesUnchanged,
});
