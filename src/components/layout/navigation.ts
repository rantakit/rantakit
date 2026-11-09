import type { ComponentType } from "react";
import { GridSquares, House, Inbox, Settings, type IconProps } from "@keyline-icons/react";
import {
  GridSquares as GridSquaresFill,
  House as HouseFill,
  Inbox as InboxFill,
  Settings as SettingsFill,
} from "@keyline-icons/react/fill";

/** Home, the All tools page ("Peralatan"), and History. */
export type Page = "home" | "tools" | "history";
/** Where a navigation item leads: a page, or Settings (a dialog over the page, or a page on a phone). */
export type NavTarget = Page | "settings";

type Icon = ComponentType<IconProps>;

export interface NavItem {
  id: NavTarget;
  label: "nav.home" | "nav.tools" | "nav.history" | "nav.settings";
  icon: Icon;
  /** Shown while the item is the current page. */
  currentIcon: Icon;
}

export const MAIN_NAV: NavItem[] = [
  { id: "home", label: "nav.home", icon: House, currentIcon: HouseFill },
  { id: "tools", label: "nav.tools", icon: GridSquares, currentIcon: GridSquaresFill },
  { id: "history", label: "nav.history", icon: Inbox, currentIcon: InboxFill },
];

export const SETTINGS_NAV: NavItem = {
  id: "settings",
  label: "nav.settings",
  icon: Settings,
  currentIcon: SettingsFill,
};
