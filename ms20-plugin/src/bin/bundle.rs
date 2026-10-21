use std::fs;
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = manifest_dir
        .parent()
        .expect("ms20-plugin should be inside workspace root");
    let target_release = workspace_root.join("target").join("release");

    #[cfg(target_os = "macos")]
    let dylib_name = "libkalokorg_m20s_plugin.dylib";
    #[cfg(target_os = "linux")]
    let dylib_name = "libkalokorg_m20s_plugin.so";
    #[cfg(target_os = "windows")]
    let dylib_name = "kalokorg_m20s_plugin.dll";

    let src_dylib = target_release.join(dylib_name);
    if !src_dylib.exists() {
        eprintln!(
            "Error: {} not found. Run `cargo build --release -p ms20-plugin` first.",
            src_dylib.display()
        );
        std::process::exit(1);
    }

    let bundled_dir = workspace_root.join("target").join("bundled");
    fs::create_dir_all(&bundled_dir)?;

    // 1. Package macOS .clap bundle: KaloKorg M20s.clap/Contents/MacOS/KaloKorg M20s
    let clap_bundle = bundled_dir.join("KaloKorg M20s.clap");
    let clap_contents = clap_bundle.join("Contents");
    let clap_macos = clap_contents.join("MacOS");
    fs::create_dir_all(&clap_macos)?;
    fs::copy(&src_dylib, clap_macos.join("KaloKorg M20s"))?;
    fs::write(
        clap_contents.join("Info.plist"),
        info_plist_xml("com.kalokorg.m20s.clap", "KaloKorg M20s"),
    )?;
    fs::write(clap_contents.join("PkgInfo"), "BNDL????")?;

    // 2. Package macOS .vst3 bundle: KaloKorg M20s.vst3/Contents/MacOS/KaloKorg M20s
    let vst3_bundle = bundled_dir.join("KaloKorg M20s.vst3");
    let vst3_contents = vst3_bundle.join("Contents");
    let vst3_macos = vst3_contents.join("MacOS");
    fs::create_dir_all(&vst3_macos)?;
    fs::copy(&src_dylib, vst3_macos.join("KaloKorg M20s"))?;
    fs::write(
        vst3_contents.join("Info.plist"),
        info_plist_xml("com.kalokorg.m20s.vst3", "KaloKorg M20s"),
    )?;
    fs::write(vst3_contents.join("PkgInfo"), "BNDL????")?;

    println!("==============================================================");
    println!(" KaloKorg M20s DAW Plugin Bundles Created Successfully:");
    println!("   CLAP : {}", clap_bundle.display());
    println!("   VST3 : {}", vst3_bundle.display());
    println!("==============================================================");
    println!(" To install on macOS, copy either bundle to:");
    println!("   ~/Library/Audio/Plug-Ins/CLAP/");
    println!("   ~/Library/Audio/Plug-Ins/VST3/");
    Ok(())
}

fn info_plist_xml(bundle_id: &str, executable_name: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleDevelopmentRegion</key>
    <string>English</string>
    <key>CFBundleExecutable</key>
    <string>{executable_name}</string>
    <key>CFBundleIdentifier</key>
    <string>{bundle_id}</string>
    <key>CFBundleInfoDictionaryVersion</key>
    <string>6.0</string>
    <key>CFBundleName</key>
    <string>{executable_name}</string>
    <key>CFBundlePackageType</key>
    <string>BNDL</string>
    <key>CFBundleShortVersionString</key>
    <string>0.1.0</string>
    <key>CFBundleSignature</key>
    <string>????</string>
    <key>CFBundleVersion</key>
    <string>0.1.0</string>
    <key>CSResourcesFileMapped</key>
    <true/>
</dict>
</plist>
"#
    )
}
