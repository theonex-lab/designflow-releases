//! Windows: embed the icon and version info in DesignFlow and size its main thread's stack
//! (no-op elsewhere).
fn main() {
    println!("cargo::rerun-if-env-changed=DESIGNFLOW_UPDATE_PUBLIC_KEY");
    let key = std::env::var_os("DESIGNFLOW_UPDATE_PUBLIC_KEY").map(std::path::PathBuf::from);
    let key_bytes = if let Some(key) = key {
        println!("cargo::rerun-if-changed={}", key.display());
        std::fs::read(key).expect("read DesignFlow update public key")
    } else {
        Vec::new()
    };
    let out = std::path::PathBuf::from(std::env::var_os("OUT_DIR").expect("Cargo output directory"));
    std::fs::write(out.join("update-public.der"), key_bytes).expect("write DesignFlow update public key");
    println!("cargo::rustc-check-cfg=cfg(windows_slim_errors)");
    println!("cargo::rerun-if-changed=../../assets/designflow/mark.ico");
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    if target_os == "windows" {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("../../assets/designflow/mark.ico");
        res.set("ProductName", "DesignFlow");
        res.set("FileDescription", "DesignFlow desktop design editor");
        res.set("CompanyName", "DesignFlow");
        if let Err(e) = res.compile() {
            println!("cargo::warning=winresource: {e}");
        }
        // Windows starts the main thread with 1 MB of stack, Linux and macOS with 8 MB. Layers
        // nest up to 256 deep in an export, and that recursion needs the same room everywhere.
        let msvc = std::env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc");
        println!("cargo::rustc-link-arg-bins={}", if msvc { "/STACK:8388608" } else { "-Wl,--stack,8388608" });
    }
    let sha = std::env::var("DESIGNFLOW_BUILD_SHA").unwrap_or_default();
    println!("cargo::rustc-env=DESIGNFLOW_BUILD_SHA={sha}");
    println!("cargo::rerun-if-env-changed=DESIGNFLOW_BUILD_SHA");
}
