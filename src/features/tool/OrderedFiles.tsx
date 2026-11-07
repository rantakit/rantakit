import { useState } from "react";
import { GripVertical, MoreVertical } from "@keyline-icons/react";
import { GridList, GridListItem } from "react-aria-components/GridList";
import { DropIndicator, useDragAndDrop } from "react-aria-components/useDragAndDrop";
import { FileTile } from "@/components/blocks/FileRow";
import { Button } from "@/components/ui/Button";
import { Menu, MenuItem, MenuTrigger } from "@/components/ui/Menu";
import { fileName } from "@/features/compress/imageApi";
import { useT } from "@/i18n";
import { formatBytes } from "@/lib/format";
import { useFileTile, type FileInfo } from "./fileInfo";
import { NOUN_ICONS } from "./FilesColumn";
import { moveBy, reorder } from "./order";

interface OrderedFilesProps {
  noun: "photo" | "pdf";
  paths: string[];
  infos: Record<string, FileInfo | undefined>;
  onReorder: (paths: string[]) => void;
  onRemove: (path: string) => void;
}

/**
 * The Files step for Tools that make one Result from several files (X2): each
 * row has a drag handle, and its menu moves it up or down for a keyboard or a
 * screen reader. The order is the order the Result follows.
 */
export function OrderedFiles({ noun, paths, infos, onReorder, onRemove }: OrderedFilesProps) {
  const { t } = useT();
  // Each move is said aloud; the count makes a repeated message new.
  const [moved, setMoved] = useState({ text: "", count: 0 });

  const { dragAndDropHooks } = useDragAndDrop({
    getItems: (keys) => [...keys].map((key) => ({ "text/plain": fileName(String(key)) })),
    onReorder: (event) => {
      if (event.target.dropPosition === "on") return;
      onReorder(reorder(paths, new Set([...event.keys].map(String)), String(event.target.key), event.target.dropPosition));
    },
    renderDropIndicator: (target) => (
      <DropIndicator
        target={target}
        className={({ isDropTarget }) => `mx-3 h-0.5 rounded-full ${isDropTarget ? "bg-brand-strong" : "bg-transparent"}`}
      />
    ),
  });

  const move = (path: string, by: -1 | 1) => {
    const from = paths.indexOf(path);
    const next = moveBy(paths, from, by);
    if (next === paths) return;
    onReorder(next);
    setMoved((last) => ({ text: t("files.moved", { position: from + by + 1 }), count: last.count + 1 }));
  };

  return (
    <>
      <GridList
        aria-label={t("files.list")}
        items={paths.map((path) => ({ id: path }))}
        dragAndDropHooks={dragAndDropHooks}
        className="flex flex-col rounded-2xl border border-border bg-surface"
      >
        {({ id }) => (
          <OrderedRow
            path={id}
            noun={noun}
            info={infos[id]}
            isFirst={id === paths[0]}
            isLast={id === paths[paths.length - 1]}
            onMove={(by) => move(id, by)}
            onRemove={() => onRemove(id)}
          />
        )}
      </GridList>
      <div role="status" className="sr-only" key={moved.count}>
        {moved.text}
      </div>
    </>
  );
}

interface OrderedRowProps {
  path: string;
  noun: "photo" | "pdf";
  info?: FileInfo;
  isFirst: boolean;
  isLast: boolean;
  onMove: (by: -1 | 1) => void;
  onRemove: () => void;
}

function OrderedRow({ path, noun, info, isFirst, isLast, onMove, onRemove }: OrderedRowProps) {
  const { t, count, locale } = useT();
  const tile = useFileTile(path, noun, 96);
  const name = fileName(path);
  const note = [
    info?.pages != null ? count("pages", info.pages) : null,
    info?.size != null ? formatBytes(info.size, locale) : null,
  ]
    .filter(Boolean)
    .join(" · ");

  return (
    <GridListItem
      id={path}
      textValue={name}
      className={({ isDragging, isFocusVisible }) =>
        `flex items-center gap-2 border-b border-border py-3 ps-1 pe-2 last:border-b-0 ${
          isDragging ? "opacity-50" : ""
        } ${isFocusVisible ? "rounded-xl outline-3 outline-solid -outline-offset-3 outline-brand-strong" : "outline-none"}`
      }
    >
      <Button
        slot="drag"
        variant="icon"
        aria-label={t("files.drag", { name })}
        className="cursor-grab touch-none text-text-muted"
      >
        <GripVertical aria-hidden className="size-5" />
      </Button>
      <FileTile icon={NOUN_ICONS[noun]} {...tile} />
      <span className="ms-1.5 flex min-w-0 flex-1 flex-col gap-1">
        <span className="truncate text-label" title={path}>
          {name}
        </span>
        <span className="truncate text-small text-text-muted">{note}</span>
      </span>
      <MenuTrigger placement="bottom end">
        <Button variant="icon" aria-label={t("files.menu", { name })}>
          <MoreVertical aria-hidden className="size-5" />
        </Button>
        <Menu
          disabledKeys={[...(isFirst ? ["up"] : []), ...(isLast ? ["down"] : [])]}
          onAction={(key) => (key === "up" ? onMove(-1) : key === "down" ? onMove(1) : onRemove())}
        >
          <MenuItem id="up">{t("files.moveUp")}</MenuItem>
          <MenuItem id="down">{t("files.moveDown")}</MenuItem>
          <MenuItem id="remove">{t("files.removeFromList")}</MenuItem>
        </Menu>
      </MenuTrigger>
    </GridListItem>
  );
}
