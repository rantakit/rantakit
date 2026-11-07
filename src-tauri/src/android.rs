//! Files on Android. The pickers hand over `content://` addresses, while the
//! engines read and write paths: an Original is read through its open file,
//! a Task works on a copy in the app's cache, and each Result is then saved
//! as a new file in `Download/Rantakit/` through MediaStore.

use std::fs::{self, File};
use std::io;
use std::path::{Path, PathBuf};

use rantakit_tasks::{
    Details, ErrorKind, FileInfo, FileKind, FolderFile, ResultFile, numbered,
};
use tauri::{AppHandle, Manager};
use tauri_plugin_android_fs::{AndroidFsExt, FsUri, ImageFormat, PublicGeneralPurposeDir, Size};

/// The folder in `Download/` that Results are saved to.
const RESULTS_FOLDER: &str = "Rantakit";

/// Whether `path` is a `content://` address rather than a file path.
pub fn is_content(path: &Path) -> bool {
    path.to_str()
        .is_some_and(|path| path.starts_with("content://"))
}

fn uri(path: &Path) -> FsUri {
    FsUri::from_uri(path.to_string_lossy())
}

/// The name the picker showed, such as `ktp.jpg`, which the address itself
/// does not contain.
pub fn name(app: &AppHandle, path: &Path) -> Option<String> {
    let name = app.android_fs().get_name(&uri(path)).ok()?;
    Some(name.replace('/', "_"))
}

/// Opens the system file picker for `mime_types` and returns the chosen
/// files' addresses, or none when cancelled. Unlike the Photo Picker, which
/// hands over names like `33.jpg`, it keeps each file's real name.
pub async fn pick(app: &AppHandle, mime_types: &[String]) -> Vec<PathBuf> {
    let mime_types: Vec<&str> = mime_types.iter().map(String::as_str).collect();
    let picker = app.android_fs_async().picker();
    match picker.pick_files(None, &mime_types, false).await {
        Ok(uris) => uris.into_iter().map(|uri| uri.uri.into()).collect(),
        Err(err) => {
            eprintln!("picker failed: {err}");
            Vec::new()
        }
    }
}

/// `rantakit_tasks::file_info` for a `content://` Original, read through
/// its open file so that nothing is copied.
pub fn file_info(app: &AppHandle, path: &Path) -> FileInfo {
    let fs = app.android_fs();
    let uri = uri(path);
    let info = fs
        .open_file_readable(&uri)
        .map(rantakit_tasks::file_info_of)
        .unwrap_or_default();
    FileInfo {
        size: info
            .size
            .filter(|&size| size > 0)
            .or_else(|| fs.get_len(&uri).ok()),
        ..info
    }
}

/// What a `content://` file is, judged by its content like `detect_kind`.
pub fn kind(app: &AppHandle, path: &Path) -> Option<FileKind> {
    let mut file = app.android_fs().open_file_readable(&uri(path)).ok()?;
    rantakit_tasks::read_kind(&mut file)
}

/// A small JPG of a `content://` photo, made by the system.
/// The whole of a chosen file, such as a PDF whose pages are drawn.
pub fn read(app: &AppHandle, path: &Path) -> Result<Vec<u8>, ErrorKind> {
    use std::io::Read;

    let mut bytes = Vec::new();
    app.android_fs()
        .open_file_readable(&uri(path))
        .map_err(io::Error::from)
        .and_then(|mut file| file.read_to_end(&mut bytes))
        .map_err(|err| ErrorKind::reading(&err))?;
    Ok(bytes)
}

pub fn thumbnail(app: &AppHandle, path: &Path, size: u32) -> Option<Vec<u8>> {
    let preferred = Size {
        width: size,
        height: size,
    };
    let fs = app.android_fs();
    fs.get_thumbnail(&uri(path), preferred, ImageFormat::Jpeg)
        .ok()
        .flatten()
}

/// The MediaStore address of a Result, which other apps can be given. Results
/// are known by their path in `Download/Rantakit/`, the one History keeps.
fn result_uri(app: &AppHandle, path: &Path) -> tauri_plugin_android_fs::Result<FsUri> {
    if is_content(path) {
        return Ok(uri(path));
    }
    app.android_fs().public_storage().scan_by_path(path, None)
}

/// Whether a Result is still where it was saved, for History. Android 11 and
/// later show the app the files it saved in `Download/` by path, as do 8 and
/// 9 with the storage permission; Android 10 shows them only through
/// MediaStore, so there it asks MediaStore.
pub fn result_exists(app: &AppHandle, path: &Path) -> bool {
    if path.exists() {
        return true;
    }
    let android_10 = app.android_fs().api_level().is_ok_and(|level| level == 29);
    android_10 && result_uri(app, path).is_ok()
}

/// "Buka": the system asks which app opens the Result.
pub fn open(app: &AppHandle, path: &Path) -> tauri_plugin_android_fs::Result<()> {
    let uri = result_uri(app, path)?;
    app.android_fs().opener().open_file(&uri)
}

/// "Bagikan": the system share sheet, with every Result as one share.
pub fn share(app: &AppHandle, paths: &[PathBuf]) -> tauri_plugin_android_fs::Result<()> {
    let uris = paths
        .iter()
        .map(|path| result_uri(app, path))
        .collect::<tauri_plugin_android_fs::Result<Vec<_>>>()?;
    app.android_fs().opener().share_files(&uris)
}

/// Android 9 and earlier need the storage permission to save Results; later
/// versions always have it.
pub async fn can_save(app: &AppHandle) -> bool {
    let storage = app.android_fs_async().public_storage();
    storage.check_permission().await.unwrap_or(false)
}

/// Shows the system's storage permission prompt when it is needed.
pub async fn ask_to_save(app: &AppHandle) -> bool {
    let storage = app.android_fs_async().public_storage();
    storage.request_permission().await.unwrap_or(false)
}

fn workspaces(app: &AppHandle) -> io::Result<PathBuf> {
    let cache = app.path().app_cache_dir().map_err(io::Error::other)?;
    Ok(cache.join("tasks"))
}

/// Removes what Tasks that Android cut short left in the cache; called at launch.
pub fn clear_workspaces(app: &AppHandle) {
    if let Ok(dir) = workspaces(app) {
        let _ = fs::remove_dir_all(dir);
    }
}

/// The copies one Task works on, in the app's cache. Removed when dropped.
pub struct Workspace {
    dir: PathBuf,
}

impl Workspace {
    pub fn new(app: &AppHandle, task: u64) -> io::Result<Self> {
        let dir = workspaces(app)?.join(task.to_string());
        fs::create_dir_all(dir.join("results"))?;
        Ok(Self { dir })
    }

    /// Where the engines write Results before they are saved.
    pub fn results(&self) -> PathBuf {
        self.dir.join("results")
    }

    /// Copies the `file`th Original in under its own name, so that its Result
    /// is named after it. A plain path is used as it is.
    pub fn copy_in(
        &self,
        app: &AppHandle,
        file: usize,
        original: &Path,
    ) -> Result<PathBuf, ErrorKind> {
        if !is_content(original) {
            return Ok(original.to_path_buf());
        }
        let name = name(app, original).unwrap_or_else(|| "file".into());
        let mut from = app
            .android_fs()
            .open_file_readable(&uri(original))
            .map_err(|err| ErrorKind::reading(&err.into()))?;
        let folder = self.dir.join(file.to_string());
        let copy = folder.join(name);
        fs::create_dir_all(&folder)
            .and_then(|()| io::copy(&mut from, &mut File::create(&copy)?))
            .map_err(|err| ErrorKind::writing(&err))?;
        Ok(copy)
    }

    /// Removes the `file`th Original's copy once its Result is saved.
    pub fn remove(&self, file: usize) {
        let _ = fs::remove_dir_all(self.dir.join(file.to_string()));
    }
}

impl Drop for Workspace {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.dir);
    }
}

/// Saves `result` as a new file in `Download/Rantakit/` and removes its copy
/// in the cache. It keeps the name the Task gave it in the cache, numbered
/// against the Results this app saved there before; MediaStore numbers any
/// other taken name itself, so no file is ever replaced. Several Results
/// from one Original (X3) go into a new folder there, such as
/// `Download/Rantakit/ijazah (pages)/`.
pub fn save(app: &AppHandle, result: ResultFile) -> Result<ResultFile, ErrorKind> {
    let made = result
        .path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .into_owned();
    if let Details::Folder { files } = &result.details {
        return save_folder(app, &made, files.clone(), result);
    }
    let results = results_folder(app);
    let name = free_name(results.as_deref(), |n| numbered(&made, n));
    let path = write_new(app, &Path::new(RESULTS_FOLDER).join(&name), &result.path)?;
    Ok(ResultFile { path, ..result })
}

/// Each file of a folder Result, into a new folder in `Download/Rantakit/`.
/// The files already saved stay when a later one fails, as in a Task that
/// stops halfway.
fn save_folder(
    app: &AppHandle,
    made: &str,
    files: Vec<FolderFile>,
    result: ResultFile,
) -> Result<ResultFile, ErrorKind> {
    let results = results_folder(app);
    let folder = free_name(results.as_deref(), |n| numbered(made, n));
    let relative = Path::new(RESULTS_FOLDER).join(&folder);
    let mut saved = Vec::with_capacity(files.len());
    for file in files {
        let name = file.path.file_name().unwrap_or_default();
        let path = write_new(app, &relative.join(name), &file.path)?;
        saved.push(FolderFile { path, ..file });
    }
    let path = saved
        .first()
        .and_then(|file| file.path.parent())
        .map(Path::to_path_buf)
        .unwrap_or_else(|| results.unwrap_or_default().join(&folder));
    Ok(ResultFile {
        path,
        details: Details::Folder { files: saved },
        ..result
    })
}

/// `Download/Rantakit/` as a path, only to look for names already taken;
/// files are written through MediaStore.
fn results_folder(app: &AppHandle) -> Option<PathBuf> {
    #[allow(deprecated)]
    let download = app
        .android_fs()
        .public_storage()
        .resolve_path(None, PublicGeneralPurposeDir::Download);
    download.map(|download| download.join(RESULTS_FOLDER)).ok()
}

/// The first of `name(1)`, `name(2)`, … not taken in `folder`.
fn free_name(folder: Option<&Path>, name: impl Fn(u32) -> String) -> String {
    let taken = |candidate: &str| folder.is_some_and(|folder| folder.join(candidate).exists());
    (1..1000)
        .map(&name)
        .find(|candidate| !taken(candidate))
        .unwrap_or_else(|| name(1))
}

/// Copies `from` to a new file at `relative` inside `Download/`, removes
/// `from`, and returns the new file's path.
fn write_new(app: &AppHandle, relative: &Path, from: &Path) -> Result<PathBuf, ErrorKind> {
    let fs = app.android_fs();
    let storage = fs.public_storage();
    let cannot_write = |err: tauri_plugin_android_fs::Error| ErrorKind::writing(&err.into());
    // Pending until it is whole: other apps don't see it, and if Android
    // stops the Task halfway, Android 10 and later remove it after a week.
    let uri = storage
        .create_new_file_with_pending(None, PublicGeneralPurposeDir::Download, relative, None)
        .map_err(cannot_write)?;
    let written = fs
        .open_file_writable(&uri)
        .map_err(io::Error::from)
        .and_then(|mut to| {
            io::copy(&mut File::open(from)?, &mut to)?;
            to.sync_all()
        })
        .and_then(|()| storage.set_pending(&uri, false).map_err(io::Error::from));
    if let Err(err) = written {
        let _ = fs.remove_file(&uri);
        return Err(ErrorKind::writing(&err));
    }
    // Android 9 and earlier only show the file in Gallery and Files once scanned.
    let _ = storage.scan(&uri);
    let _ = fs::remove_file(from);

    #[allow(deprecated)] // The path is what the Result page and History show.
    let path = storage.get_path(&uri).unwrap_or_else(|_| {
        let download = storage.resolve_path(None, PublicGeneralPurposeDir::Download);
        download.unwrap_or_default().join(relative)
    });
    Ok(path)
}
