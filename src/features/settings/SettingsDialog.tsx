import { useEffect, useState, type ComponentType, type ReactNode } from "react";
import { getVersion } from "@tauri-apps/api/app";
import { invoke, isTauri } from "@tauri-apps/api/core";
import {
  ArrowLeft,
  ChevronRight,
  Folder,
  Globe,
  History,
  Info,
  Monitor,
  Moon,
  Palette,
  Sun,
  X,
  type IconProps,
} from "@keyline-icons/react";
import { Button as AriaButton, DialogTrigger, Heading } from "react-aria-components";
import { AlertDialog } from "@/components/ui/AlertDialog";
import { Button } from "@/components/ui/Button";
import { Dialog } from "@/components/ui/Dialog";
import { Link } from "@/components/ui/Link";
import { Modal } from "@/components/ui/Modal";
import { RadioCards } from "@/components/ui/RadioCards";
import { Radio, RadioGroup } from "@/components/ui/RadioGroup";
import { Switch } from "@/components/ui/Switch";
import { Tab, TabList, TabPanel, TabPanels, Tabs } from "@/components/ui/Tabs";
import { ToggleButton } from "@/components/ui/ToggleButton";
import { ToggleButtonGroup } from "@/components/ui/ToggleButtonGroup";
import { useT } from "@/i18n";
import { ISSUES_URL, LICENSE_URL, openExternal, SOURCE_URL } from "@/lib/links";
import { isAndroidApp, isDesktopApp } from "@/lib/platform";
import { focusRing, tv } from "@/lib/react-aria-utils";
import { pickResultsFolder, setCheckUpdates, setKeepHistory, setResultsFolder, useAppSettings } from "./appSettings";
import { systemLanguage, usePreferences, type Language, type LanguageChoice, type TextSize, type Theme } from "./preferences";

export type SettingsTab = "appearance" | "language" | "results" | "history" | "about";

const ALL_TABS: { id: SettingsTab; label: `settings.tab.${SettingsTab}`; icon: ComponentType<IconProps> }[] = [
  { id: "appearance", label: "settings.tab.appearance", icon: Palette },
  { id: "language", label: "settings.tab.language", icon: Globe },
  { id: "results", label: "settings.tab.results", icon: Folder },
  { id: "history", label: "settings.tab.history", icon: History },
  { id: "about", label: "settings.tab.about", icon: Info },
];

/** Android always saves Results in `Download/Rantakit/`, so it has no "File hasil". */
const TABS = ALL_TABS.filter(({ id }) => !(isAndroidApp && id === "results"));

/** Each language is named in itself, whatever the UI language. */
const LANGUAGE_NAMES: Record<Language, string> = { id: "Bahasa Indonesia", en: "English" };

interface SettingsDialogProps {
  tab: SettingsTab;
  onTabChange: (tab: SettingsTab) => void;
  onClose: () => void;
}

/**
 * "Pengaturan", over the main window: vertical tabs on the left, the tab's
 * panel on the right. Every change applies at once; there is no Save button.
 */
export function SettingsDialog({ tab, onTabChange, onClose }: SettingsDialogProps) {
  const { t } = useT();
  return (
    <Modal isOpen isDismissable size="large" onOpenChange={(open) => !open && onClose()}>
      <Dialog className="flex h-[min(560px,85dvh)] flex-col p-0">
        <div className="flex items-center justify-between gap-4 px-6 pt-5 pb-4">
          <Heading slot="title" className="my-0 font-display text-dialog">
            {t("settings.title")}
          </Heading>
          <Button variant="icon" aria-label={t("common.close")} onPress={onClose}>
            <X aria-hidden className="size-5" />
          </Button>
        </div>
        <Tabs
          orientation="vertical"
          selectedKey={tab}
          onSelectionChange={(key) => onTabChange(key as SettingsTab)}
          className="min-h-0 flex-1 gap-4 px-4 pb-4"
        >
          <TabList aria-label={t("settings.title")} className="w-48 shrink-0 gap-1 overflow-visible">
            {TABS.map(({ id, label, icon: Icon }) => (
              <Tab
                key={id}
                id={id}
                className="min-h-11 w-full gap-3 rounded-xl px-3 py-0 selected:font-semibold hover:bg-badge-bg selected:hover:bg-brand-soft"
              >
                <Icon aria-hidden className="size-5" />
                {t(label)}
              </Tab>
            ))}
          </TabList>
          {/* The panel being left fades out in place, over the one coming in; without
              this box it would jump to the dialog's top corner while it fades. */}
          <TabPanels className="h-full min-w-0 flex-1">
            {TABS.map(({ id }) => (
              <TabPanel key={id} id={id} className="h-full overflow-y-auto rounded-2xl bg-panel p-6">
                {PANELS[id]}
              </TabPanel>
            ))}
          </TabPanels>
        </Tabs>
      </Dialog>
    </Modal>
  );
}

const row = tv({
  extend: focusRing,
  base: [
    "flex min-h-14 w-full items-center gap-3 px-4 text-start text-label",
    "cursor-default select-none [-webkit-tap-highlight-color:transparent] pressed:bg-badge-bg",
  ],
});

interface SettingsPageProps {
  /** The part on screen; null for the list of parts. */
  tab: SettingsTab | null;
  onTabChange: (tab: SettingsTab | null) => void;
}

/**
 * "Pengaturan" on a phone, as a page of its own: the list of parts, and
 * each part as its own page. Back from a part returns to the list.
 */
export function SettingsPage({ tab, onTabChange }: SettingsPageProps) {
  const { t } = useT();
  if (tab) {
    return (
      <div className="flex flex-1 flex-col">
        <header className="flex h-14 shrink-0 items-center gap-2 px-2">
          <Button variant="icon" aria-label={t("nav.back")} onPress={() => onTabChange(null)}>
            <ArrowLeft aria-hidden className="size-5" />
          </Button>
          <h1 className="truncate ps-1 font-display text-section">{t(`settings.tab.${tab}`)}</h1>
        </header>
        <div className="px-5 pt-2 pb-8">{PANELS[tab]}</div>
      </div>
    );
  }
  return (
    <div className="flex flex-1 flex-col gap-6 px-5 pt-6 pb-8">
      <h1 className="font-display text-title">{t("settings.title")}</h1>
      <ul className="flex flex-col divide-y divide-border overflow-hidden rounded-2xl border border-border bg-surface">
        {TABS.map(({ id, label, icon: Icon }) => (
          <li key={id}>
            <AriaButton onPress={() => onTabChange(id)} className={(renderProps) => row(renderProps)}>
              <Icon aria-hidden className="size-5 shrink-0 text-text-muted" />
              <span className="flex-1">{t(label)}</span>
              <ChevronRight aria-hidden className="size-5 shrink-0 text-text-muted" />
            </AriaButton>
          </li>
        ))}
      </ul>
    </div>
  );
}

function Section({ title, children }: { title: string; children: ReactNode }) {
  return (
    <section className="flex flex-col gap-3">
      <h3 className="text-label font-semibold">{title}</h3>
      {children}
    </section>
  );
}

function AppearancePanel() {
  const { t } = useT();
  const [prefs, update] = usePreferences();
  return (
    <div className="flex flex-col gap-8">
      <Section title={t("settings.theme")}>
        <ToggleButtonGroup
          aria-label={t("settings.theme")}
          selectionMode="single"
          disallowEmptySelection
          selectedKeys={[prefs.theme]}
          onSelectionChange={(keys) => update({ theme: [...keys][0] as Theme })}
          className="[&>*]:flex-1"
        >
          <ToggleButton id="system">
            <Monitor aria-hidden className="size-5" /> {t("settings.theme.system")}
          </ToggleButton>
          <ToggleButton id="light">
            <Sun aria-hidden className="size-5" /> {t("settings.theme.light")}
          </ToggleButton>
          <ToggleButton id="dark">
            <Moon aria-hidden className="size-5" /> {t("settings.theme.dark")}
          </ToggleButton>
        </ToggleButtonGroup>
      </Section>
      <Section title={t("settings.textSize")}>
        <ToggleButtonGroup
          aria-label={t("settings.textSize")}
          selectionMode="single"
          disallowEmptySelection
          selectedKeys={[prefs.textSize]}
          onSelectionChange={(keys) => update({ textSize: [...keys][0] as TextSize })}
          className="[&>*]:flex-1"
        >
          <ToggleButton id="normal">{t("settings.textSize.normal")}</ToggleButton>
          <ToggleButton id="large">{t("settings.textSize.large")}</ToggleButton>
          <ToggleButton id="larger">{t("settings.textSize.larger")}</ToggleButton>
        </ToggleButtonGroup>
        <p className="rounded-xl border border-border bg-surface px-4 py-3">{t("settings.textSize.sample")}</p>
      </Section>
    </div>
  );
}

function LanguagePanel() {
  const { t } = useT();
  const [prefs, update] = usePreferences();
  return (
    <Section title={t("settings.language")}>
      <RadioGroup
        aria-label={t("settings.language")}
        value={prefs.language}
        onChange={(value) => update({ language: value as LanguageChoice })}
      >
        <Radio value="system">{t("settings.language.system", { language: LANGUAGE_NAMES[systemLanguage()] })}</Radio>
        <Radio value="id" lang="id">
          {LANGUAGE_NAMES.id}
        </Radio>
        <Radio value="en" lang="en">
          {LANGUAGE_NAMES.en}
        </Radio>
      </RadioGroup>
    </Section>
  );
}

function ResultsPanel() {
  const { t } = useT();
  const settings = useAppSettings();
  const folder = settings?.resultsFolder ?? null;
  const choose = async () => {
    const picked = await pickResultsFolder().catch(() => null);
    if (picked) await setResultsFolder(picked).catch(() => {});
  };
  return (
    <Section title={t("settings.results.title")}>
      <RadioCards
        label={t("settings.results.title")}
        value={folder ? "folder" : "next"}
        options={[
          { id: "next", label: t("settings.results.next"), description: t("settings.results.next.description") },
          { id: "folder", label: t("settings.results.folder"), description: t("settings.results.folder.description") },
        ]}
        onChange={(choice) => (choice === "next" ? setResultsFolder(null).catch(() => {}) : choose())}
      />
      {folder && (
        <div className="flex items-center gap-3 rounded-xl border border-border bg-surface px-4 py-3">
          <Folder aria-hidden className="size-5 shrink-0 text-text-muted" />
          <span className="min-w-0 flex-1 truncate" title={folder}>
            {folder}
          </span>
          <Button variant="secondary" size="row" onPress={choose}>
            {t("settings.results.change")}
          </Button>
        </div>
      )}
    </Section>
  );
}

/** "Simpan riwayat": turning it off asks first, since it clears History. */
function HistoryPanel() {
  const { t } = useT();
  const settings = useAppSettings();
  const [asking, setAsking] = useState(false);
  return (
    <Section title={t("settings.tab.history")}>
      <div className="flex items-start justify-between gap-6">
        <div className="flex flex-col gap-0.5">
          <span className="text-label">{t("settings.keepHistory")}</span>
          <span className="text-small text-text-muted">{t("settings.keepHistory.description")}</span>
        </div>
        <Switch
          aria-label={t("settings.keepHistory")}
          isDisabled={settings === null}
          isSelected={settings?.keepHistory ?? true}
          onChange={(keep) => (keep ? setKeepHistory(true).catch(() => {}) : setAsking(true))}
        />
      </div>
      <DialogTrigger isOpen={asking} onOpenChange={setAsking}>
        <Modal isDismissable>
          <AlertDialog
            title={t("settings.keepHistory.offTitle")}
            variant="danger"
            actionLabel={t("settings.keepHistory.off")}
            cancelLabel={t("common.cancel")}
            onAction={() => setKeepHistory(false).catch(() => {})}
          >
            {t("settings.keepHistory.offBody")}
          </AlertDialog>
        </Modal>
      </DialogTrigger>
    </Section>
  );
}

/** "Tentang", with "Cek versi baru" on desktop and in the APK from GitHub; a store updates the others. */
function AboutPanel() {
  const { t } = useT();
  const settings = useAppSettings();
  const [version, setVersion] = useState<string>();
  const [canCheck, setCanCheck] = useState(isDesktopApp || !isTauri());
  useEffect(() => {
    if (isTauri()) getVersion().then(setVersion).catch(() => {});
    if (isAndroidApp) invoke<boolean>("can_check_updates").then(setCanCheck).catch(() => {});
  }, []);
  return (
    <div className="flex flex-col items-center gap-4 py-6 text-center">
      <img src="/rantakit.svg" alt="" className="size-16" />
      <div className="flex flex-col">
        <span className="font-display text-dialog">Rantakit</span>
        <span className="text-small text-text-muted">
          {version ? t("about.version", { version }) : t("about.devVersion")}
        </span>
      </div>
      <p className="max-w-sm">{t("about.tagline")}</p>
      <p className="max-w-sm text-small text-text-muted">{t("about.license")}</p>
      <div className="flex flex-wrap justify-center gap-4 text-small">
        <Link onPress={() => openExternal(LICENSE_URL)}>{t("about.viewLicense")}</Link>
        <Link onPress={() => openExternal(SOURCE_URL)}>{t("about.source")}</Link>
        <Link onPress={() => openExternal(ISSUES_URL)}>{t("about.reportProblem")}</Link>
      </div>
      {canCheck && (
        <div className="mt-4 flex w-full items-start justify-between gap-6 border-t border-border pt-6 text-start">
          <div className="flex flex-col gap-0.5">
            <span className="text-label">{t("settings.checkUpdates")}</span>
            <span className="text-small text-text-muted">{t("settings.checkUpdates.description")}</span>
          </div>
          <Switch
            aria-label={t("settings.checkUpdates")}
            isDisabled={settings === null}
            isSelected={settings?.checkUpdates ?? true}
            onChange={(check) => setCheckUpdates(check).catch(() => {})}
          />
        </div>
      )}
    </div>
  );
}

const PANELS: Record<SettingsTab, ReactNode> = {
  appearance: <AppearancePanel />,
  language: <LanguagePanel />,
  results: <ResultsPanel />,
  history: <HistoryPanel />,
  about: <AboutPanel />,
};
