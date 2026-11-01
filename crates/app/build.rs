fn main() {
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    if target_os == "windows" {
        println!("cargo:rerun-if-changed=../../packaging/windows/strata.exe.manifest");
        println!("cargo:rerun-if-changed=../../packaging/windows/icon.ico");
        println!("cargo:rerun-if-changed=../../packaging/windows/strata.rc");

        let manifest_dir = std::path::PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
        let rc_path = manifest_dir.join("../../packaging/windows/strata.rc");
        let rc_dir = rc_path.parent().unwrap();

        let _ = embed_resource::compile(&rc_path, embed_resource::ParamsIncludeDirs([rc_dir]));
    }
}
