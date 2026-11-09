import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { TextField } from "react-aria-components";
import { FieldGroup, Input } from "@/components/ui/Field";
import { Switch } from "@/components/ui/Switch";
import { ToggleButton } from "@/components/ui/ToggleButton";
import { ToggleButtonGroup } from "@/components/ui/ToggleButtonGroup";
import { pickImages } from "@/features/compress/imageApi";
import { pickPdfs } from "@/features/compress/pdfApi";
import type { TaskResult, ToolOptions, WatermarkRequest } from "@/features/tool/taskApi";
import { pickAnyFiles } from "@/features/drop/pickFiles";
import { nounOf, type Noun } from "@/features/tool/fileInfo";
import { OptionField } from "@/features/tool/OptionField";
import { defineTool, type OptionsProps } from "@/features/tool/toolDef";
import { currentTranslator, useT, type Translator } from "@/i18n";
import { useStoredState } from "@/lib/storedState";

type Placement = WatermarkRequest["placement"];
type MarkSize = WatermarkRequest["size"];
type Opacity = WatermarkRequest["opacity"];
type Color = WatermarkRequest["color"];

export interface WatermarkSettings {
  /** The watermark's first line; the last one written is remembered. */
  text: string;
  /** "Tambahkan tanggal hari ini": today's date on a second line. */
  addDate: boolean;
  placement: Placement;
  size: MarkSize;
  opacity: Opacity;
  color: Color;
}

const PLACEMENTS: Placement[] = ["repeated", "center", "corner"];
const SIZES: MarkSize[] = ["small", "medium", "large"];
const OPACITIES: Opacity[] = ["faint", "medium", "strong"];
const COLORS: Color[] = ["gray", "black", "white", "red"];

/** What each color looks like on its chip. */
const SWATCHES: Record<Color, string> = { gray: "#808080", black: "#000000", white: "#ffffff", red: "#d32f2f" };

const pickFrom = <T,>(choices: T[], stored: unknown, fallback: T) => choices.find((c) => c === stored) ?? fallback;

function parse(stored: unknown): WatermarkSettings {
  const s = (stored ?? {}) as Partial<Record<keyof WatermarkSettings, unknown>>;
  return {
    // The first visit starts with an example to finish: "Hanya untuk verifikasi …".
    text: typeof s.text === "string" ? s.text.slice(0, 200) : currentTranslator().t("watermark.example"),
    addDate: s.addDate === true,
    placement: pickFrom(PLACEMENTS, s.placement, "repeated"),
    size: pickFrom(SIZES, s.size, "medium"),
    opacity: pickFrom(OPACITIES, s.opacity, "medium"),
    color: pickFrom(COLORS, s.color, "gray"),
  };
}

function useWatermarkSettings() {
  return useStoredState("rantakit.beriWatermark", parse);
}

/** Today as on Indonesian forms: "08/11/2026". */
function today() {
  const now = new Date();
  const two = (n: number) => String(n).padStart(2, "0");
  return `${two(now.getDate())}/${two(now.getMonth() + 1)}/${now.getFullYear()}`;
}

function toWatermark(settings: WatermarkSettings): WatermarkRequest {
  return {
    text: settings.text.trim(),
    date: settings.addDate ? today() : undefined,
    placement: settings.placement,
    size: settings.size,
    opacity: settings.opacity,
    color: settings.color,
  };
}

function toRequest(settings: WatermarkSettings): ToolOptions {
  return { tool: "beri-watermark", ...toWatermark(settings) };
}

/** Wide enough for the Options column on a sharp screen. */
const PREVIEW_SIZE = 640;

/**
 * The first file with the watermark on it, the photo or a PDF's first page,
 * made again a moment after each change. The last picture stays up while the
 * next one is made, so typing doesn't flicker.
 */
function Preview({ path, watermark }: { path: string; watermark: WatermarkRequest }) {
  const { t } = useT();
  const [shown, setShown] = useState<{ path: string; url: string } | null>(null);
  const key = JSON.stringify(watermark);
  // The picture on screen, let go of once another takes its place or the page closes.
  const url = useRef<string | null>(null);

  useEffect(() => {
    let current = true;
    const timer = setTimeout(() => {
      invoke<ArrayBuffer>("watermark_preview", {
        path,
        pdf: nounOf(path) === "pdf",
        watermark: JSON.parse(key) as WatermarkRequest,
        size: PREVIEW_SIZE,
      })
        .then((bytes) => {
          if (!current || !bytes || bytes.byteLength === 0) return;
          if (url.current) URL.revokeObjectURL(url.current);
          url.current = URL.createObjectURL(new Blob([bytes], { type: "image/jpeg" }));
          setShown({ path, url: url.current });
        })
        .catch(() => {});
    }, 150);
    return () => {
      current = false;
      clearTimeout(timer);
    };
  }, [path, key]);

  useEffect(() => () => void (url.current && URL.revokeObjectURL(url.current)), []);

  return (
    <figure className="flex flex-col gap-2">
      <figcaption className="text-label font-semibold">{t("watermark.preview")}</figcaption>
      <div className="flex h-56 items-center justify-center overflow-hidden rounded-2xl border border-border bg-badge-bg p-2">
        {shown?.path === path ? (
          <img src={shown.url} alt="" className="max-h-full max-w-full rounded-md object-contain shadow-xs" />
        ) : (
          <span aria-hidden className="h-full w-2/3 animate-pulse rounded-md bg-surface" />
        )}
      </div>
    </figure>
  );
}

function Main({ settings, onChange, paths }: OptionsProps<WatermarkSettings>) {
  const { t } = useT();
  return (
    <>
      {paths.length > 0 && <Preview path={paths[0]} watermark={toWatermark(settings)} />}
      <TextField
        value={settings.text}
        onChange={(text) => onChange({ text })}
        maxLength={200}
        className="flex flex-col gap-2"
      >
        <span className="text-label font-semibold">{t("watermark.text")}</span>
        <FieldGroup className="h-11">
          <Input className="min-h-11 w-full" />
        </FieldGroup>
      </TextField>
      <Switch isSelected={settings.addDate} onChange={(addDate) => onChange({ addDate })}>
        {t("watermark.date")}
      </Switch>
    </>
  );
}

interface ChipsProps<T extends string> {
  label: string;
  value: T;
  choices: T[];
  name: (choice: T) => string;
  swatch?: (choice: T) => string;
  onChange: (choice: T) => void;
}

/** One choice among a few short ones, side by side. */
function Chips<T extends string>({ label, value, choices, name, swatch, onChange }: ChipsProps<T>) {
  return (
    <OptionField label={label}>
      <ToggleButtonGroup
        aria-label={label}
        selectionMode="single"
        disallowEmptySelection
        selectedKeys={[value]}
        onSelectionChange={(keys) => onChange([...keys][0] as T)}
        className="flex-wrap [&>*]:flex-auto [&>*]:gap-1.5 [&>*]:px-2"
      >
        {choices.map((choice) => (
          <ToggleButton key={choice} id={choice}>
            {swatch && (
              <span
                aria-hidden
                className="size-4 shrink-0 rounded-full border border-border-strong"
                style={{ background: swatch(choice) }}
              />
            )}
            {name(choice)}
          </ToggleButton>
        ))}
      </ToggleButtonGroup>
    </OptionField>
  );
}

function Advanced({ settings, onChange }: OptionsProps<WatermarkSettings>) {
  const { t } = useT();
  return (
    <>
      <Chips
        label={t("watermark.placement")}
        value={settings.placement}
        choices={PLACEMENTS}
        name={(placement) => t(`watermark.placement.${placement}`)}
        onChange={(placement) => onChange({ placement })}
      />
      <Chips
        label={t("watermark.opacity")}
        value={settings.opacity}
        choices={OPACITIES}
        name={(opacity) => t(`watermark.opacity.${opacity}`)}
        onChange={(opacity) => onChange({ opacity })}
      />
      <Chips
        label={t("watermark.size")}
        value={settings.size}
        choices={SIZES}
        name={(size) => t(`watermark.size.${size}`)}
        onChange={(size) => onChange({ size })}
      />
      <Chips
        label={t("watermark.color")}
        value={settings.color}
        choices={COLORS}
        name={(color) => t(`watermark.color.${color}`)}
        swatch={(color) => SWATCHES[color]}
        onChange={(color) => onChange({ color })}
      />
    </>
  );
}

/** Photos or PDFs, whichever the page already holds; either while it is empty. */
function pick(current: Noun) {
  return current === "photo" ? pickImages() : current === "pdf" ? pickPdfs() : pickAnyFiles();
}

/** "Watermark ditambahkan ke 3 foto." */
function summary(results: TaskResult[], { t, count }: Translator) {
  const noun = results.length > 0 ? nounOf(results[0].outputPath) : "photo";
  return t("watermark.result", { files: count(`files.${noun}`, results.length) });
}

export const beriWatermark = defineTool({
  noun: "file",
  pick,
  useSettings: useWatermarkSettings,
  Main,
  Advanced,
  toRequest,
  blocked: (settings, _files, _count, { t }) => (settings.text.trim() === "" ? t("watermark.needText") : null),
  summary,
});
