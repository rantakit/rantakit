import type { ComponentType } from "react";
import type { IconProps } from "@keyline-icons/react";
import { Button } from "react-aria-components";
import { focusRing, tv } from "@/lib/react-aria-utils";

const card = tv({
  extend: focusRing,
  base: [
    "flex flex-1 border border-border bg-surface text-start",
    "cursor-default select-none transition-colors hover:border-brand",
    "[-webkit-tap-highlight-color:transparent]",
  ],
  variants: {
    size: {
      normal: "min-h-25 items-start gap-3.5 rounded-2xl p-4",
      large: "flex-col items-start gap-4 rounded-[20px] p-5",
    },
  },
});

const tile = tv({
  base: "flex shrink-0 items-center justify-center",
  variants: {
    size: {
      normal: "size-12 rounded-xl",
      large: "size-14 rounded-[14px]",
    },
    soon: {
      false: "bg-brand-soft text-brand-strong",
      true: "bg-badge-bg text-text-disabled",
    },
  },
});

interface ToolCardProps {
  name: string;
  /** One plain line under the name. */
  description?: string;
  /** A two-tone icon from `@keyline-icons/react/two-tone`. */
  icon: ComponentType<IconProps>;
  size?: "normal" | "large";
  /**
   * Shown greyed out with this badge ("Segera hadir"): not focusable, does
   * nothing when clicked, and read as "<name>, segera hadir".
   */
  comingSoon?: string;
  onPress?: () => void;
}

/** A Tool on the All tools page ("Peralatan"): icon, name, and description. */
export function ToolCard({ name, description, icon: Icon, size = "normal", comingSoon, onPress }: ToolCardProps) {
  if (comingSoon) {
    return (
      <div className="relative flex min-h-25 flex-1 items-start gap-3.5 rounded-2xl border border-dashed border-border p-4 text-text-muted">
        <span className={tile({ size, soon: true })}>
          <Icon aria-hidden className="size-6" />
        </span>
        <span className="flex min-w-0 flex-col gap-0.5">
          <span className="font-semibold">
            {name}
            <span className="sr-only">, </span>
          </span>
          <span className="absolute -top-2.5 end-3.5 rounded-full bg-badge-bg px-2 py-0.5 text-small font-semibold text-badge-text">
            {comingSoon}
          </span>
          {description && <span className="line-clamp-2 text-small text-text-disabled">{description}</span>}
        </span>
      </div>
    );
  }

  return (
    <Button onPress={onPress} className={(renderProps) => card({ ...renderProps, size })}>
      <span className={tile({ size, soon: false })}>
        <Icon aria-hidden className={size === "large" ? "size-7" : "size-6"} />
      </span>
      <span className="flex min-w-0 flex-col gap-0.5">
        <span className="font-semibold">{name}</span>
        {description && <span className="line-clamp-2 text-small text-text-muted">{description}</span>}
      </span>
    </Button>
  );
}
