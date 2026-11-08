import { Check } from "@keyline-icons/react";
import { ToggleButton } from "react-aria-components";
import { fileName } from "@/features/compress/imageApi";
import { formatRanges, parseRanges, togglePage } from "@/features/split/pageRanges";
import { useT } from "@/i18n";
import { focusRing, tv } from "@/lib/react-aria-utils";
import { PageThumbnail } from "./PageThumbnail";

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
export function PageStrip({ path, pages, ranges, onChange }: PageStripProps) {
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
