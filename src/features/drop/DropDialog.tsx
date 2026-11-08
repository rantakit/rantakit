import { Button as AriaButton, DialogTrigger, Heading } from "react-aria-components";
import { ChevronRight, File, FileText, Image, X } from "@keyline-icons/react";
import { Button } from "@/components/ui/Button";
import { Dialog } from "@/components/ui/Dialog";
import { Modal } from "@/components/ui/Modal";
import { fileName } from "@/features/compress/imageApi";
import { TOOL_ICONS, type ToolId } from "@/features/home/tools";
import { useT } from "@/i18n";
import { focusRing, tv } from "@/lib/react-aria-utils";
import { filesNoun, type DropMatch } from "./dropMatch";

const toolRow = tv({
  extend: focusRing,
  base: [
    "flex w-full items-center gap-3.5 rounded-[14px] border border-border bg-surface py-3 ps-3.5 pe-3 text-start",
    "cursor-default transition-colors hover:border-brand",
  ],
});

interface DropDialogProps {
  paths: string[];
  match: Extract<DropMatch, { status: "fits" }>;
  onChoose: (tool: ToolId) => void;
  onClose: () => void;
}

/** "Mau diapakan 3 foto ini?": the shipped Tools that take every dropped file. */
export function DropDialog({ paths, match, onChoose, onClose }: DropDialogProps) {
  const { t, count } = useT();
  const noun = filesNoun(match);
  const title =
    paths.length === 1
      ? t("drop.ask.single", { noun: t(`noun.${noun}`) })
      : t("drop.ask.many", { files: count(`files.${noun}`, paths.length) });
  const FileIcon = noun === "photo" ? Image : noun === "document" ? FileText : File;

  return (
    <DialogTrigger isOpen onOpenChange={(open) => !open && onClose()}>
      <Modal size="wide" isDismissable>
        <Dialog className="flex flex-col gap-5 pb-5">
          <div className="flex items-start gap-3">
            <Heading slot="title" className="flex-1 pt-1.5 font-display text-dialog">
              {title}
            </Heading>
            <Button variant="icon" aria-label={t("common.close")} onPress={onClose}>
              <X aria-hidden className="size-5" />
            </Button>
          </div>

          <ul aria-label={t("drop.files")} className="flex flex-wrap gap-2">
            {paths.map((path) => (
              <li
                key={path}
                title={path}
                className="flex items-center gap-1.5 rounded-full bg-badge-bg py-1.5 ps-2.5 pe-3 text-small text-badge-text"
              >
                <FileIcon aria-hidden className="size-4" />
                {fileName(path)}
              </li>
            ))}
          </ul>

          <ul aria-label={t("drop.tools")} className="flex flex-col gap-2">
            {match.tools.map((id) => {
              const Icon = TOOL_ICONS[id];
              return (
                <li key={id}>
                  <AriaButton onPress={() => onChoose(id)} className={(renderProps) => toolRow(renderProps)}>
                    <span className="flex size-12 shrink-0 items-center justify-center rounded-xl bg-brand-soft text-brand-strong">
                      <Icon aria-hidden className="size-6" />
                    </span>
                    <span className="flex min-w-0 flex-1 flex-col gap-0.5">
                      <span className="font-semibold">{t(`tool.${id}`)}</span>
                      <span className="text-small text-text-muted">{t(`tool.${id}.description`)}</span>
                    </span>
                    <ChevronRight aria-hidden className="size-5 shrink-0 text-text-muted" />
                  </AriaButton>
                </li>
              );
            })}
          </ul>

          <div className="flex justify-end">
            <Button variant="secondary" onPress={onClose}>
              {t("setup.cancel")}
            </Button>
          </div>
        </Dialog>
      </Modal>
    </DialogTrigger>
  );
}
