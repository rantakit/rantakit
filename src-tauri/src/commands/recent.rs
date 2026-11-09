use rantakit_tasks::{Platform, RecentLimits, RecentTools, ToolId, home_tools as tools_on_home};
use tauri::State;

use crate::error::Result;

/// The last Size limits used, in bytes, newest first.
#[tauri::command]
pub fn recent_limits(limits: State<'_, RecentLimits>) -> Result<Vec<u64>> {
    Ok(limits.list()?)
}

/// The Tools Home shows in its grid: the recently used ones, newest first,
/// topped up with this platform's defaults. Recent Tools are kept even while
/// History is off.
#[tauri::command]
pub fn home_tools(tools: State<'_, RecentTools>) -> Result<Vec<&'static str>> {
    let recent = tools.list()?;
    Ok(tools_on_home(Platform::current(), &recent)
        .into_iter()
        .map(ToolId::as_str)
        .collect())
}
