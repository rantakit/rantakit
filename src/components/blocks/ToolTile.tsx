import type { ComponentType } from "react";
import type { IconProps } from "@keyline-icons/react";
import { Button } from "react-aria-components";
import { focusRing, tv } from "@/lib/react-aria-utils";

const tileButton = tv({
  extend: focusRing,
  base: [
    "group flex flex-1 flex-col items-center gap-2 rounded-2xl px-0.5 pt-1.5 pb-1 text-center",
    "cursor-default select-none [-webkit-tap-highlight-color:transparent]",
  ],
});

const box = tv({
  base: "flex size-14 shrink-0 items-center justify-center rounded-2xl transition-colors",
  variants: {
    look: {
      tool: "bg-brand-soft text-brand-strong group-hover:ring-1 group-hover:ring-brand",
      all: "border border-border-strong bg-surface text-text-muted group-hover:border-brand group-hover:text-brand-strong",
      soon: "border border-dashed border-border-strong bg-badge-bg text-text-disabled",
    },
  },
});

const name = "line-clamp-2 text-small font-semibold";

interface ToolTileProps {
  name: string;
  /** A two-tone icon from `@keyline-icons/react/two-tone`, or an outline one for "Semua". */
  icon: ComponentType<IconProps>;
  /** "Semua", which leads to every Tool, has a plain box. */
  look?: "tool" | "all";
  /**
   * Shown greyed out with this line ("Segera hadir"): not focusable, does
   * nothing, and read as "<name>, segera hadir".
   */
  comingSoon?: string;
  onPress?: () => void;
}

/** A Tool in Home's grid: its icon in a rounded box, and its name under it. */
export function ToolTile({ name: label, icon: Icon, look = "tool", comingSoon, onPress }: ToolTileProps) {
  if (comingSoon) {
    return (
      <div className="flex flex-1 flex-col items-center gap-2 px-0.5 pt-1.5 pb-1 text-center text-text-muted">
        <span className={box({ look: "soon" })}>
          <Icon aria-hidden className="size-7" />
        </span>
        <span className="flex flex-col items-center">
          <span className={name}>
            {label}
            <span className="sr-only">, </span>
          </span>
          <span className="text-[12px] leading-4 font-semibold text-badge-text">{comingSoon}</span>
        </span>
      </div>
    );
  }
  return (
    <Button onPress={onPress} className={(renderProps) => tileButton(renderProps)}>
      <span className={box({ look })}>
        <Icon aria-hidden className="size-7" />
      </span>
      <span className={name}>{label}</span>
    </Button>
  );
}
