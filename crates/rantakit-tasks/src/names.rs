use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// Where a Result goes: `ijazah.pdf` with the suffix `compressed` becomes
/// `<folder>/ijazah (compressed).pdf`, then `ijazah (compressed) (2).pdf`, …
/// when the name is taken, so no file is ever written over.
pub fn result_path(original: &Path, folder: &Path, suffix: &str, extension: &str) -> PathBuf {
    (1..)
        .map(|n| folder.join(result_name(original, suffix, extension, n)))
        .find(|path| !path.try_exists().unwrap_or(true))
        .expect("some name is always free")
}

/// The `n`th name `result_path` tries, counting from 1: `ijazah (compressed).pdf`,
/// then `ijazah (compressed) (2).pdf`. For folders the app cannot look into
/// through a path, such as `Download/` on Android.
pub fn result_name(original: &Path, suffix: &str, extension: &str, n: u32) -> String {
    let stem = original.file_stem().unwrap_or_default().to_string_lossy();
    match n {
        1 => format!("{stem} ({suffix}).{extension}"),
        n => format!("{stem} ({suffix}) ({n}).{extension}"),
    }
}

/// Where several Results from one Original go (X3 in docs/stories/v0.4.md):
/// a new folder `<folder>/ijazah (pages)`, then `ijazah (pages) (2)`, … when
/// the name is taken. The folder is made here, so no other Task can take it.
pub fn result_folder(original: &Path, folder: &Path, suffix: &str) -> io::Result<PathBuf> {
    for n in 1.. {
        let path = folder.join(result_folder_name(original, suffix, n));
        match fs::create_dir(&path) {
            Ok(()) => return Ok(path),
            Err(err) if err.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(err) => return Err(err),
        }
    }
    unreachable!("some name is always free")
}

/// The `n`th name `result_folder` tries, counting from 1: `ijazah (pages)`,
/// then `ijazah (pages) (2)`.
pub fn result_folder_name(original: &Path, suffix: &str, n: u32) -> String {
    let stem = original.file_stem().unwrap_or_default().to_string_lossy();
    match n {
        1 => format!("{stem} ({suffix})"),
        n => format!("{stem} ({suffix}) ({n})"),
    }
}

/// A Result named on its own, not after an Original, such as Gabung PDF's
/// `<folder>/merged 2026-11-07.pdf`, then `merged 2026-11-07 (2).pdf`.
pub fn named_path(folder: &Path, stem: &str, extension: &str) -> PathBuf {
    (1..)
        .map(|n| folder.join(numbered(&format!("{stem}.{extension}"), n)))
        .find(|path| !path.try_exists().unwrap_or(true))
        .expect("some name is always free")
}

/// The `n`th name to try for a file or folder called `name`, counting from 1:
/// `ijazah (edited).pdf`, then `ijazah (edited) (2).pdf`; `ijazah (pages)`,
/// then `ijazah (pages) (2)`. A name with no extension, such as a folder's,
/// gets the number at its end.
pub fn numbered(name: &str, n: u32) -> String {
    if n == 1 {
        return name.to_string();
    }
    let path = Path::new(name);
    match (path.file_stem(), path.extension()) {
        (Some(stem), Some(extension)) if !name.ends_with(')') => format!(
            "{} ({n}).{}",
            stem.to_string_lossy(),
            extension.to_string_lossy()
        ),
        _ => format!("{name} ({n})"),
    }
}

/// The words for pages in a Result's name, counting from 1: `page 4`, or
/// `pages 1-3, 5` with runs joined up.
pub fn pages_suffix(pages: &[usize]) -> String {
    let mut runs: Vec<(usize, usize)> = Vec::new();
    for &page in pages {
        match runs.last_mut() {
            Some((_, end)) if *end + 1 == page => *end = page,
            _ => runs.push((page, page)),
        }
    }
    let text: Vec<String> = runs
        .iter()
        .map(|&(start, end)| match start == end {
            true => start.to_string(),
            false => format!("{start}-{end}"),
        })
        .collect();
    match pages.len() {
        1 => format!("page {}", text.join(", ")),
        _ => format!("pages {}", text.join(", ")),
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

    #[test]
    fn adds_suffix_and_counts_taken_names() {
        let dir = std::env::temp_dir().join("rantakit-tasks-names");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let original = dir.join("ijazah.pdf");

        let first = result_path(&original, &dir, "compressed", "pdf");
        assert_eq!(first, dir.join("ijazah (compressed).pdf"));

        fs::write(&first, b"").unwrap();
        let second = result_path(&original, &dir, "compressed", "pdf");
        assert_eq!(second, dir.join("ijazah (compressed) (2).pdf"));

        fs::write(&second, b"").unwrap();
        assert_eq!(
            result_path(&original, &dir, "compressed", "pdf"),
            dir.join("ijazah (compressed) (3).pdf")
        );
    }

    #[test]
    fn several_results_get_a_new_folder_named_after_the_original() {
        let dir = std::env::temp_dir().join("rantakit-tasks-folders");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let original = dir.join("ijazah.pdf");

        let first = result_folder(&original, &dir, "pages").unwrap();
        assert_eq!(first, dir.join("ijazah (pages)"));
        assert!(first.is_dir());

        let second = result_folder(&original, &dir, "pages").unwrap();
        assert_eq!(second, dir.join("ijazah (pages) (2)"));

        // A file with the folder's name counts as taken too.
        fs::write(dir.join("ijazah (pages) (3)"), b"").unwrap();
        assert_eq!(
            result_folder(&original, &dir, "pages").unwrap(),
            dir.join("ijazah (pages) (4)")
        );
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn page_names_join_runs() {
        assert_eq!(pages_suffix(&[4]), "page 4");
        assert_eq!(pages_suffix(&[1, 2, 3, 5]), "pages 1-3, 5");
        assert_eq!(pages_suffix(&[6, 7, 8, 9, 10]), "pages 6-10");
        assert_eq!(pages_suffix(&[3, 1]), "pages 3, 1");
    }

    #[test]
    fn taken_names_get_a_number() {
        assert_eq!(numbered("merged 2026-11-07.pdf", 1), "merged 2026-11-07.pdf");
        assert_eq!(numbered("merged 2026-11-07.pdf", 2), "merged 2026-11-07 (2).pdf");
        assert_eq!(numbered("ijazah (pages)", 3), "ijazah (pages) (3)");

        let dir = std::env::temp_dir().join("rantakit-tasks-named");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let first = named_path(&dir, "merged 2026-11-07", "pdf");
        assert_eq!(first, dir.join("merged 2026-11-07.pdf"));
        fs::write(&first, b"").unwrap();
        assert_eq!(
            named_path(&dir, "merged 2026-11-07", "pdf"),
            dir.join("merged 2026-11-07 (2).pdf")
        );
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn extension_follows_the_result_not_the_original() {
        let dir = std::env::temp_dir().join("rantakit-tasks-names-ext");

        assert_eq!(
            result_path(Path::new("foto.png"), &dir, "converted", "jpg"),
            dir.join("foto (converted).jpg")
        );
    }
}
