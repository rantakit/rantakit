import { useState } from "react";
import { Menu as MenuIcon } from "@keyline-icons/react";
import { Button, type ButtonRenderProps, type Key } from "react-aria-components";
import { Menu, MenuItem, MenuSeparator, MenuTrigger, SubmenuTrigger } from "@/components/ui/Menu";
import { usePreferences, type Preferences } from "@/features/settings/preferences";
import { useT } from "@/i18n";
import { shortcutLabel, type AppCommand } from "@/lib/appCommands";
import { editableField, MENU_ATTRIBUTE } from "@/lib/textEditing";
import { useCanUndo } from "@/lib/undoTarget";
import { useWideLayout } from "@/lib/wideLayout";

export interface AppMenuProps {
  canBack: boolean;
  canForward: boolean;
  sidebarOpen: boolean;
  onCommand: (command: AppCommand) => void;
  className: (renderProps: ButtonRenderProps) => string;
}

const TEXT_SIZES: Preferences["textSize"][] = ["normal", "large", "larger"];
const THEMES: Preferences["theme"][] = ["light", "dark", "system"];

/**
 * The hamburger menu at the start of the title bar on Windows and Linux: File, Edit,
 * View, Go, and Help, each opening its own list. macOS has its
 * menu bar instead.
 */
export function AppMenu({ canBack, canForward, sidebarOpen, onCommand, className }: AppMenuProps) {
  const { t } = useT();
  const [prefs, setPrefs] = usePreferences();
  const canUndo = useCanUndo();
  const wide = useWideLayout();
  const [open, setOpen] = useState(false);
  // Read as the menu opens: the text field the user was typing in, if any.
  const text = open ? editableField() : null;

  const run = (key: Key) => onCommand(key as AppCommand);

  /** One command with its shortcut, if it has one. */
  const item = (command: AppCommand, label: string, isDisabled = false) => (
    <MenuItem id={command} textValue={label} isDisabled={isDisabled}>
      {label}
      {shortcutLabel(command) && (
        <kbd className="ms-auto ps-8 font-sans text-small text-text-muted group-disabled:text-text-disabled">
          {shortcutLabel(command)}
        </kbd>
      )}
    </MenuItem>
  );

  return (
    <MenuTrigger isOpen={open} onOpenChange={setOpen} placement="bottom start">
      <Button aria-label={t("menu.label")} className={className} {...{ [MENU_ATTRIBUTE]: "" }}>
        <MenuIcon aria-hidden className="size-4" />
      </Button>
      <Menu>
        <SubmenuTrigger>
          <MenuItem>{t("menu.file")}</MenuItem>
          <Menu onAction={run}>
            {item("open_files", t("menu.open"))}
            {item("open_settings", t("nav.settings"))}
            {item("quit", t("menu.quit"))}
          </Menu>
        </SubmenuTrigger>
        <SubmenuTrigger>
          <MenuItem>{t("menu.edit")}</MenuItem>
          <Menu onAction={run}>
            {item("undo", t("arrange.undo"), !canUndo)}
            <MenuSeparator />
            {item("cut", t("menu.cut"), !text?.canChange || !text.hasSelection)}
            {item("copy", t("menu.copy"), !text?.hasSelection)}
            {item("paste", t("menu.paste"), !text?.canChange)}
            {item("select_all", t("menu.selectAll"), !text?.hasText)}
          </Menu>
        </SubmenuTrigger>
        <SubmenuTrigger>
          <MenuItem>{t("menu.view")}</MenuItem>
          <Menu onAction={run}>
            {/* The sidebar only exists from md up. */}
            {item("toggle_sidebar", t(sidebarOpen ? "nav.sidebar.hide" : "nav.sidebar.show"), !wide)}
            <MenuSeparator />
            <SubmenuTrigger>
              <MenuItem>{t("settings.textSize")}</MenuItem>
              <Menu
                selectionMode="single"
                selectedKeys={[prefs.textSize]}
                onAction={(key) => setPrefs({ textSize: key as Preferences["textSize"] })}
              >
                {TEXT_SIZES.map((size) => (
                  <MenuItem key={size} id={size}>
                    {t(`settings.textSize.${size}`)}
                  </MenuItem>
                ))}
              </Menu>
            </SubmenuTrigger>
            <SubmenuTrigger>
              <MenuItem>{t("settings.theme")}</MenuItem>
              <Menu
                selectionMode="single"
                selectedKeys={[prefs.theme]}
                onAction={(key) => setPrefs({ theme: key as Preferences["theme"] })}
              >
                {THEMES.map((theme) => (
                  <MenuItem key={theme} id={theme}>
                    {t(`settings.theme.${theme}`)}
                  </MenuItem>
                ))}
              </Menu>
            </SubmenuTrigger>
          </Menu>
        </SubmenuTrigger>
        <SubmenuTrigger>
          <MenuItem>{t("menu.go")}</MenuItem>
          <Menu onAction={run}>
            {item("go_home", t("nav.home"))}
            {item("go_history", t("nav.history"))}
            <MenuSeparator />
            {item("go_back", t("nav.back"), !canBack)}
            {item("go_forward", t("nav.forward"), !canForward)}
          </Menu>
        </SubmenuTrigger>
        <SubmenuTrigger>
          <MenuItem>{t("menu.help")}</MenuItem>
          <Menu onAction={run}>
            {item("open_source", t("menu.source"))}
            {item("report_problem", t("menu.reportProblem"))}
            {item("open_license", t("menu.license"))}
            <MenuSeparator />
            {item("open_about", t("menu.about"))}
          </Menu>
        </SubmenuTrigger>
      </Menu>
    </MenuTrigger>
  );
}
