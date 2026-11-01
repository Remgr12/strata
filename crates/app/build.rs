use std::path::PathBuf;
use std::process::Command;

fn find_mingw_static_lib(compiler: &str, lib_name: &str) -> Option<PathBuf> {
    let output = Command::new(compiler)
        .arg(format!("-print-file-name={}", lib_name))
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let path_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let path = PathBuf::from(&path_str);
    if path.exists() && path.is_file() {
        Some(path)
    } else {
        None
    }
}

fn setup_windows_gnu_static_libs() {
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    let static_dir = out_dir.join("mingw-static-libs");
    let _ = std::fs::create_dir_all(&static_dir);

    let compiler = std::env::var("CXX_x86_64_pc_windows_gnu")
        .or_else(|_| std::env::var("CC_x86_64_pc_windows_gnu"))
        .unwrap_or_else(|_| "x86_64-w64-mingw32-g++-posix".to_string());

    if let Some(stdcpp_a) = find_mingw_static_lib(&compiler, "libstdc++.a")
        .or_else(|| find_mingw_static_lib("x86_64-w64-mingw32-g++", "libstdc++.a"))
    {
        for name in ["libstdc++.dll.a", "libstdc++.a"] {
            let dest = static_dir.join(name);
            let _ = std::fs::remove_file(&dest);
            let _ = std::fs::copy(&stdcpp_a, &dest);
        }
    }

    if let Some(winpthread_a) = find_mingw_static_lib(&compiler, "libwinpthread.a")
        .or_else(|| find_mingw_static_lib("x86_64-w64-mingw32-gcc", "libwinpthread.a"))
    {
        for name in [
            "libwinpthread.dll.a",
            "libwinpthread.a",
            "libpthread.dll.a",
            "libpthread.a",
        ] {
            let dest = static_dir.join(name);
            let _ = std::fs::remove_file(&dest);
            let _ = std::fs::copy(&winpthread_a, &dest);
        }
    }

    // Override libgcc_s.a (which normally imports libgcc_s_seh-1.dll) with a linker script
    // that pulls in static libgcc_eh, libgcc, and libwinpthread instead.
    let gcc_s_script = static_dir.join("libgcc_s.a");
    let _ = std::fs::write(&gcc_s_script, "INPUT(-lgcc_eh -lgcc -lwinpthread)\n");

    println!("cargo:rustc-link-search=native={}", static_dir.display());
    println!("cargo:rustc-link-arg=-static-libgcc");
    println!("cargo:rustc-link-arg=-static-libstdc++");
    println!("cargo:rustc-link-arg=-Wl,-Bstatic");
    println!("cargo:rustc-link-arg=-lstdc++");
    println!("cargo:rustc-link-arg=-lwinpthread");
    println!("cargo:rustc-link-arg=-lgcc_eh");
    println!("cargo:rustc-link-arg=-lgcc");
    println!("cargo:rustc-link-arg=-Wl,-Bdynamic");
}

fn main() {
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    let target_env = std::env::var("CARGO_CFG_TARGET_ENV").unwrap_or_default();

    if target_os == "windows" {
        println!("cargo:rerun-if-changed=../../packaging/windows/strata.exe.manifest");
        println!("cargo:rerun-if-changed=../../packaging/windows/icon.ico");
        println!("cargo:rerun-if-changed=../../packaging/windows/strata.rc");

        let manifest_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
        let rc_path = manifest_dir.join("../../packaging/windows/strata.rc");
        let rc_dir = rc_path.parent().unwrap();

        let _ = embed_resource::compile(&rc_path, embed_resource::ParamsIncludeDirs([rc_dir]));

        if target_env == "gnu" {
            setup_windows_gnu_static_libs();
        }
    }
}
