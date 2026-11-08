import { Button, Input, Label, SearchField } from "react-aria-components";
import { Search, X } from "@keyline-icons/react";
import { useT } from "@/i18n";
import { focusRing, tv } from "@/lib/react-aria-utils";

const box = tv({
  base: "flex h-14 items-center gap-3 rounded-[14px] bg-surface ps-4 pe-1.5 text-text-muted",
  variants: {
    filled: { false: "border border-border", true: "border-2 border-brand-strong" },
  },
});

const clear = tv({
  extend: focusRing,
  base: "flex size-11 shrink-0 items-center justify-center rounded-[10px] text-text-muted hover:bg-badge-bg hover:text-text",
});

interface SearchBoxProps {
  value: string;
  onChange: (value: string) => void;
}

/** "Cari peralatan" at the top of Home. Escape clears it. */
export function SearchBox({ value, onChange }: SearchBoxProps) {
  const { t } = useT();
  return (
    <div role="search">
      <SearchField value={value} onChange={onChange} className={box({ filled: value !== "" })}>
        <Search aria-hidden className="size-6 shrink-0" />
        <Label className="sr-only">{t("home.search")}</Label>
        <Input
          placeholder={t("home.search.placeholder")}
          className="h-11 min-w-0 flex-1 bg-transparent text-ellipsis text-text outline-none placeholder:text-text-muted [&::-webkit-search-cancel-button]:hidden"
        />
        {value !== "" && (
          <Button aria-label={t("home.search.clear")} className={(renderProps) => clear(renderProps)}>
            <X aria-hidden className="size-5" />
          </Button>
        )}
      </SearchField>
    </div>
  );
}
