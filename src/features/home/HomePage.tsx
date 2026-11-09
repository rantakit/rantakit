import { useEffect, useState, type ReactNode } from "react";
import { ChevronRight, Download, FolderOpen, GridSquares, Inbox, Plus, ScanText, X } from "@keyline-icons/react";
import { revealItemInDir } from "@tauri-apps/plugin-opener";
import { Button as AriaButton, Link } from "react-aria-components";
import { ToolTile } from "@/components/blocks/ToolTile";
import { Button } from "@/components/ui/Button";
import { fileName } from "@/features/compress/imageApi";
import { listHistory, type HistoryItem } from "@/features/history/historyApi";
import { useAppSettings } from "@/features/settings/appSettings";
import { useT, type Translator } from "@/i18n";
import { openPath } from "@/lib/files";
import { canShowInFolder, looksLikeAndroid } from "@/lib/platform";
import { focusRing, tv } from "@/lib/react-aria-utils";
import { SearchBox } from "./SearchBox";
import { searchTools } from "./searchTools";
import { NoToolFound } from "./ToolsPage";
import { TOOL_ICONS, useHomeTools, useTools, type ToolId } from "./tools";

/** How many Tasks "Hasil terbaru" lists; the rest are in Riwayat. */
const RECENT_RESULTS = 10;

interface HomePageProps {
  onOpenTool: (id: ToolId) => void;
  /** The "Semua" tile: the All tools page. */
  onAllTools: () => void;
  /** "Lihat semua" under "Hasil terbaru": Riwayat. */
  onHistory: () => void;
  /** "Pilih file": choose files, then a Tool, as with "Buka". */
  onPickFiles: () => void;
  /** "Versi baru" on Android, which has no title bar; nothing until a newer version is found. */
  notice?: ReactNode;
}

/**
 * Search, a drop card on desktop, the Tools used lately as an icon grid with
 * "Semua" at its end, then the latest Results. On Android a button in the
 * corner takes the drop card's place. Typing a search replaces everything
 * under it with the matching Tools.
 */
export function HomePage({ onOpenTool, onAllTools, onHistory, onPickFiles, notice }: HomePageProps) {
  const { t } = useT();
  const [query, setQuery] = useState("");
  const searching = query.trim() !== "";
  const tools = useTools();
  // Only the Tools this platform lists and has shipped; the backend agrees, so this is a safeguard.
  const shown = useHomeTools().filter((id) => tools.some((tool) => tool.id === id && !tool.comingSoon));
  const matched = searchTools(tools, query);
  const found = [...matched.filter((tool) => !tool.comingSoon), ...matched.filter((tool) => tool.comingSoon)];

  return (
    <div
      className={`mx-auto flex w-full max-w-5xl flex-1 flex-col gap-6 px-4 pt-5 md:gap-8 md:px-12 md:pt-10 md:pb-12 ${
        looksLikeAndroid ? "pb-29" : "pb-10"
      }`}
    >
      <h1 className="sr-only">{t("nav.home")}</h1>
      {notice}
      <SearchBox value={query} onChange={setQuery} />

      {searching ? (
        found.length === 0 ? (
          <NoToolFound />
        ) : (
          <section aria-labelledby="home-found" className="flex flex-col gap-4">
            <h2 id="home-found" className="text-section">
              {t("home.search.results")}
            </h2>
            <TileGrid>
              {found.map(({ id, comingSoon }) => (
                <li key={id} className="flex">
                  <ToolTile
                    name={t(`tool.${id}`)}
                    icon={TOOL_ICONS[id]}
                    comingSoon={comingSoon ? t("home.comingSoon") : undefined}
                    onPress={() => onOpenTool(id)}
                  />
                </li>
              ))}
            </TileGrid>
          </section>
        )
      ) : (
        <>
          {!looksLikeAndroid && <DropCard onPickFiles={onPickFiles} />}
          <section aria-labelledby="home-tools">
            <h2 id="home-tools" className="sr-only">
              {t("home.tools")}
            </h2>
            {shown.length > 0 && (
              <TileGrid>
                {shown.map((id) => (
                  <li key={id} className="flex">
                    <ToolTile name={t(`tool.${id}`)} icon={TOOL_ICONS[id]} onPress={() => onOpenTool(id)} />
                  </li>
                ))}
                <li className="flex">
                  <ToolTile name={t("home.allTools")} icon={GridSquares} look="all" onPress={onAllTools} />
                </li>
              </TileGrid>
            )}
          </section>
          <LatestResults onHistory={onHistory} />
        </>
      )}

      {looksLikeAndroid && <AddButton onPickFiles={onPickFiles} onScan={() => onOpenTool("scan-dokumen")} />}
    </div>
  );
}

/** Four columns on a phone, eight from `md`. */
function TileGrid({ children }: { children: ReactNode }) {
  return <ul className="grid grid-cols-4 gap-x-1 gap-y-3 md:grid-cols-8 md:gap-x-4">{children}</ul>;
}

/** Desktop: "Tarik file ke sini", with "Pilih file" for those who'd rather choose. */
function DropCard({ onPickFiles }: { onPickFiles: () => void }) {
  const { t } = useT();
  return (
    <section
      aria-labelledby="home-drop"
      className="flex items-center gap-5 rounded-[20px] border-2 border-dashed border-border-strong bg-panel px-8 py-10"
    >
      <span className="flex size-14 shrink-0 items-center justify-center rounded-2xl border border-border bg-surface text-brand-strong">
        <Download aria-hidden className="size-7" />
      </span>
      <div className="flex min-w-0 flex-1 flex-col gap-0.5">
        <h2 id="home-drop" className="font-bold">
          {t("home.drop")}
        </h2>
        <p className="text-small text-text-muted">{t("home.drop.kinds")}</p>
      </div>
      <Button onPress={onPickFiles} className="shrink-0 gap-2">
        <FolderOpen aria-hidden className="size-5" />
        {t("home.pick")}
      </Button>
    </section>
  );
}

const fab = tv({
  extend: focusRing,
  base: [
    "fixed end-4 bottom-[calc(73px+env(safe-area-inset-bottom))] z-40 flex size-14 items-center justify-center rounded-full",
    "cursor-default bg-primary text-on-primary shadow-lg hover:bg-primary-hover [-webkit-tap-highlight-color:transparent]",
  ],
});

const dialItem = tv({
  extend: focusRing,
  base: "flex cursor-default items-center gap-3 rounded-full [-webkit-tap-highlight-color:transparent]",
});

/**
 * Android: the round add button above the tab bar. It opens two choices over
 * a scrim, "Pilih file" nearest to it and "Scan Dokumen" above, and turns
 * into a close button meanwhile.
 */
function AddButton({ onPickFiles, onScan }: { onPickFiles: () => void; onScan: () => void }) {
  const { t } = useT();
  const [open, setOpen] = useState(false);
  const choose = (then: () => void) => () => {
    setOpen(false);
    then();
  };
  const choices = [
    { label: t("tool.scan-dokumen"), icon: ScanText, onPress: choose(onScan) },
    { label: t("home.pick"), icon: FolderOpen, onPress: choose(onPickFiles) },
  ];
  return (
    <div onKeyDown={(event) => event.key === "Escape" && setOpen(false)}>
      {open && (
        <>
          {/* The scrim closes the choices; Escape and the button do too. */}
          <div aria-hidden className="fixed inset-0 z-30 bg-backdrop" onClick={() => setOpen(false)} />
          <ul className="fixed end-5 bottom-[calc(145px+env(safe-area-inset-bottom))] z-40 flex flex-col items-end gap-3">
            {choices.map(({ label, icon: Icon, onPress }) => (
              <li key={label}>
                <AriaButton onPress={onPress} className={(renderProps) => dialItem(renderProps)}>
                  <span className="rounded-full border border-border bg-surface px-4 py-2 text-label">{label}</span>
                  <span className="flex size-12 items-center justify-center rounded-full border border-border bg-surface text-brand-strong">
                    <Icon aria-hidden className="size-6" />
                  </span>
                </AriaButton>
              </li>
            ))}
          </ul>
        </>
      )}
      <AriaButton
        aria-label={open ? t("home.add.close") : t("home.add")}
        aria-expanded={open}
        onPress={() => setOpen(!open)}
        className={(renderProps) => fab(renderProps)}
      >
        {open ? <X aria-hidden className="size-6" /> : <Plus aria-hidden className="size-[26px]" />}
      </AriaButton>
    </div>
  );
}

/** "Hasil terbaru": the latest Tasks, one row each; hidden while History is off. */
function LatestResults({ onHistory }: { onHistory: () => void }) {
  const { t } = useT();
  const settings = useAppSettings();
  const [items, setItems] = useState<HistoryItem[] | null>(null);
  const off = settings?.keepHistory === false;

  useEffect(() => {
    if (off) return;
    const load = () =>
      listHistory()
        .then((all) => setItems(all.slice(0, RECENT_RESULTS)))
        .catch(() => setItems([]));
    load();
    // Results may have been moved or deleted while Rantakit was out of sight.
    const onVisible = () => document.visibilityState === "visible" && load();
    document.addEventListener("visibilitychange", onVisible);
    return () => document.removeEventListener("visibilitychange", onVisible);
  }, [off]);

  if (settings === null || off || items === null) return null;
  return (
    <section aria-labelledby="home-results" className="flex flex-col gap-3">
      <div className="flex items-center justify-between gap-3">
        <h2 id="home-results" className="text-section">
          {t("home.results")}
        </h2>
        {items.length > 0 && (
          <Link onPress={onHistory} className={(renderProps) => seeAll(renderProps)}>
            {t("home.results.all")}
            <ChevronRight aria-hidden className="size-[18px]" />
          </Link>
        )}
      </div>
      {items.length === 0 ? (
        <div className="flex items-center gap-3.5 rounded-2xl border border-dashed border-border-strong p-3.5">
          <span className="flex size-11 shrink-0 items-center justify-center rounded-xl bg-badge-bg text-text-muted">
            <Inbox aria-hidden className="size-[22px]" />
          </span>
          <p className="text-text-muted">{t("history.empty.hint")}</p>
        </div>
      ) : (
        <ul className="flex flex-col divide-y divide-border overflow-hidden rounded-2xl border border-border bg-surface">
          {items.map((item) => (
            <ResultRow key={item.id} item={item} />
          ))}
        </ul>
      )}
    </section>
  );
}

const seeAll = tv({
  extend: focusRing,
  base: "flex min-h-11 cursor-default items-center gap-0.5 rounded-[10px] ps-2 pe-1 text-label text-brand-strong hover:bg-brand-soft",
});

const row = tv({
  extend: focusRing,
  base: "flex min-h-16 w-full cursor-default items-center gap-3.5 py-2.5 ps-3.5 pe-3 text-start hover:bg-panel",
  variants: {
    isDisabled: { true: "text-text-muted hover:bg-transparent" },
  },
});

const isToolId = (tool: string): tool is ToolId => tool in TOOL_ICONS;

/**
 * One Task: its first Result's name, then the Tool, how many files, and when.
 * Pressing it opens the Result, or with several, the folder they are in.
 */
function ResultRow({ item }: { item: HistoryItem }) {
  const tr = useT();
  const { t, count } = tr;
  const first = item.files[0];
  // Several Originals merged into one Result, as in Gabung PDF, make one file.
  const merged = item.files.every((file) => file.resultPath === first.resultPath);
  const results = merged ? 1 : item.files.length;
  const existing = item.files.filter((file) => file.exists);
  const missing = existing.length === 0;
  const known = isToolId(item.tool);
  const Icon = known ? TOOL_ICONS[item.tool as ToolId] : Inbox;
  const title =
    results === 1 ? fileName(first.resultPath) : t("history.andMore", { name: fileName(first.resultPath), count: results - 1 });
  const meta = [
    known ? t(`tool.${item.tool as ToolId}`) : item.tool,
    results > 1 && count("files.any", results),
    missing ? t("history.missing") : when(tr, item.createdAt),
  ]
    .filter(Boolean)
    .join(" · ");

  const open = () => {
    if (results === 1) return openPath(existing[0].resultPath);
    // Several Results from one Original sit in a folder of their own (X3).
    const fromOne = item.files.every((file) => file.originalPath === first.originalPath);
    if (!canShowInFolder) return openPath(existing[0].resultPath);
    return fromOne ? openPath(parentOf(existing[0].resultPath)) : revealItemInDir(existing[0].resultPath);
  };

  return (
    <li>
      <AriaButton isDisabled={missing} onPress={open} className={(renderProps) => row(renderProps)}>
        <span
          className={`flex size-11 shrink-0 items-center justify-center rounded-xl ${
            missing ? "bg-badge-bg" : "bg-brand-soft text-brand-strong"
          }`}
        >
          <Icon aria-hidden className="size-[22px]" />
        </span>
        <span className="flex min-w-0 flex-1 flex-col">
          <span className="truncate text-label">{title}</span>
          <span className="truncate text-small text-text-muted">{meta}</span>
        </span>
        {!missing && <ChevronRight aria-hidden className="size-5 shrink-0 text-text-muted" />}
      </AriaButton>
    </li>
  );
}

/** The folder a Result sits in. */
const parentOf = (path: string) => path.slice(0, Math.max(path.lastIndexOf("/"), path.lastIndexOf("\\")));

/** "Hari ini, 10.24", "Kemarin, 16.40", then "7 Nov, 19.15". */
function when({ t, locale }: Translator, time: number) {
  const date = new Date(time);
  const today = new Date();
  const yesterday = new Date();
  yesterday.setDate(today.getDate() - 1);
  const day =
    date.toDateString() === today.toDateString()
      ? t("history.today")
      : date.toDateString() === yesterday.toDateString()
        ? t("history.yesterday")
        : new Intl.DateTimeFormat(locale, {
            day: "numeric",
            month: "short",
            year: date.getFullYear() === today.getFullYear() ? undefined : "numeric",
          }).format(date);
  const clock = new Intl.DateTimeFormat(locale, { hour: "2-digit", minute: "2-digit" }).format(date);
  return t("home.results.when", { day, time: clock });
}
