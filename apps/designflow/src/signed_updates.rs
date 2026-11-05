//! Fetch and stage signed Windows releases. Staging never modifies the running installation.
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::{self, Read},
    path::{Path, PathBuf},
    process::Command,
    thread::{self, JoinHandle},
    time::Duration,
};

const PUBLIC_KEY: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/update-public.der"));
const RELEASES: &str = "https://github.com/theonex-lab/designflow-releases/releases/latest/download";
const MANIFEST: &str = "designflow-update-windows-x64.json";
const SIGNATURE: &str = "designflow-update-windows-x64.sig";
const HELPER: &str = "designflow-update-helper.exe";

fn installed_exe() -> Result<PathBuf, String> {
    let local = std::env::var_os("LOCALAPPDATA").ok_or("Pasta local indisponível.")?;
    PathBuf::from(local).join("Programs").join("DesignFlow").join("DesignFlow.exe")
        .canonicalize().map_err(|e| e.to_string())
}

pub(crate) fn check_at_start() -> Option<JoinHandle<Result<PathBuf, String>>> {
    if PUBLIC_KEY.is_empty() || std::env::current_exe().ok()?.canonicalize().ok()? != installed_exe().ok()? {
        return None;
    }
    Some(thread::spawn(stage))
}

fn stage() -> Result<PathBuf, String> {
    let agent: ureq::Agent = ureq::Agent::config_builder().timeout_global(Some(Duration::from_secs(180))).build().into();
    let metadata = |name, limit| -> Result<Vec<u8>, String> {
        let mut response = agent.get(format!("{RELEASES}/{name}")).call().map_err(|e| e.to_string())?;
        let mut bytes = Vec::new();
        response.body_mut().as_reader().take(limit).read_to_end(&mut bytes).map_err(|e| e.to_string())?;
        Ok(bytes)
    };
    let manifest = metadata(MANIFEST, 8193)?;
    let signature = metadata(SIGNATURE, 1025)?;
    let installed = env!("CARGO_PKG_VERSION");
    let update = crate::update_verification::verify_manifest(PUBLIC_KEY, &manifest, &signature, installed)?;
    let name = update["file"].as_str().ok_or("Arquivo ausente.")?;
    let size = update["bytes"].as_u64().filter(|size| (1..=512 * 1024 * 1024).contains(size)).ok_or("Tamanho inválido.")?;
    let exe = installed_exe()?;
    let directory = exe.parent().ok_or("Pasta de instalação inválida.")?.join(".designflow-update");
    fs::create_dir_all(&directory).map_err(|e| e.to_string())?;
    let binary = directory.join(name);
    if crate::update_verification::verify(PUBLIC_KEY, &manifest, &signature, &binary, installed).is_ok() {
        fs::write(directory.join(MANIFEST), manifest).map_err(|e| e.to_string())?;
        fs::write(directory.join(SIGNATURE), signature).map_err(|e| e.to_string())?;
        return Ok(directory);
    }
    let mut response = agent.get(format!("{RELEASES}/{name}")).call().map_err(|e| e.to_string())?;
    let temporary = binary.with_extension(format!("{}.part", std::process::id()));
    let result = (|| -> Result<(), String> {
        let mut file = fs::OpenOptions::new().write(true).create_new(true).open(&temporary).map_err(|e| e.to_string())?;
        let download = io::copy(&mut response.body_mut().as_reader().take(size + 1), &mut file);
        download.map_err(|e| e.to_string())?;
        file.sync_all().map_err(|e| e.to_string())?;
        drop(file);
        crate::update_verification::verify_binary(&update, &temporary)?;
        fs::rename(&temporary, &binary).map_err(|e| e.to_string())?;
        fs::write(directory.join(MANIFEST), manifest).map_err(|e| e.to_string())?;
        fs::write(directory.join(SIGNATURE), signature).map_err(|e| e.to_string())
    })();
    if let Err(error) = result {
        let _ = fs::remove_file(&temporary);
        return Err(error);
    }
    Ok(directory)
}

pub(crate) fn apply_after_close(worker: Option<JoinHandle<Result<PathBuf, String>>>) {
    let Some(worker) = worker.filter(|worker| worker.is_finished()) else { return };
    let Ok(Ok(directory)) = worker.join() else { return };
    let result = (|| -> io::Result<()> {
        let helper = directory.join(HELPER);
        fs::copy(std::env::current_exe()?, &helper)?;
        Command::new(helper).arg("--finish-designflow-signed-update").spawn()?;
        Ok(())
    })();
    if let Err(error) = result {
        log::warn!("Não foi possível iniciar a instalação da atualização: {error}");
    }
}

pub(crate) fn finish_helper() -> Option<i32> {
    if std::env::args_os().nth(1).as_deref() != Some(std::ffi::OsStr::new("--finish-designflow-signed-update")) {
        return None;
    }
    Some(match finish() {
        Ok(()) => 0,
        Err(_) => 1,
    })
}

fn finish() -> Result<(), String> {
    let exe = installed_exe()?;
    let directory = exe.parent().ok_or("Pasta de instalação inválida.")?.join(".designflow-update");
    let helper = std::env::current_exe().and_then(|path| path.canonicalize()).map_err(|e| e.to_string())?;
    if helper != directory.join(HELPER) {
        return Err("Auxiliar fora da instalação do DesignFlow.".into());
    }
    let manifest = read_bounded(&directory.join(MANIFEST), 8192)?;
    let signature = read_bounded(&directory.join(SIGNATURE), 1024)?;
    let installed = env!("CARGO_PKG_VERSION");
    let update = crate::update_verification::verify_manifest(PUBLIC_KEY, &manifest, &signature, installed)?;
    let binary = directory.join(update["file"].as_str().ok_or("Arquivo ausente.")?);
    crate::update_verification::verify_binary(&update, &binary)?;
    replace(&binary, &exe, &helper)
}

fn read_bounded(path: &Path, limit: u64) -> Result<Vec<u8>, String> {
    let mut bytes = Vec::new();
    fs::File::open(path).and_then(|file| file.take(limit + 1).read_to_end(&mut bytes)).map_err(|e| e.to_string())?;
    if bytes.len() as u64 > limit {
        return Err("Metadados de atualização excedem o limite.".into());
    }
    Ok(bytes)
}

fn digest(path: &Path) -> io::Result<[u8; 32]> {
    let mut hasher = Sha256::new();
    io::copy(&mut fs::File::open(path)?, &mut hasher)?;
    Ok(hasher.finalize().into())
}

fn replace(binary: &Path, exe: &Path, helper: &Path) -> Result<(), String> {
    // A helper from an earlier run must not overwrite an installation that already changed.
    let previous = digest(helper).map_err(|e| e.to_string())?;
    for attempt in 0..40 {
        if digest(exe).map_err(|e| e.to_string())? != previous {
            return Err("O DesignFlow instalado já foi alterado.".into());
        }
        match fs::rename(binary, exe) {
            Ok(()) => return Ok(()),
            Err(e) if attempt == 39 => return Err(e.to_string()),
            Err(_) => thread::sleep(Duration::from_millis(250)),
        }
    }
    Err("Não foi possível substituir o executável.".into())
}
