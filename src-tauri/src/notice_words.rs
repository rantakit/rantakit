//! The words of a Task's notification, which only Android shows. Built on
//! every platform so that its tests run with the others.
#![cfg_attr(not(target_os = "android"), allow(dead_code))]

use rantakit_tasks::ToolId;

/// The notification's words in one language.
pub struct Words {
    english: bool,
    /// The channels' names, as the phone's Settings list them.
    pub running: &'static str,
    pub finished: &'static str,
}

pub const ID: Words = Words {
    english: false,
    running: "Tugas berjalan",
    finished: "Tugas selesai",
};

pub const EN: Words = Words {
    english: true,
    running: "Running tasks",
    finished: "Finished tasks",
};

fn tool_name(words: &Words, tool: ToolId) -> &'static str {
    match (tool, words.english) {
        (ToolId::CompressPhoto, false) => "Kompres Foto",
        (ToolId::CompressPhoto, true) => "Compress Photo",
        (ToolId::CompressPdf, false) => "Kompres PDF",
        (ToolId::CompressPdf, true) => "Compress PDF",
        (ToolId::CompressDocument, false) => "Kompres Dokumen",
        (ToolId::CompressDocument, true) => "Compress Document",
        (ToolId::ConvertToJpg, false) => "Ubah ke JPG",
        (ToolId::ConvertToJpg, true) => "Convert to JPG",
        (ToolId::ConvertToPng, false) => "Ubah ke PNG",
        (ToolId::ConvertToPng, true) => "Convert to PNG",
        (ToolId::ResizePhoto, false) => "Ubah Ukuran Foto",
        (ToolId::ResizePhoto, true) => "Resize Photo",
        _ => "Rantakit",
    }
}

/// "Kompres Foto · 3 dari 12 file", counting the file being worked on.
pub fn progress_title(words: &Words, tool: ToolId, file: usize, files: usize) -> String {
    let tool = tool_name(words, tool);
    let at = (file + 1).min(files);
    if words.english {
        let noun = if files == 1 { "file" } else { "files" };
        format!("{tool} · {at} of {files} {noun}")
    } else {
        format!("{tool} · {at} dari {files} file")
    }
}

/// "Selesai · 12 file hasil di Download/Rantakit", counting failed files too.
pub fn done_title(words: &Words, results: usize, failed: usize) -> String {
    if words.english {
        let count =
            |n: usize, one: &str, many: &str| format!("{n} {}", if n == 1 { one } else { many });
        match (results, failed) {
            (_, 0) => format!(
                "Done · {} in Download/Rantakit",
                count(results, "result", "results")
            ),
            (0, _) => format!("Done · {} failed", count(failed, "file", "files")),
            _ => format!(
                "Done · {} in Download/Rantakit, {failed} failed",
                count(results, "result", "results")
            ),
        }
    } else {
        match (results, failed) {
            (_, 0) => format!("Selesai · {results} file hasil di Download/Rantakit"),
            (0, _) => format!("Selesai · {failed} file gagal"),
            _ => format!("Selesai · {results} file hasil di Download/Rantakit, {failed} gagal"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn progress_counts_the_file_being_worked_on() {
        assert_eq!(
            progress_title(&ID, ToolId::CompressPhoto, 2, 12),
            "Kompres Foto · 3 dari 12 file"
        );
        assert_eq!(
            progress_title(&EN, ToolId::CompressPdf, 0, 1),
            "Compress PDF · 1 of 1 file"
        );
    }

    #[test]
    fn done_counts_failed_files() {
        assert_eq!(
            done_title(&ID, 12, 0),
            "Selesai · 12 file hasil di Download/Rantakit"
        );
        assert_eq!(
            done_title(&ID, 10, 2),
            "Selesai · 10 file hasil di Download/Rantakit, 2 gagal"
        );
        assert_eq!(
            done_title(&EN, 1, 0),
            "Done · 1 result in Download/Rantakit"
        );
        assert_eq!(done_title(&EN, 0, 3), "Done · 3 files failed");
    }
}
