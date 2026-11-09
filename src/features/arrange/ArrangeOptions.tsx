import { useEffect, useRef, useState, type ComponentType, type KeyboardEvent as ReactKeyboardEvent } from "react";
import { Bin, Check, GripVertical, RotateCcw, RotateCw, Undo, X, type IconProps } from "@keyline-icons/react";
import { Checkbox } from "react-aria-components";
import { GridList, GridListItem } from "react-aria-components/GridList";
import { DropIndicator, useDragAndDrop } from "react-aria-components/useDragAndDrop";
import { Button } from "@/components/ui/Button";
import { fileName } from "@/features/compress/imageApi";
import { pickPdfs } from "@/features/compress/pdfApi";
import type { TaskResult } from "@/features/tool/taskApi";
import { PageThumbnail } from "@/features/pages/PageThumbnail";
import { moveBy, reorder } from "@/features/tool/order";
import { defineTool, type WorkspaceProps } from "@/features/tool/toolDef";
import { useT, type Translator } from "@/i18n";
import { useUndoTarget } from "@/lib/undoTarget";
import { isChanged, originalPages, remove, rotate, type PlacedPage } from "./arrangement";

/** Atur Halaman PDF has no options; its work is the page grid. */
type ArrangeSettings = Record<string, never>;

const NO_SETTINGS: ArrangeSettings = {};
const useArrangeSettings = () => [NO_SETTINGS, () => {}] as const;

/**
 * The page grid (M3): every page with a checkbox, a toolbar for the checked
 * ones, dragging to reorder, and "Urungkan" one change at a time. On desktop
 * Ctrl+Z undoes, Delete deletes, and Alt with an arrow key moves a page.
 */
function Workspace({ path, pages: count, heading, working, onStart, onUnsavedChange }: WorkspaceProps) {
  const { t, count: plural } = useT();
  /** The pages as edited; null until the first change. */
  const [edits, setEdits] = useState<PlacedPage[] | null>(null);
  const pages = edits ?? (count === null ? [] : originalPages(count));
  /** Every earlier arrangement, the last one first to come back. */
  const [history, setHistory] = useState<PlacedPage[][]>([]);
  const [selected, setSelected] = useState<Set<string>>(new Set());
  // Each change is said aloud; the count makes a repeated message new.
  const [said, setSaid] = useState({ text: "", count: 0 });
  const say = (text: string) => setSaid((last) => ({ text, count: last.count + 1 }));

  const unsaved = count !== null && isChanged(pages, count);
  const running = working !== null;
  useEffect(() => onUnsavedChange(unsaved), [unsaved, onUnsavedChange]);
  useEffect(() => () => onUnsavedChange(false), [onUnsavedChange]);

  const change = (next: PlacedPage[], message: string) => {
    setHistory((earlier) => [...earlier, pages]);
    setEdits(next);
    say(message);
  };

  const undo = () => {
    const last = history[history.length - 1];
    if (!last || running) return;
    setHistory(history.slice(0, -1));
    setEdits(last);
    setSelected((keys) => new Set(last.filter((p) => keys.has(p.key)).map((p) => p.key)));
    say(t("arrange.undone"));
  };
  // Undo in the title bar menu.
  useUndoTarget(history.length > 0 && !running, undo);

  const turn = (by: 1 | -1) => {
    if (selected.size === 0) return;
    change(rotate(pages, selected, by), plural("arrange.rotated", selected.size));
  };

  const removeSelected = () => {
    if (selected.size === 0) return;
    if (selected.size >= pages.length) return say(t("arrange.keepOne"));
    change(remove(pages, selected), plural("arrange.deleted", selected.size));
    setSelected(new Set());
  };

  const keys = pages.map((p) => p.key);
  const byKey = new Map(pages.map((p) => [p.key, p]));
  const placeAs = (order: string[]) => order.map((key) => byKey.get(key)!);

  const move = (key: string, by: -1 | 1) => {
    const from = keys.indexOf(key);
    const next = moveBy(keys, from, by);
    if (next === keys) return;
    change(placeAs(next), t("files.moved", { position: from + by + 1 }));
  };

  const { dragAndDropHooks } = useDragAndDrop({
    getItems: (dragged) => [...dragged].map((key) => ({ "text/plain": String(key) })),
    onReorder: (event) => {
      const position = event.target.dropPosition === "after" ? "after" : "before";
      const next = reorder(keys, new Set([...event.keys].map(String)), String(event.target.key), position);
      if (next !== keys) change(placeAs(next), t("files.moved", { position: next.indexOf(String([...event.keys][0])) + 1 }));
    },
    // A line between pages; it takes no room, so the pages never jump while dragging.
    renderDropIndicator: (target) => (
      <DropIndicator
        target={target}
        className={({ isDropTarget }) =>
          `w-0 self-stretch rounded-full ${isDropTarget ? "outline-2 outline-solid outline-brand-strong" : ""}`
        }
      />
    ),
  });

  // Ctrl+Z and Delete work wherever focus is on this page.
  const keyActions = useRef({ undo, removeSelected });
  keyActions.current = { undo, removeSelected };
  useEffect(() => {
    if (running) return;
    const onKeyDown = (event: KeyboardEvent) => {
      const target = event.target as HTMLElement | null;
      if (target?.closest("input[type=text], textarea, [role=dialog]")) return;
      if ((event.ctrlKey || event.metaKey) && !event.shiftKey && event.key.toLowerCase() === "z") {
        event.preventDefault();
        keyActions.current.undo();
      } else if (event.key === "Delete") {
        event.preventDefault();
        keyActions.current.removeSelected();
      }
    };
    // Capture: the grid stops the keys it handles before they bubble.
    window.addEventListener("keydown", onKeyDown, true);
    return () => window.removeEventListener("keydown", onKeyDown, true);
  }, [running]);

  /** Alt with an arrow key moves the focused page one place. */
  const onGridKeyDown = (event: ReactKeyboardEvent) => {
    if (!event.altKey) return;
    const by = event.key === "ArrowLeft" || event.key === "ArrowUp" ? -1 : event.key === "ArrowRight" || event.key === "ArrowDown" ? 1 : 0;
    const key = (event.target as HTMLElement).closest("[data-key]")?.getAttribute("data-key");
    if (by === 0 || !key) return;
    event.preventDefault();
    event.stopPropagation();
    move(key, by);
  };

  const name = fileName(path);
  const actions: { id: string; label: string; icon: ComponentType<IconProps>; onPress: () => void }[] = [
    { id: "left", label: t("arrange.rotateLeft"), icon: RotateCcw, onPress: () => turn(-1) },
    { id: "right", label: t("arrange.rotateRight"), icon: RotateCw, onPress: () => turn(1) },
    { id: "delete", label: t("arrange.delete"), icon: Bin, onPress: removeSelected },
  ];
  const hasSelection = selected.size > 0;

  return (
    <div className="flex flex-1 flex-col md:col-span-2 md:row-span-2 md:min-h-0">
      <header className="flex h-14 shrink-0 items-center gap-2 px-2 md:px-4">
        <div className="flex min-w-0 flex-1 items-center gap-2">{heading}</div>
        <Button variant="secondary" size="row" isDisabled={history.length === 0 || running} onPress={undo}>
          <Undo aria-hidden className="size-[18px]" /> {t("arrange.undo")}
        </Button>
      </header>

      <div className="flex min-h-12 flex-wrap items-center gap-2 px-4 pb-2 md:px-5">
        <span className="me-auto min-w-0 truncate text-label font-semibold" title={path}>
          {hasSelection ? plural("arrange.selected", selected.size) : `${name} · ${plural("pages", pages.length)}`}
        </span>
        {hasSelection && !running && (
          <div className="hidden min-w-0 flex-wrap items-center gap-2 md:flex">
            {actions.map(({ id, label, icon: Icon, onPress }) => (
              <Button key={id} variant="secondary" size="row" onPress={onPress}>
                <Icon aria-hidden className="size-[18px]" /> {label}
              </Button>
            ))}
            <Button variant="link" onPress={() => setSelected(new Set())}>
              {t("arrange.clear")}
            </Button>
          </div>
        )}
      </div>
      <p className="px-4 pb-2 text-small text-text-muted md:px-5">{t("arrange.hint")}</p>

      <div
        inert={running}
        onKeyDownCapture={onGridKeyDown}
        className={`px-2.5 pb-4 md:min-h-0 md:flex-1 md:overflow-y-auto md:px-3.5 ${running ? "opacity-45" : ""}`}>
        <GridList
          aria-label={t("arrange.grid", { name })}
          layout="grid"
          items={pages}
          selectionMode="multiple"
          selectionBehavior="toggle"
          selectedKeys={selected}
          onSelectionChange={(next) => setSelected(next === "all" ? new Set(keys) : new Set([...next].map(String)))}
          dragAndDropHooks={dragAndDropHooks}
          className="flex flex-wrap outline-none"
        >
          {(placed) => (
            <PageItem placed={placed} path={path} count={count ?? pages.length} />
          )}
        </GridList>
      </div>

      <div role="status" className="sr-only" key={said.count}>
        {said.text}
      </div>

      {/* The main button; on a phone the toolbar takes its place while pages are checked. */}
      <div className="sticky bottom-0 z-20 flex flex-col gap-2 border-t border-border bg-panel p-4 md:static md:items-end md:px-5">
        {working ? (
          <div className="flex w-full flex-col gap-2 md:w-80">{working}</div>
        ) : (
          <>
            {hasSelection && (
              <div className="grid grid-cols-4 gap-1 md:hidden">
                {actions.map(({ id, label, icon: Icon, onPress }) => (
                  <ToolbarButton key={id} label={label} icon={Icon} onPress={onPress} />
                ))}
                <ToolbarButton label={t("arrange.clear")} icon={X} onPress={() => setSelected(new Set())} />
              </div>
            )}
            <div className={`flex w-full flex-col gap-2 md:w-80 ${hasSelection ? "max-md:hidden" : ""}`}>
              <Button
                isDisabled={!unsaved}
                onPress={() => onStart({ tool: "atur-halaman-pdf", pages: pages.map(({ page, turns }) => ({ page, turns })) })}
              >
                {t("main.atur-halaman-pdf")}
              </Button>
              {!unsaved && <p className="text-center text-small text-text-muted">{t("arrange.noChange")}</p>}
            </div>
          </>
        )}
      </div>
    </div>
  );
}

/** A phone toolbar button: its icon over a short label. */
function ToolbarButton({ label, icon: Icon, onPress }: { label: string; icon: ComponentType<IconProps>; onPress: () => void }) {
  return (
    <Button variant="icon" onPress={onPress} className="h-auto min-h-14 w-full flex-col gap-1 rounded-xl px-1 py-2">
      <Icon aria-hidden className="size-5" />
      <span className="text-small leading-tight">{label}</span>
    </Button>
  );
}

interface PageItemProps {
  placed: PlacedPage;
  path: string;
  /** Pages of the Original. */
  count: number;
}

/** One page, named by its number in the Original wherever it is moved, so a new order shows. */
function PageItem({ placed, path, count }: PageItemProps) {
  const { t } = useT();
  return (
    <GridListItem
      id={placed.key}
      textValue={t("page.label", { page: placed.page + 1, pages: count })}
      className={({ isDragging }) =>
        `w-1/2 p-1.5 outline-none [@media(orientation:landscape)_and_(max-height:32rem)]:w-1/3! md:w-44 ${
          isDragging ? "opacity-50" : ""
        }`
      }
    >
      {({ isSelected, isFocusVisible }) => (
        <div
          className={`relative flex flex-col items-center gap-1.5 rounded-xl p-2 transition-colors ${
            isSelected ? "bg-brand-soft" : "hover:bg-badge-bg"
          } ${isFocusVisible ? "outline-3 outline-solid outline-brand-strong" : ""}`}
        >
          <Checkbox
            slot="selection"
            aria-label={t("arrange.select", { page: placed.page + 1 })}
            className="absolute start-1 top-1 z-10 flex size-11 items-center justify-center outline-none"
          >
            {({ isSelected: checked, isFocusVisible: focused }) => (
              <span
                aria-hidden
                className={`flex size-6 items-center justify-center rounded-md border-2 shadow-sm ${
                  checked ? "border-brand-strong bg-brand-strong text-on-primary" : "border-control bg-surface"
                } ${focused ? "outline-3 outline-offset-2 outline-solid outline-brand-strong" : ""}`}
              >
                {checked && <Check className="size-4" />}
              </span>
            )}
          </Checkbox>
          <Button
            slot="drag"
            variant="icon"
            aria-label={t("files.drag", { name: t("page.label", { page: placed.page + 1, pages: count }) })}
            className="absolute end-1 bottom-0 z-10 cursor-grab touch-none text-text-muted"
          >
            <GripVertical aria-hidden className="size-5" />
          </Button>
          <PageThumbnail
            path={path}
            page={placed.page}
            pages={count}
            size={320}
            rotation={placed.turns * 90}
            selected={isSelected}
            className="w-full"
          />
          <span aria-hidden className="text-small text-text-muted">
            {placed.page + 1}
          </span>
        </div>
      )}
    </GridListItem>
  );
}

function summary(results: TaskResult[], { t, count }: Translator) {
  return t("arrange.result", { pages: count("pages", results[0]?.pages ?? 0) });
}

export const aturHalamanPdf = defineTool({
  noun: "pdf",
  presets: false,
  pick: pickPdfs,
  useSettings: useArrangeSettings,
  Main: () => null,
  toRequest: () => ({ tool: "atur-halaman-pdf", pages: [] }),
  Workspace,
  summary,
});
