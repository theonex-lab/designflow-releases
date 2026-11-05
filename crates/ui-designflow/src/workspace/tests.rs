use super::*;

struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        let directory = std::env::temp_dir().join(format!("designflow-workspace-test-{}", unique_id()));
        fs::create_dir_all(&directory).unwrap();
        Self(directory)
    }
    fn store(&self) -> Store {
        Store::from_directory(self.0.join("workspace")).unwrap()
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        if self.0.parent() == Some(std::env::temp_dir().as_path())
            && self.0.file_name().is_some_and(|name| name.to_string_lossy().starts_with("designflow-workspace-test-"))
        {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
}

#[test]
fn real_blank_native_project_persists_and_reopens() {
    let temporary = Temp::new();
    let mut store = temporary.store();
    assert!(store.data.projects.is_empty());
    let project = store.create_project("Meu projeto", None).unwrap();
    let (document, _) = designflow_format::load_file(&store.project_path(&project.id).unwrap()).unwrap();
    assert_eq!(document.name, "Meu projeto");
    assert_eq!(document.pages.len(), 1);
    assert!(document.nodes().all(|node| node.is_page()));
    let reopened = temporary.store();
    assert_eq!(reopened.data.projects, vec![project]);
}

#[test]
fn library_projects_keep_the_title_without_duplicating_the_source() {
    let temporary = Temp::new();
    let mut store = temporary.store();
    let item = store.add_template_to_library("icon-set").unwrap();
    store.rename_library(&item.id, "Meus ícones").unwrap();
    let source = fs::read(store.library_path(&item.id).unwrap()).unwrap();
    let project = store.create_project_from_library(&item.id).unwrap();
    assert_eq!(project.title, "Meus ícones");
    assert_eq!(store.data.library.len(), 1);
    assert_eq!(source, fs::read(store.library_path(&item.id).unwrap()).unwrap());
    let (document, _) = designflow_format::load_file(&store.project_path(&project.id).unwrap()).unwrap();
    assert_eq!(document.name, "Meus ícones");
    assert!(document.nodes().any(|node| !node.is_page()));
    assert!(store.create_project_from_library("missing").is_err());
    assert_eq!(temporary.store().data.projects.len(), 1);
}

#[test]
fn unsupported_template_does_not_create_a_project() {
    let temporary = Temp::new();
    let mut store = temporary.store();
    assert!(store.create_project("Modelo indisponível", Some("not-a-template")).is_err());
    assert!(store.data.projects.is_empty());
    assert_eq!(fs::read_dir(store.root.join("projects")).unwrap().count(), 0);
    let project = store.create_project("Ícones editáveis", Some("icon-set")).unwrap();
    let (document, _) = designflow_format::load_file(&store.project_path(&project.id).unwrap()).unwrap();
    assert!(document.nodes().any(|node| !node.is_page()));
}

#[test]
fn imports_are_validated_and_original_files_survive_library_deletion() {
    let temporary = Temp::new();
    let source = temporary.0.join("image.png");
    image::RgbaImage::new(2, 3).save(&source).unwrap();
    let original = fs::read(&source).unwrap();
    let mut store = temporary.store();
    let imported = store.import_file(&source).unwrap();
    assert_eq!(store.data.projects.len(), 1);
    assert_eq!(store.data.library.len(), 1);
    let library_path = store.library_path(&imported.library_item.id).unwrap();
    assert_ne!(source, library_path);
    assert_eq!(original, fs::read(&library_path).unwrap());
    assert!(store.project_path(&imported.project.id).unwrap().is_file());
    store.trash_library(&imported.library_item.id).unwrap();
    assert!(store.library_path(&imported.library_item.id).is_err());
    assert!(library_path.is_file());
    store.delete_permanently(&[imported.library_item.id]).unwrap();
    assert!(!library_path.exists());
    assert_eq!(fs::read(&source).unwrap(), original);
    assert!(store.project_path(&imported.project.id).unwrap().is_file());
    let unsupported = temporary.0.join("image.psd");
    fs::write(&unsupported, b"not-a-PSD").unwrap();
    assert!(store.import_file(&unsupported).is_err());
    let corrupt = temporary.0.join("corrupt.png");
    fs::write(&corrupt, b"not-a-PNG").unwrap();
    assert!(store.import_file(&corrupt).is_err());
    assert_eq!(store.data.projects.len(), 1);
}

#[test]
fn favorites_and_recoverable_trash_are_persisted() {
    let temporary = Temp::new();
    let mut store = temporary.store();
    let project = store.create_project("Recuperável", None).unwrap();
    let path = store.project_path(&project.id).unwrap();
    store.favorite_project(&project.id, true).unwrap();
    store.trash_project(&project.id).unwrap();
    assert!(store.data.projects.is_empty());
    assert!(path.is_file());
    assert!(store.project_path(&project.id).is_err());
    let mut reopened = temporary.store();
    reopened.restore(&[project.id.clone()]).unwrap();
    assert!(reopened.data.trash.is_empty());
    assert!(reopened.data.projects[0].favorite);
    reopened.trash_project(&project.id).unwrap();
    reopened.delete_permanently(&[project.id]).unwrap();
    assert!(!path.exists());
    assert!(temporary.store().data.trash.is_empty());
    assert!(reopened.restore(&[]).is_err());
    assert!(reopened.delete_permanently(&[]).is_err());
}

#[test]
fn stale_catalog_writes_fail_without_losing_newer_records_or_leaving_extra_projects() {
    let temporary = Temp::new();
    let mut first = temporary.store();
    let mut stale = temporary.store();
    let project = first.create_project("Primeiro", None).unwrap();
    assert!(stale.create_project("Não deve sobrescrever", None).is_err());
    assert!(stale.data.projects.is_empty());
    assert_eq!(fs::read_dir(first.root.join("projects")).unwrap().count(), 1);
    stale.reload().unwrap();
    assert_eq!(stale.data.projects, vec![project]);
}

#[test]
fn preferences_profile_and_real_catalog_resources_have_no_mock_defaults() {
    let temporary = Temp::new();
    let mut store = temporary.store();
    assert!(store.data.profile.name.is_empty());
    let mut preferences = store.data.preferences.clone();
    preferences.default_route = "/projetos".into();
    store.save_preferences(preferences.clone()).unwrap();
    store.save_profile("Nome escolhido").unwrap();
    let item = store.add_template_to_library("design-system").unwrap();
    let (document, _) = designflow_format::load_file(&store.library_path(&item.id).unwrap()).unwrap();
    assert!(document.nodes().any(|node| !node.is_page()));
    store.favorite_library(&item.id, true).unwrap();
    store.rename_library(&item.id, "Kit pessoal").unwrap();
    let reopened = temporary.store();
    assert_eq!(reopened.data.preferences, preferences);
    assert_eq!(reopened.data.profile.name, "Nome escolhido");
    assert!(reopened.data.library[0].favorite);
    assert_eq!(reopened.data.library[0].title, "Kit pessoal");
    let previous = store.data.clone();
    preferences.theme = "invalid".into();
    assert!(store.save_preferences(preferences).is_err());
    assert_eq!(store.data, previous);
}

#[test]
fn catalog_path_traversal_is_rejected_before_any_managed_action() {
    let temporary = Temp::new();
    let mut store = temporary.store();
    store.create_project("Projeto", None).unwrap();
    let mut corrupt = store.data.clone();
    corrupt.projects[0].path = "../outside.designflow".into();
    fs::write(store.root.join("workspace.json"), serde_json::to_vec(&corrupt).unwrap()).unwrap();
    assert!(store.reload().is_err());
}

#[test]
fn config_location_prioritizes_override_and_keeps_empty_override_isolated() {
    let temporary = Temp::new();
    let executable = temporary.0.join("executable");
    fs::create_dir(&executable).unwrap();
    fs::write(executable.join("portable"), b"").unwrap();
    let platform = temporary.0.join("platform");
    let override_dir = temporary.0.join("isolated");
    assert_eq!(config_directory_from(Some(override_dir.as_os_str().to_owned()), Some(&executable), Some(platform.clone())).unwrap(), override_dir);
    assert!(config_directory_from(Some(std::ffi::OsString::new()), Some(&executable), Some(platform.clone())).is_err());
    assert_eq!(config_directory_from(None, Some(&executable), Some(platform.clone())).unwrap(), executable.join("config"));
    assert_eq!(config_directory_from(None, None, Some(platform.clone())).unwrap(), platform.join("DesignFlow"));
    assert!(config_directory_from(None, None, None).is_err());
}

#[test]
fn account_loading_honors_configuration_override_without_reading_platform_data() {
    const CHILD: &str = "DESIGNFLOW_WORKSPACE_CONFIG_TEST_CHILD";
    if let Ok(mode) = std::env::var(CHILD) {
        if mode == "empty" {
            assert!(Store::load("account@example.invalid").is_err());
        } else {
            let mut store = Store::load("account@example.invalid").unwrap();
            let configured = fs::canonicalize(std::env::var_os("DESIGNFLOW_CONFIG_DIR").unwrap()).unwrap();
            assert!(store.root.starts_with(configured.join("workspaces")));
            store.create_project("Projeto isolado", None).unwrap();
        }
        return;
    }
    let temporary = Temp::new();
    let config = temporary.0.join("config");
    let platform = temporary.0.join("platform");
    fs::create_dir(&platform).unwrap();
    fs::write(platform.join("preservar.txt"), b"existing platform data").unwrap();
    for mode in ["configured", "empty"] {
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .arg("--exact")
            .arg("workspace::tests::account_loading_honors_configuration_override_without_reading_platform_data")
            .arg("--nocapture")
            .env(CHILD, mode)
            .env("DESIGNFLOW_CONFIG_DIR", if mode == "empty" { std::ffi::OsStr::new("") } else { config.as_os_str() })
            .env("APPDATA", &platform)
            .env("XDG_CONFIG_HOME", &platform)
            .output()
            .unwrap();
        assert!(output.status.success(), "{}\n{}", String::from_utf8_lossy(&output.stdout), String::from_utf8_lossy(&output.stderr));
        if mode == "configured" {
            assert!(config.join("workspaces").is_dir(), "the isolated child must actually execute Store::load");
        }
    }
    assert_eq!(fs::read_dir(&platform).unwrap().count(), 1);
    assert_eq!(fs::read(platform.join("preservar.txt")).unwrap(), b"existing platform data");
}

#[test]
fn rejected_catalog_records_and_failed_reads_preserve_existing_files_and_memory() {
    let temporary = Temp::new();
    let mut store = temporary.store();
    let project = store.create_project("Preservado", None).unwrap();
    let path = store.project_path(&project.id).unwrap();
    let original = fs::read(&path).unwrap();
    let previous = store.data.clone();
    let mut invalid = previous.clone();
    invalid.projects[0].path = format!("library/{}.designflow", project.id);
    fs::write(store.root.join("workspace.json"), serde_json::to_vec(&invalid).unwrap()).unwrap();
    assert!(store.reload().is_err());
    assert_eq!(store.data, previous);
    assert_eq!(fs::read(&path).unwrap(), original);
    fs::rename(store.root.join("workspace.json"), store.root.join("catalog-backup.json")).unwrap();
    fs::create_dir(store.root.join("workspace.json")).unwrap();
    assert!(store.reload().is_err());
    assert_eq!(store.data, previous);
    assert_eq!(fs::read(&path).unwrap(), original);
}

#[test]
fn old_preferences_gain_normal_scale_and_invalid_accessibility_choices_do_not_persist() {
    let preferences: Preferences = serde_json::from_str(r#"{"workspace_name":"Meu espaço","theme":"light","locale":"pt-BR"}"#).unwrap();
    assert_eq!(preferences.ui_scale_percent, 100);
    let temporary = Temp::new();
    let mut store = temporary.store();
    let mut enlarged = preferences;
    enlarged.workspace_name = "  Meu espaço acessível  ".into();
    enlarged.ui_scale_percent = 200;
    store.save_preferences(enlarged.clone()).unwrap();
    assert_eq!(store.data.preferences.ui_scale_percent, 200);
    assert_eq!(store.data.preferences.workspace_name, "Meu espaço acessível");
    assert_eq!(temporary.store().data.preferences, store.data.preferences);
    let saved = store.data.clone();
    for percent in [0, 74, 301, u16::MAX] {
        enlarged.ui_scale_percent = percent;
        assert!(store.save_preferences(enlarged.clone()).is_err());
        assert_eq!(store.data, saved);
    }
    enlarged.ui_scale_percent = 100;
    enlarged.locale = "not-a-supported-locale".into();
    assert!(store.save_preferences(enlarged).is_err());
    assert_eq!(temporary.store().data, saved);
}
