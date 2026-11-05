//! Real, account-scoped local adapters for the DesignFlow hub.
//! No network service or demonstration records are used. The catalog owns only files beneath
//! its root; importing always copies, and a soft deletion changes catalog membership only.
use std::collections::BTreeSet;
use std::fs::{self, File, OpenOptions};
use std::io::Read;
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use designflow_doc::Document;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Conservative local import policy, below the underlying document reader's 16 GiB ceiling.
pub const MAX_IMPORT_BYTES: u64 = 256 * 1024 * 1024;
pub const IMPORT_EXTENSIONS: &[&str] = &["designflow", "svg", "png", "jpg", "jpeg", "webp"];
const MAX_CATALOG_BYTES: u64 = 8 * 1024 * 1024;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Project {
    pub id: String,
    pub title: String,
    pub status: String,
    pub created_at: u64,
    pub updated_at: u64,
    pub favorite: bool,
    /// A generated relative managed path, never an imported source path.
    pub path: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct LibraryItem {
    pub id: String,
    pub title: String,
    pub kind: String,
    pub created_at: u64,
    pub updated_at: u64,
    pub favorite: bool,
    pub path: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", content = "item", rename_all = "snake_case")]
pub enum TrashedRecord {
    Project(Project),
    Library(LibraryItem),
}

impl TrashedRecord {
    fn path(&self) -> &str {
        match self {
            Self::Project(item) => &item.path,
            Self::Library(item) => &item.path,
        }
    }
    fn id(&self) -> &str {
        match self {
            Self::Project(item) => &item.id,
            Self::Library(item) => &item.id,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct TrashItem {
    pub id: String,
    pub title: String,
    /// `project` or `file`, for the user-facing filter.
    pub kind: String,
    pub deleted_at: u64,
    pub original: TrashedRecord,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct Preferences {
    pub workspace_name: String,
    pub autosave: bool,
    pub default_route: String,
    pub theme: String,
    pub locale: String,
    /// Explicit accessibility preference; canvas zoom is independent of interface size.
    pub ui_scale_percent: u16,
}

impl Default for Preferences {
    fn default() -> Self {
        Self {
            workspace_name: "Meu espaço".into(),
            autosave: true,
            default_route: "/".into(),
            theme: "light".into(),
            locale: "pt-BR".into(),
            ui_scale_percent: 100,
        }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct Profile {
    pub name: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct Data {
    pub version: u32,
    pub projects: Vec<Project>,
    pub library: Vec<LibraryItem>,
    pub trash: Vec<TrashItem>,
    pub preferences: Preferences,
    pub profile: Profile,
}

impl Default for Data {
    fn default() -> Self {
        Self {
            version: 1,
            projects: Vec::new(),
            library: Vec::new(),
            trash: Vec::new(),
            preferences: Preferences::default(),
            profile: Profile::default(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct ImportResult {
    pub project: Project,
    pub library_item: LibraryItem,
    /// Genuine parser warnings, never a simulated upload status.
    pub warnings: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct Store {
    pub data: Data,
    pub root: PathBuf,
    revision: Option<String>,
}

impl Store {
    /// Empty accounts are refused: private files must not accidentally share a guest directory.
    pub fn load(account: &str) -> Result<Self, String> {
        let account = account.trim().to_ascii_lowercase();
        if account.is_empty() {
            return Err("Entre em uma conta local para acessar seus arquivos.".into());
        }
        Self::from_directory(config_directory()?.join("workspaces").join(hash(account.as_bytes())))
    }

    /// Explicit directory constructor for isolated integration tests and portable hosts.
    pub fn from_directory(root: PathBuf) -> Result<Self, String> {
        fs::create_dir_all(&root).map_err(io_error)?;
        if fs::symlink_metadata(&root).map_err(io_error)?.file_type().is_symlink() {
            return Err("A pasta do espaço local não pode ser um link simbólico.".into());
        }
        let root = fs::canonicalize(root).map_err(io_error)?;
        for directory in ["projects", "library", "pending-delete"] {
            let path = root.join(directory);
            store_directory_is_not_link(&path)?;
            fs::create_dir_all(&path).map_err(io_error)?;
            if fs::canonicalize(&path).map_err(io_error)? != path {
                return Err("Uma pasta de dados locais aponta para fora do espaço do aplicativo.".into());
            }
        }
        let mut store = Self { root, data: Data::default(), revision: None };
        store.reload()?;
        Ok(store)
    }

    pub fn reload(&mut self) -> Result<(), String> {
        let index = self.root.join("workspace.json");
        self.reject_link(&index)?;
        let bytes = match File::open(&index) {
            Ok(file) => Some(read_open_file_capped(file, MAX_CATALOG_BYTES)?),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
            Err(error) => return Err(io_error(error)),
        };
        let data = match &bytes {
            Some(bytes) => serde_json::from_slice(bytes)
                .map_err(|_| "Os dados locais estão inválidos. Preserve a pasta e tente recuperar uma cópia.".to_owned())?,
            None => Data::default(),
        };
        validate_data(&data)?;
        self.revision = bytes.as_ref().map(|value| hash(value));
        self.data = data;
        Ok(())
    }

    /// Metadata comes from actual file timestamps, not illustrative relative dates.
    pub fn refresh(&mut self) -> Result<(), String> {
        let mut next = self.data.clone();
        let mut changed = false;
        for project in &mut next.projects {
            let path = self.managed_path(&project.path)?;
            let modified =
                fs::metadata(path).map_err(io_error)?.modified().map_err(io_error)?.duration_since(UNIX_EPOCH).map_or(0, |elapsed| elapsed.as_secs());
            if modified > project.updated_at {
                project.updated_at = modified;
                project.status = "active".into();
                changed = true;
            }
        }
        if changed {
            self.persist(next)?;
        }
        Ok(())
    }

    pub fn project_path(&self, id: &str) -> Result<PathBuf, String> {
        let item = self.data.projects.iter().find(|item| item.id == id).ok_or("Projeto não encontrado neste espaço.")?;
        let path = self.managed_path(&item.path)?;
        if !path.is_file() {
            return Err("O arquivo do projeto não está disponível. Recarregue seus projetos.".into());
        }
        Ok(path)
    }

    pub fn library_path(&self, id: &str) -> Result<PathBuf, String> {
        let item = self.data.library.iter().find(|item| item.id == id).ok_or("Arquivo não encontrado na biblioteca.")?;
        let path = self.managed_path(&item.path)?;
        if !path.is_file() {
            return Err("O arquivo da biblioteca não está disponível.".into());
        }
        Ok(path)
    }

    pub fn create_project(&mut self, name: &str, template: Option<&str>) -> Result<Project, String> {
        let title = validated_name(name)?;
        let mut document = match template {
            Some(id) => designflow_engine::templates::find(id)
                .ok_or("Este modelo não está mais disponível.")?
                .build(designflow_color::Color::WHITE)
                .map_err(|error| error.to_string())?,
            None => Document::new(&title),
        };
        document.name = title.clone();
        if template.is_none() {
            for page in document.pages.clone() {
                if let Some(node) = document.node_mut(page)
                    && let designflow_doc::NodeKind::Page { background, .. } = &mut node.kind
                {
                    let [r, g, b, a] = crate::theme::system::CANVAS.to_array();
                    *background = designflow_color::Color {
                        r: f32::from(r) / 255.0,
                        g: f32::from(g) / 255.0,
                        b: f32::from(b) / 255.0,
                        a: f32::from(a) / 255.0,
                    };
                }
            }
        }
        let bytes = designflow_format::save_bytes(&document, None).map_err(|error| error.to_string())?;
        let project = new_project(title, if template.is_some() { "active" } else { "draft" });
        let path = self.managed_path(&project.path)?;
        write_new(&path, &bytes)?;
        let mut next = self.data.clone();
        next.projects.insert(0, project.clone());
        if let Err(error) = self.persist(next) {
            let _ = fs::remove_file(&path);
            return Err(error);
        }
        Ok(project)
    }

    /// Creates an editable project from an existing owned library item, without duplicating it.
    pub fn create_project_from_library(&mut self, id: &str) -> Result<Project, String> {
        let item = self.data.library.iter().find(|item| item.id == id).cloned().ok_or("Arquivo não encontrado na biblioteca.")?;
        let source = self.library_path(id)?;
        let bytes = read_capped(&source, MAX_IMPORT_BYTES)?;
        let hint = source.file_name().and_then(|value| value.to_str()).ok_or("Nome de arquivo inválido.")?;
        let (mut document, _) = designflow_engine::cmd::file::load_bytes(&bytes, hint).map_err(|error| format!("Não foi possível abrir: {error}"))?;
        let title = validated_name(&item.title)?;
        document.name = title.clone();
        let native = designflow_format::save_bytes(&document, None).map_err(|error| error.to_string())?;
        let project = new_project(title, "active");
        let path = self.managed_path(&project.path)?;
        write_new(&path, &native)?;
        let mut next = self.data.clone();
        next.projects.insert(0, project.clone());
        if let Err(error) = self.persist(next) {
            let _ = fs::remove_file(&path);
            return Err(error);
        }
        Ok(project)
    }

    /// Validates content with the existing engine before creating either managed copy.
    pub fn import_file(&mut self, source: &Path) -> Result<ImportResult, String> {
        let extension = source.extension().and_then(|value| value.to_str()).unwrap_or("").to_ascii_lowercase();
        if !IMPORT_EXTENSIONS.contains(&extension.as_str()) {
            return Err("Formato não permitido. Use DesignFlow, SVG, PNG, JPG ou WebP.".into());
        }
        let bytes = read_capped(source, MAX_IMPORT_BYTES)?;
        if bytes.is_empty() {
            return Err("O arquivo está vazio.".into());
        }
        let hint = source.file_name().and_then(|value| value.to_str()).unwrap_or("arquivo.designflow");
        let (mut document, warnings) =
            designflow_engine::cmd::file::load_bytes(&bytes, hint).map_err(|error| format!("Não foi possível importar: {error}"))?;
        let title = validated_name(source.file_stem().and_then(|value| value.to_str()).unwrap_or("Arquivo importado"))?;
        document.name = title.clone();
        let native = designflow_format::save_bytes(&document, None).map_err(|error| error.to_string())?;
        let project = new_project(title.clone(), "active");
        let library_item =
            new_library(title, if matches!(extension.as_str(), "png" | "jpg" | "jpeg" | "webp" | "svg") { "image" } else { "upload" }, &extension);
        let project_path = self.managed_path(&project.path)?;
        let library_path = self.managed_path(&library_item.path)?;
        write_new(&project_path, &native)?;
        if let Err(error) = write_new(&library_path, &bytes) {
            let _ = fs::remove_file(&project_path);
            return Err(error);
        }
        let mut next = self.data.clone();
        next.projects.insert(0, project.clone());
        next.library.insert(0, library_item.clone());
        if let Err(error) = self.persist(next) {
            let _ = fs::remove_file(&project_path);
            let _ = fs::remove_file(&library_path);
            return Err(error);
        }
        Ok(ImportResult { project, library_item, warnings })
    }

    pub fn add_template_to_library(&mut self, template_id: &str) -> Result<LibraryItem, String> {
        let template = designflow_engine::templates::find(template_id).ok_or("Recurso não disponível.")?;
        let document = template.build(designflow_color::Color::WHITE).map_err(|error| error.to_string())?;
        let bytes = designflow_format::save_bytes(&document, None).map_err(|error| error.to_string())?;
        let item = new_library(template.name.to_owned(), "collection", "designflow");
        let path = self.managed_path(&item.path)?;
        write_new(&path, &bytes)?;
        let mut next = self.data.clone();
        next.library.insert(0, item.clone());
        if let Err(error) = self.persist(next) {
            let _ = fs::remove_file(&path);
            return Err(error);
        }
        Ok(item)
    }

    pub fn favorite_project(&mut self, id: &str, favorite: bool) -> Result<(), String> {
        let mut next = self.data.clone();
        next.projects.iter_mut().find(|item| item.id == id).ok_or("Projeto não encontrado.")?.favorite = favorite;
        self.persist(next)
    }

    pub fn favorite_library(&mut self, id: &str, favorite: bool) -> Result<(), String> {
        let mut next = self.data.clone();
        next.library.iter_mut().find(|item| item.id == id).ok_or("Arquivo não encontrado.")?.favorite = favorite;
        self.persist(next)
    }

    pub fn rename_library(&mut self, id: &str, title: &str) -> Result<(), String> {
        let title = validated_name(title)?;
        let mut next = self.data.clone();
        let item = next.library.iter_mut().find(|item| item.id == id).ok_or("Arquivo não encontrado.")?;
        item.title = title;
        item.updated_at = now();
        self.persist(next)
    }

    pub fn set_project_title(&mut self, id: &str, title: &str) -> Result<(), String> {
        let title = validated_name(title)?;
        let mut next = self.data.clone();
        let item = next.projects.iter_mut().find(|item| item.id == id).ok_or("Projeto não encontrado.")?;
        item.title = title;
        item.updated_at = now();
        self.persist(next)
    }

    pub fn trash_project(&mut self, id: &str) -> Result<(), String> {
        let mut next = self.data.clone();
        let index = next.projects.iter().position(|item| item.id == id).ok_or("Projeto não encontrado.")?;
        let item = next.projects.remove(index);
        self.managed_path(&item.path)?;
        next.trash.insert(
            0,
            TrashItem {
                id: item.id.clone(),
                title: item.title.clone(),
                kind: "project".into(),
                deleted_at: now(),
                original: TrashedRecord::Project(item),
            },
        );
        self.persist(next)
    }

    pub fn trash_library(&mut self, id: &str) -> Result<(), String> {
        let mut next = self.data.clone();
        let index = next.library.iter().position(|item| item.id == id).ok_or("Arquivo não encontrado.")?;
        let item = next.library.remove(index);
        self.managed_path(&item.path)?;
        next.trash.insert(
            0,
            TrashItem {
                id: item.id.clone(),
                title: item.title.clone(),
                kind: "file".into(),
                deleted_at: now(),
                original: TrashedRecord::Library(item),
            },
        );
        self.persist(next)
    }

    pub fn restore(&mut self, ids: &[String]) -> Result<(), String> {
        if ids.is_empty() {
            return Err("Selecione pelo menos um item para restaurar.".into());
        }
        let selected: BTreeSet<_> = ids.iter().map(String::as_str).collect();
        let mut next = self.data.clone();
        let mut failures = Vec::new();
        next.trash.retain(|item| {
            if !selected.contains(item.id.as_str()) {
                return true;
            }
            if !self.managed_path(item.original.path()).is_ok_and(|path| path.is_file()) {
                failures.push(item.title.clone());
                return true;
            }
            match &item.original {
                TrashedRecord::Project(item) => next.projects.push(item.clone()),
                TrashedRecord::Library(item) => next.library.push(item.clone()),
            }
            false
        });
        self.persist(next)?;
        if failures.is_empty() {
            Ok(())
        } else {
            Err(format!("Não foi possível restaurar: {}. Os demais itens selecionados foram restaurados.", failures.join(", ")))
        }
    }

    /// Call only after a user confirmation. Only explicitly selected catalog records are touched.
    pub fn delete_permanently(&mut self, ids: &[String]) -> Result<(), String> {
        if ids.is_empty() {
            return Err("Selecione pelo menos um item para excluir.".into());
        }
        let mut failures = Vec::new();
        for id in ids.iter().collect::<BTreeSet<_>>() {
            let Some(item) = self.data.trash.iter().find(|item| &item.id == id).cloned() else { continue };
            let original = self.managed_path(item.original.path())?;
            if !original.exists() {
                let mut next = self.data.clone();
                next.trash.retain(|entry| entry.id != item.id);
                self.persist(next)?;
                continue;
            }
            let holding = self.managed_path(&format!(
                "pending-delete/{}",
                original.file_name().and_then(|name| name.to_str()).ok_or("Nome de arquivo inválido.")?
            ))?;
            if holding.exists() {
                failures.push(item.title.clone());
                continue;
            }
            if fs::rename(&original, &holding).is_err() {
                failures.push(item.title.clone());
                continue;
            }
            let mut next = self.data.clone();
            next.trash.retain(|entry| entry.id != item.id);
            if let Err(error) = self.persist(next) {
                let _ = fs::rename(&holding, &original);
                return Err(error);
            }
            if fs::remove_file(&holding).is_err() {
                let rollback = fs::rename(&holding, &original);
                let mut previous = self.data.clone();
                previous.trash.push(item.clone());
                self.persist(previous)?;
                if rollback.is_err() {
                    return Err("O arquivo permanece na pasta pending-delete do espaço local. Recarregue antes de continuar.".into());
                }
                failures.push(item.title);
            }
        }
        if failures.is_empty() {
            Ok(())
        } else {
            Err(format!("Não foi possível excluir: {}. Os demais itens selecionados foram excluídos.", failures.join(", ")))
        }
    }

    pub fn save_preferences(&mut self, preferences: Preferences) -> Result<(), String> {
        let mut preferences = preferences;
        preferences.workspace_name = preferences.workspace_name.trim().to_owned();
        validate_preferences(&preferences)?;
        let mut next = self.data.clone();
        next.preferences = preferences;
        self.persist(next)
    }

    pub fn save_profile(&mut self, name: &str) -> Result<(), String> {
        let name = validated_name(name)?;
        let mut next = self.data.clone();
        next.profile.name = name;
        self.persist(next)
    }

    fn persist(&mut self, next: Data) -> Result<(), String> {
        validate_data(&next)?;
        let path = self.root.join("workspace.json");
        self.reject_link(&path)?;
        let lock_path = self.root.join("catalog.lock");
        self.reject_link(&lock_path)?;
        let lock = OpenOptions::new().read(true).write(true).create(true).truncate(false).open(&lock_path).map_err(io_error)?;
        #[cfg(not(target_arch = "wasm32"))]
        lock.try_lock().map_err(|_| "O espaço está sendo alterado em outra janela. Tente novamente.".to_owned())?;
        let current = if path.exists() { Some(hash(&read_capped(&path, MAX_CATALOG_BYTES)?)) } else { None };
        if current != self.revision {
            return Err("O espaço foi alterado em outra janela. Recarregue antes de salvar.".into());
        }
        let bytes = serde_json::to_vec_pretty(&next).map_err(|_| "Não foi possível preparar os dados locais.".to_owned())?;
        if bytes.len() as u64 > MAX_CATALOG_BYTES {
            return Err("O catálogo local excede o limite de 8 MiB.".into());
        }
        atomic_write(&path, &bytes)?;
        self.revision = Some(hash(&bytes));
        self.data = next;
        drop(lock);
        Ok(())
    }

    fn managed_path(&self, relative: &str) -> Result<PathBuf, String> {
        validate_relative(relative)?;
        let path = self.root.join(relative);
        let parent = path.parent().ok_or("Caminho local inválido.")?;
        store_directory_is_not_link(parent)?;
        if fs::canonicalize(parent).map_err(io_error)? != parent {
            return Err("O arquivo aponta para fora do espaço local.".into());
        }
        self.reject_link(&path)?;
        Ok(path)
    }

    fn reject_link(&self, path: &Path) -> Result<(), String> {
        match fs::symlink_metadata(path) {
            Ok(metadata) if metadata.file_type().is_symlink() => Err("Links simbólicos não são permitidos nos arquivos gerenciados.".into()),
            Ok(_) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(io_error(error)),
        }
    }
}

/// Match the native application's configuration-location policy without falling back to the
/// working directory. An explicitly empty override isolates the process from existing data.
pub fn config_directory() -> Result<PathBuf, String> {
    #[cfg(target_os = "windows")]
    let platform = std::env::var_os("APPDATA").filter(|value| !value.is_empty()).map(PathBuf::from);
    #[cfg(target_os = "macos")]
    let platform = std::env::var_os("HOME").filter(|value| !value.is_empty()).map(|base| PathBuf::from(base).join("Library/Application Support"));
    #[cfg(all(unix, not(target_os = "macos")))]
    let platform = std::env::var_os("XDG_CONFIG_HOME")
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").filter(|value| !value.is_empty()).map(|base| PathBuf::from(base).join(".config")));
    #[cfg(not(any(target_os = "windows", unix)))]
    let platform = None;
    let executable = std::env::current_exe().ok();
    config_directory_from(std::env::var_os("DESIGNFLOW_CONFIG_DIR"), executable.as_deref().and_then(Path::parent), platform)
}

fn config_directory_from(
    override_dir: Option<std::ffi::OsString>,
    executable_dir: Option<&Path>,
    platform: Option<PathBuf>,
) -> Result<PathBuf, String> {
    if let Some(directory) = override_dir {
        return if directory.is_empty() {
            Err("O armazenamento local está desativado por DESIGNFLOW_CONFIG_DIR vazio.".into())
        } else {
            Ok(PathBuf::from(directory))
        };
    }
    if let Some(executable_dir) = executable_dir {
        let base = executable_dir;
        #[cfg(target_os = "macos")]
        let base = if executable_dir.file_name().is_some_and(|name| name == "MacOS")
            && let Some(contents) = executable_dir.parent().filter(|path| path.file_name().is_some_and(|name| name == "Contents"))
            && let Some(bundle) = contents.parent().filter(|path| path.extension().is_some_and(|extension| extension == "app"))
            && let Some(parent) = bundle.parent()
        {
            parent
        } else {
            base
        };
        if base.join("portable").is_file() {
            return Ok(base.join("config"));
        }
    }
    platform.map(|path| path.join("DesignFlow")).ok_or_else(|| "A pasta de dados locais do aplicativo não está disponível.".into())
}

fn store_directory_is_not_link(path: &Path) -> Result<(), String> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() => Err("Links simbólicos não são permitidos nas pastas gerenciadas.".into()),
        Ok(metadata) if !metadata.is_dir() => {
            Err("Uma pasta de dados locais foi substituída por um arquivo. Preserve-o antes de tentar novamente.".into())
        }
        Ok(_) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(io_error(error)),
    }
}

fn new_project(title: String, status: &str) -> Project {
    let id = unique_id();
    Project { path: format!("projects/{id}.designflow"), id, title, status: status.into(), created_at: now(), updated_at: now(), favorite: false }
}
fn new_library(title: String, kind: &str, extension: &str) -> LibraryItem {
    let id = unique_id();
    LibraryItem { path: format!("library/{id}.{extension}"), id, title, kind: kind.into(), created_at: now(), updated_at: now(), favorite: false }
}
fn unique_id() -> String {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    format!(
        "{:x}-{:x}-{:x}",
        SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |time| time.as_nanos()),
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    )
}
fn now() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |time| time.as_secs())
}
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn io_error(error: std::io::Error) -> String {
    format!("Não foi possível acessar os dados locais: {error}")
}
fn validated_name(name: &str) -> Result<String, String> {
    let name = name.trim();
    if name.is_empty() || name.chars().count() > 120 || name.chars().any(char::is_control) {
        return Err("Informe um nome com 1 a 120 caracteres, sem caracteres de controle.".into());
    }
    Ok(name.to_owned())
}
fn validate_preferences(preferences: &Preferences) -> Result<(), String> {
    validated_name(&preferences.workspace_name)?;
    if preferences.workspace_name.trim().chars().count() > 80 {
        return Err("O nome do espaço deve ter até 80 caracteres.".into());
    }
    if !["/", "/projetos", "/recursos", "/biblioteca", "/lixeira", "/conta", "/configuracoes", "/modelos", "/importar"]
        .contains(&preferences.default_route.as_str())
    {
        return Err("Tela inicial inválida.".into());
    }
    if !["light", "dark", "system"].contains(&preferences.theme.as_str()) {
        return Err("Tema inválido.".into());
    }
    if preferences.locale.is_empty() || preferences.locale.len() > 32 || preferences.locale.chars().any(char::is_control) {
        return Err("Idioma inválido.".into());
    }
    if preferences.locale != "system" && !crate::i18n::languages().iter().any(|language| language.code == preferences.locale) {
        return Err("Este idioma não está disponível nesta versão do aplicativo.".into());
    }
    if !(75..=300).contains(&preferences.ui_scale_percent) {
        return Err("O tamanho da interface deve estar entre 75% e 300%.".into());
    }
    Ok(())
}
fn validate_relative(relative: &str) -> Result<(), String> {
    let path = Path::new(relative);
    let parts: Vec<_> = path.components().collect();
    if parts.len() != 2 || parts.iter().any(|part| !matches!(part, Component::Normal(_))) {
        return Err("Caminho inválido no catálogo local.".into());
    }
    let directory = parts[0].as_os_str().to_str().unwrap_or("");
    let file = parts[1].as_os_str().to_str().unwrap_or("");
    if !["projects", "library", "pending-delete"].contains(&directory)
        || file.is_empty()
        || !file.bytes().all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.'))
        || file.starts_with('.')
    {
        return Err("Arquivo não gerenciado no catálogo local.".into());
    }
    Ok(())
}
fn validate_data(data: &Data) -> Result<(), String> {
    if data.version != 1 {
        return Err("Este espaço foi salvo por uma versão diferente do DesignFlow.".into());
    }
    validate_preferences(&data.preferences)?;
    if !data.profile.name.is_empty() {
        validated_name(&data.profile.name)?;
    }
    for project in &data.projects {
        validate_project(project)?;
    }
    for item in &data.library {
        validate_library_item(item)?;
    }
    for item in &data.trash {
        match &item.original {
            TrashedRecord::Project(project) => {
                validate_project(project)?;
                if item.kind != "project" {
                    return Err("Tipo inválido na lixeira.".into());
                }
            }
            TrashedRecord::Library(file) => {
                validate_library_item(file)?;
                if item.kind != "file" {
                    return Err("Tipo inválido na lixeira.".into());
                }
            }
        }
    }
    let mut ids = BTreeSet::new();
    for (id, title, path) in data
        .projects
        .iter()
        .map(|item| (&item.id, &item.title, &item.path))
        .chain(data.library.iter().map(|item| (&item.id, &item.title, &item.path)))
        .chain(data.trash.iter().map(|item| {
            (
                &item.id,
                &item.title,
                match &item.original {
                    TrashedRecord::Project(original) => &original.path,
                    TrashedRecord::Library(original) => &original.path,
                },
            )
        }))
    {
        if !ids.insert(id) || id.is_empty() || !id.bytes().all(|byte| byte.is_ascii_alphanumeric() || byte == b'-') {
            return Err("Identificador duplicado ou inválido no catálogo local.".into());
        }
        validated_name(title)?;
        validate_relative(path)?;
        if Path::new(path).file_stem().and_then(|stem| stem.to_str()) != Some(id.as_str()) {
            return Err("Caminho e identificador não correspondem no catálogo local.".into());
        }
    }
    for item in &data.trash {
        if item.id != item.original.id() {
            return Err("Item inválido na lixeira.".into());
        }
    }
    Ok(())
}
fn validate_project(project: &Project) -> Result<(), String> {
    validated_name(&project.title)?;
    if !matches!(project.status.as_str(), "draft" | "active" | "archived")
        || Path::new(&project.path).parent() != Some(Path::new("projects"))
        || Path::new(&project.path).extension().and_then(|extension| extension.to_str()) != Some("designflow")
    {
        return Err("Projeto inválido no catálogo local.".into());
    }
    Ok(())
}
fn validate_library_item(item: &LibraryItem) -> Result<(), String> {
    validated_name(&item.title)?;
    if !matches!(item.kind.as_str(), "upload" | "collection" | "logo" | "palette" | "image" | "other")
        || Path::new(&item.path).parent() != Some(Path::new("library"))
    {
        return Err("Arquivo inválido no catálogo da biblioteca.".into());
    }
    Ok(())
}
fn read_capped(path: &Path, limit: u64) -> Result<Vec<u8>, String> {
    let file = File::open(path).map_err(io_error)?;
    read_open_file_capped(file, limit)
}
fn read_open_file_capped(file: File, limit: u64) -> Result<Vec<u8>, String> {
    let metadata = file.metadata().map_err(io_error)?;
    if !metadata.is_file() {
        return Err("Selecione um arquivo regular.".into());
    }
    if metadata.len() > limit {
        return Err(format!("O arquivo excede o limite local de {} MiB.", limit / 1024 / 1024));
    }
    let mut bytes = Vec::new();
    file.take(limit + 1).read_to_end(&mut bytes).map_err(io_error)?;
    if bytes.len() as u64 > limit {
        return Err("O arquivo mudou de tamanho durante a leitura. Tente novamente.".into());
    }
    Ok(bytes)
}
fn write_new(path: &Path, bytes: &[u8]) -> Result<(), String> {
    use std::io::Write;
    let mut file = OpenOptions::new().create_new(true).write(true).open(path).map_err(io_error)?;
    if let Err(error) = file.write_all(bytes).and_then(|()| file.sync_all()) {
        drop(file);
        let _ = fs::remove_file(path);
        return Err(io_error(error));
    }
    Ok(())
}
fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), String> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        designflow_format::write_file_atomic(path, bytes).map_err(io_error)
    }
    #[cfg(target_arch = "wasm32")]
    {
        let _ = (path, bytes);
        Err("O armazenamento local deste aplicativo requer a versão desktop.".into())
    }
}

#[cfg(test)]
mod tests;
