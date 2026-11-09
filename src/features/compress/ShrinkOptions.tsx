import { NumberField as AriaNumberField } from "react-aria-components";
import { Button } from "@/components/ui/Button";
import { FieldGroup, Input } from "@/components/ui/Field";
import { RadioCards } from "@/components/ui/RadioCards";
import { Select, SelectItem } from "@/components/ui/Select";
import { ToggleButton } from "@/components/ui/ToggleButton";
import { ToggleButtonGroup } from "@/components/ui/ToggleButtonGroup";
import { OptionField } from "@/features/tool/OptionField";
import { useT } from "@/i18n";
import { formatBytes } from "@/lib/format";
import { asTyped, useRecentLimits } from "./recentLimits";
import { LEVELS, type ShrinkSettings } from "./shrink";

interface ShrinkOptionsProps {
  noun: "photo" | "pdf" | "document";
  settings: ShrinkSettings;
  onChange: (patch: Partial<ShrinkSettings>) => void;
}

/** The top of a compress Tool's Options: "Sesuaikan batas ukuran" or "Kecilkan saja". */
export function ShrinkOptions({ noun, settings, onChange }: ShrinkOptionsProps) {
  const { t, locale } = useT();
  const recent = useRecentLimits();
  return (
    <>
      <ToggleButtonGroup
        aria-label={t("shrink.label")}
        selectionMode="single"
        disallowEmptySelection
        selectedKeys={[settings.mode]}
        onSelectionChange={(keys) => onChange({ mode: [...keys][0] as ShrinkSettings["mode"] })}
        // A narrow panel, such as a phone turned sideways, wraps the labels instead of cutting them off.
        className="[&>*]:h-auto [&>*]:min-h-9 [&>*]:flex-1 [&>*]:py-1.5 [&>*]:whitespace-normal"
      >
        <ToggleButton id="limit">{t("shrink.limit")}</ToggleButton>
        <ToggleButton id="smaller">{t("shrink.smaller")}</ToggleButton>
      </ToggleButtonGroup>

      {settings.mode === "limit" && (
        <OptionField
          label={t("options.limit")}
          description={t(settings.limit === null ? `options.limit.empty.${noun}` : `options.limit.each.${noun}`)}
        >
          <div className="flex gap-2">
            <AriaNumberField
              aria-label={t("options.limit")}
              className="min-w-0 flex-1"
              minValue={0.1}
              value={settings.limit ?? NaN}
              formatOptions={{ maximumFractionDigits: 1 }}
              onChange={(value) => onChange({ limit: Number.isNaN(value) || value <= 0 ? null : value })}
            >
              <FieldGroup className="h-11">
                <Input placeholder={t("options.limit.placeholder")} className="min-h-11 w-full" />
              </FieldGroup>
            </AriaNumberField>
            <Select
              aria-label={t("options.limit.unit")}
              className="w-24 shrink-0 [&>button]:h-11"
              value={settings.unit}
              onChange={(key) => onChange({ unit: key as ShrinkSettings["unit"] })}
            >
              <SelectItem id="KB">KB</SelectItem>
              <SelectItem id="MB">MB</SelectItem>
            </Select>
          </div>
          {recent.length > 0 && (
            <div role="group" aria-label={t("options.limit.recent")} className="flex flex-wrap items-center gap-1.5">
              <span aria-hidden className="me-0.5 text-small text-text-muted">
                {t("options.limit.recent")}
              </span>
              {recent.map((bytes) => (
                <Button
                  key={bytes}
                  variant="secondary"
                  size="row"
                  className="min-h-8 rounded-full px-3 text-small"
                  onPress={() => onChange(asTyped(bytes))}
                >
                  {formatBytes(bytes, locale)}
                </Button>
              ))}
            </div>
          )}
        </OptionField>
      )}

      {settings.mode === "smaller" && (
        <RadioCards
          label={t("level.label")}
          value={settings.level}
          onChange={(level) => onChange({ level })}
          options={LEVELS.map((level) => ({
            id: level,
            label: t(`level.${level}`),
            description: t(`level.${level}.description.${noun}`),
          }))}
        />
      )}
    </>
  );
}
