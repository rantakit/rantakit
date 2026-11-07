import { useState } from "react";
import { ArrowLeft, BookmarkPlus, ChevronRight } from "@keyline-icons/react";
import { Button as AriaButton, ProgressBar } from "react-aria-components";
import { Button } from "@/components/ui/Button";
import type { ToolOptions } from "@/features/compress/taskApi";
import type { CompressJob, JobItem } from "@/features/compress/useCompressJob";
import { TOOL_ICONS } from "@/features/home/tools";
import { ManagePresetsDialog, SavePresetDialog } from "@/features/presets/PresetDialogs";
import { PresetPicker } from "@/features/presets/PresetPicker";
import { usePresets } from "@/features/presets/presetsApi";
import { useAppSettings } from "@/features/settings/appSettings";
import { useT } from "@/i18n";
import { isDesktopApp } from "@/lib/platform";
import { focusRing, tv } from "@/lib/react-aria-utils";
import { useStoredState } from "@/lib/storedState";
import { useFileInfo, type FileInfo } from "./fileInfo";
import { FilesColumn } from "./FilesColumn";
import { ResultList, ResultSummary } from "./ResultView";
import { toolDef, type ShippedTool } from "./tools";
import { unchangedNote } from "./unchanged";

interface ToolPageProps {
  tool: ShippedTool;
  paths: string[];
  job: CompressJob;
  onPick: () => void;
  onRemove: (path: string) => void;
  onReorder: (paths: string[]) => void;
  onBack: () => void;
  onStart: (options: ToolOptions) => void;
  onProcessAnother: () => void;
  /** For a Tool with its own work area: whether it has changes that leaving would lose. */
  onUnsavedChange: (unsaved: boolean) => void;
}

/**
 * Every Tool's page: Files on the left, Options on the right with one main
 * button. Render it with `key={tool}`; each Tool keeps its own settings.
 *
 * Desktop fills the content card: the page never scrolls, the file list and
 * the Options scroll on their own, and the main button stays at the panel's
 * foot. A phone stacks everything and pins the main button to the bottom of
 * the screen, where the tab bar is on other pages.
 */
export function ToolPage({
  tool,
  paths,
  job,
  onPick,
  onRemove,
  onReorder,
  onBack,
  onStart,
  onProcessAnother,
  onUnsavedChange,
}: ToolPageProps) {
  const tr = useT();
  const { t, count } = tr;
  const def = toolDef(tool);
  const [settings, updateSettings] = def.useSettings();
  const { presets, save, rename, remove } = usePresets(tool);
  /** The Preset whose options are in use; any change of an option leaves it. */
  const [presetId, setPresetId] = useState<string | null>(null);
  const [presetDialog, setPresetDialog] = useState<"save" | "manage" | null>(null);
  const update = (patch: Partial<typeof settings>) => {
    updateSettings(patch);
    setPresetId(null);
  };
  const appSettings = useAppSettings();
  const infos = useFileInfo(paths);
  const files = paths.flatMap((path) => (infos[path] ? [infos[path]] : []));
  const request = def.toRequest(settings, files);
  const skipNote =
    def.leavesUnchanged || def.fileNote
      ? (info: FileInfo) =>
          def.leavesUnchanged?.(request, info) ? unchangedNote(t, request) : (def.fileNote?.(request, info, tr) ?? null)
      : undefined;
  const blocked = paths.length === 0 ? t(`options.needFiles.${def.noun}`) : (def.blocked?.(settings, files, paths.length, tr) ?? null);
  const Icon = TOOL_ICONS[tool];
  const running = job.isRunning;
  const hasResult = !running && job.items.length > 0;
  // One Result from several files that didn't get made: the list stays, to fix and try again.
  const mergeFailed = def.ordered === true && hasResult && !job.items.some((item) => item.status === "done");

  const heading = (
    <>
      {/* The desktop app goes back from the title bar, which Android doesn't have. */}
      <Button variant="icon" aria-label={t("tool.back")} onPress={onBack} className={isDesktopApp ? "md:hidden" : undefined}>
        <ArrowLeft aria-hidden className="size-5" />
      </Button>
      <span className="flex size-8 shrink-0 items-center justify-center rounded-lg bg-brand-soft text-brand-strong">
        <Icon aria-hidden className="size-5" />
      </span>
      <h1 className="truncate ps-1 font-display text-section">{t(`tool.${tool}`)}</h1>
    </>
  );
  const working = running ? (
    <Working
      label={t(`working.${tool}`, {
        current: Math.min(job.items.filter(isFinished).length + 1, job.items.length),
        files: count(`files.${def.noun}`, job.items.length),
      })}
      fraction={overall(job.items)}
      onCancel={job.cancel}
    />
  ) : null;

  // A Tool with its own work area, once its file is chosen (M3).
  if (def.Workspace && paths.length > 0 && !hasResult) {
    return (
      <div className="flex flex-1 flex-col md:min-h-0">
        <def.Workspace
          key={paths[0]}
          path={paths[0]}
          pages={infos[paths[0]]?.pages ?? null}
          heading={heading}
          working={working}
          onStart={onStart}
          onUnsavedChange={onUnsavedChange}
        />
      </div>
    );
  }

  return (
    <div className="flex flex-1 flex-col md:grid md:min-h-0 md:grid-cols-[minmax(0,1fr)_20rem] md:grid-rows-[minmax(0,1fr)_auto] lg:grid-cols-[minmax(0,1fr)_22.5rem]">
      <section
        aria-label={t(hasResult ? "result.list" : `files.pick.${def.noun}`)}
        className="flex min-w-0 flex-col md:row-span-2 md:min-h-0"
      >
        <header className="flex h-14 shrink-0 items-center gap-2 px-2 md:px-4">{heading}</header>
        {hasResult ? (
          <ResultList job={job} noun={def.noun} comparesSizes={!def.summary} merges={def.ordered === true} />
        ) : (
          <FilesColumn
            noun={def.noun}
            paths={paths}
            items={running ? job.items : undefined}
            skipNote={skipNote}
            onPick={onPick}
            onRemove={onRemove}
            onReorder={def.ordered ? onReorder : undefined}
          />
        )}
      </section>

      <aside
        aria-label={t(hasResult ? "result.label" : "options.label")}
        className="flex flex-col border-t border-border bg-panel md:min-h-0 md:border-t-0 md:border-s"
      >
        {hasResult ? (
          <div className="flex flex-col p-5 md:overflow-y-auto">
            <ResultSummary
              job={job}
              noun={def.noun}
              summary={def.summary}
              merges={def.ordered === true}
              resultsFolder={appSettings?.resultsFolder ?? null}
            />
          </div>
        ) : (
          <div
            inert={running}
            className={`flex flex-col gap-[22px] p-5 transition-opacity md:overflow-y-auto ${running ? "opacity-45" : ""}`}
          >
            {def.presets !== false && (
              <PresetPicker
                presets={presets}
                selected={presetId}
                onSelect={(preset) => {
                  if (preset) updateSettings(preset.options);
                  setPresetId(preset?.id ?? null);
                }}
                onManage={() => setPresetDialog("manage")}
              />
            )}
            <def.Main settings={settings} onChange={update} files={files} paths={paths} />
            {def.Advanced && (def.showAdvanced?.(settings, files) ?? true) && (
              <MoreOptions tool={tool}>
                <def.Advanced settings={settings} onChange={update} files={files} paths={paths} />
              </MoreOptions>
            )}
          </div>
        )}
      </aside>

      {/* The main button: the panel's foot on desktop, pinned to the bottom of the screen on mobile. */}
      <div className="sticky bottom-0 flex flex-col gap-2 border-t border-border bg-panel p-5 md:static md:col-start-2 md:border-s">
        {mergeFailed ? (
          <Button onPress={job.reset}>{t("result.backToFiles")}</Button>
        ) : hasResult ? (
          <Button onPress={onProcessAnother}>{t("result.another")}</Button>
        ) : working ? (
          working
        ) : (
          <>
            {def.presets !== false && (
              <Button variant="link" className="self-center" onPress={() => setPresetDialog("save")}>
                <BookmarkPlus aria-hidden className="size-5" /> {t("preset.save")}
              </Button>
            )}
            <Button isDisabled={blocked !== null} onPress={() => onStart(request)}>
              {t(`main.${tool}`)}
            </Button>
            {blocked && <p className="text-center text-small text-text-muted">{blocked}</p>}
          </>
        )}
      </div>

      {presetDialog === "save" && (
        <SavePresetDialog
          onClose={() => setPresetDialog(null)}
          onSave={async (name) => {
            setPresetDialog(null);
            const preset = await save(name, settings).catch(() => null);
            if (preset) setPresetId(preset.id);
          }}
        />
      )}
      {presetDialog === "manage" && (
        <ManagePresetsDialog
          presets={presets}
          onRename={(id, name) => rename(id, name).catch(() => {})}
          onRemove={(id) => {
            if (id === presetId) setPresetId(null);
            remove(id).catch(() => {});
          }}
          onClose={() => setPresetDialog(null)}
        />
      )}
    </div>
  );
}

const toggle = tv({
  extend: focusRing,
  base: "flex items-center gap-1.5 self-start rounded-lg text-label font-semibold text-brand-strong cursor-default",
});

const parseOpen = (stored: unknown) =>
  Object.fromEntries(
    Object.entries(stored && typeof stored === "object" ? stored : {}).filter(([, open]) => open === true),
  ) as Record<string, boolean>;

/** "Pilihan lanjutan": closed at first, then open or closed as the user left it, per Tool. */
function MoreOptions({ tool, children }: { tool: ShippedTool; children: React.ReactNode }) {
  const { t } = useT();
  const [openTools, update] = useStoredState("rantakit.advancedOptions", parseOpen);
  const isOpen = openTools[tool] === true;
  return (
    <div className="flex flex-col gap-[22px] border-t border-border pt-5">
      <AriaButton
        aria-expanded={isOpen}
        onPress={() => update({ [tool]: !isOpen })}
        className={(renderProps) => toggle(renderProps)}
      >
        {t("options.advanced")}
        <ChevronRight aria-hidden className={`size-4 transition-transform ${isOpen ? "rotate-90" : ""}`} />
      </AriaButton>
      {isOpen && children}
    </div>
  );
}

const isFinished = (item: JobItem) => item.status !== "pending" && item.status !== "working";

/** How far the whole Task is, 0–1. */
function overall(items: JobItem[]) {
  const done = items.reduce(
    (sum, item) => sum + (isFinished(item) ? 1 : item.status === "working" ? item.fraction : 0),
    0,
  );
  return items.length > 0 ? done / items.length : 0;
}

/** Takes the main button's place while a Task runs. */
function Working({ label, fraction, onCancel }: { label: string; fraction: number; onCancel: () => void }) {
  const { t } = useT();
  return (
    <>
      <ProgressBar aria-label={label} value={fraction * 100} className="flex flex-col gap-2">
        {({ percentage }) => (
          <>
            <span aria-hidden className="text-label font-semibold">
              {label}
            </span>
            <span className="h-2 overflow-hidden rounded-full bg-badge-bg">
              <span className="block h-full rounded-full bg-brand-strong transition-[width]" style={{ width: `${percentage}%` }} />
            </span>
          </>
        )}
      </ProgressBar>
      <Button variant="secondary" onPress={onCancel}>
        {t("task.cancel")}
      </Button>
    </>
  );
}
