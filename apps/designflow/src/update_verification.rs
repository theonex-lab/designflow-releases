//! Verification of DesignFlow's signed Windows update manifests.
use ring::signature::{RSA_PSS_2048_8192_SHA256, UnparsedPublicKey};
use sha2::{Digest, Sha256};
use std::{fs::File, io::Read, path::Path};

pub(crate) fn verify(
    key: &[u8],
    manifest: &[u8],
    signature: &[u8],
    binary: &Path,
    installed: &str,
) -> Result<(), String> {
    let update = verify_manifest(key, manifest, signature, installed)?;
    let expected_name = update["file"].as_str().ok_or("Arquivo ausente.")?;
    if binary.file_name().and_then(|name| name.to_str()) != Some(expected_name) {
        return Err("Nome do executável inválido.".into());
    }
    verify_binary(&update, binary)
}

pub(crate) fn verify_manifest(key: &[u8], manifest: &[u8], signature: &[u8], installed: &str) -> Result<serde_json::Value, String> {
    if manifest.len() > 8192 || signature.len() > 1024 {
        return Err("Metadados da atualização excedem o limite.".into());
    }
    UnparsedPublicKey::new(&RSA_PSS_2048_8192_SHA256, key)
        .verify(manifest, signature)
        .map_err(|_| "Assinatura da atualização inválida.")?;
    let update: serde_json::Value = serde_json::from_slice(manifest).map_err(|e| e.to_string())?;
    let version = update["version"].as_str().ok_or("Versão ausente.")?;
    let version_parts = parse_version(version).ok_or("Versão inválida.")?;
    let current = parse_version(installed).ok_or("Versão instalada inválida.")?;
    let expected_name = format!("designflow-{version}-windows-x64.exe");
    if update["schema"].as_u64() != Some(1)
        || update["app"].as_str() != Some("designflow")
        || update["platform"].as_str() != Some("windows-x64")
        || version_parts <= current
        || update["file"].as_str() != Some(expected_name.as_str())
    {
        return Err("Atualização incompatível ou versão já instalada.".into());
    }
    Ok(update)
}

pub(crate) fn verify_binary(update: &serde_json::Value, binary: &Path) -> Result<(), String> {
    let mut file = File::open(binary).map_err(|e| e.to_string())?;
    let size = file.metadata().map_err(|e| e.to_string())?.len();
    if size == 0 || size > 512 * 1024 * 1024 || update["bytes"].as_u64() != Some(size) {
        return Err("Tamanho do executável inválido.".into());
    }
    let mut digest = Sha256::new();
    let mut buffer = [0; 65536];
    loop {
        let count = file.read(&mut buffer).map_err(|e| e.to_string())?;
        if count == 0 {
            break;
        }
        digest.update(&buffer[..count]);
    }
    if update["sha256"].as_str() != Some(format!("{:x}", digest.finalize()).as_str()) {
        return Err("O executável não corresponde ao manifesto assinado.".into());
    }
    Ok(())
}

fn parse_version(value: &str) -> Option<(u32, u32, u32)> {
    let mut parts = value.split('.');
    let mut next = || {
        let part = parts.next()?;
        if part.is_empty() || !part.bytes().all(|b| b.is_ascii_digit()) {
            return None;
        }
        part.parse().ok()
    };
    let version = (next()?, next()?, next()?);
    parts.next().is_none().then_some(version)
}

#[cfg(test)]
mod tests {
    use super::verify;
    use std::fs;
    use std::path::Path;

    #[test]
    #[ignore = "requires a local signed fixture in DESIGNFLOW_UPDATE_TEST_DIR"]
    fn verifies_signed_release_and_rejects_tampering() {
        let root = std::env::var("DESIGNFLOW_UPDATE_TEST_DIR").unwrap();
        let root = Path::new(&root);
        let key = fs::read(root.join("public.der")).unwrap();
        let mut manifest = fs::read(root.join("designflow-update-windows-x64.json")).unwrap();
        let signature = fs::read(root.join("designflow-update-windows-x64.sig")).unwrap();
        let binary = root.join("designflow-0.13.2-windows-x64.exe");
        assert!(verify(&key, &manifest, &signature, &binary, "0.13.1").is_ok());
        assert!(verify(&key, &manifest, &signature, &binary, "0.13.2").is_err());
        manifest[0] ^= 1;
        assert!(verify(&key, &manifest, &signature, &binary, "0.13.1").is_err());
    }
}
