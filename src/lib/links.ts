import { isTauri } from "@tauri-apps/api/core";
import { openUrl } from "@tauri-apps/plugin-opener";

// Keep in sync with the macOS menu in src-tauri/src/menu.rs.
export const SOURCE_URL = "https://github.com/rantakit/rantakit";
export const LICENSE_URL = "https://www.gnu.org/licenses/agpl-3.0.html";
/** Where "Report a problem" in the title bar menu and in Settings' About leads. */
export const ISSUES_URL = `${SOURCE_URL}/issues`;
/** Where "Versi baru" sends people whose install can't update itself. */
export const DOWNLOAD_URL = `${SOURCE_URL}/releases/latest`;

/** Open a link in the user's browser. */
export function openExternal(url: string) {
  return isTauri() ? openUrl(url) : window.open(url, "_blank", "noopener");
}
