mod build_support;

use std::{env, fs, path::PathBuf, process::Command};

use build_support::{download, run_command, unzip};
use space_thumbnails_windows::constant::PROVIDERS;
// use space_thumbnails_windows::providers::Provider;
use space_thumbnails_windows::registry::{RegistryData, RegistryKey};

fn clean_id(id: &str) -> String {
    let mut cleaned = id
        .replace("\\", "_")
        .replace("/", "_")
        .replace(".", "_")
        .replace("-", "_")
        .replace(" ", "_");

    if cleaned.len() > 35 {
        // Use a hash for long IDs
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        cleaned.hash(&mut hasher);
        let hash = hasher.finish();
        cleaned.truncate(30);
        cleaned.push_str(&format!("_{:x}", hash));
    }
    cleaned
}

fn walk_dir(
    dir: &std::path::Path,
    root: &std::path::Path,
    wix_dirs: &mut String,
    wix_components: &mut String,
    wix_feature_refs: &mut String,
    dir_id_prefix: &str,
    comp_id_prefix: &str,
) {
    if !dir.exists() {
        return;
    }

    let entries = fs::read_dir(dir).unwrap();
    let mut files = Vec::new();
    let mut subdirs = Vec::new();

    for entry in entries {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.is_dir() {
            subdirs.push(path);
        } else {
            files.push(path);
        }
    }

    let rel_path = dir.strip_prefix(root).unwrap();
    let dir_id = if rel_path.as_os_str().is_empty() {
        "APPLICATIONROOTDIRECTORY".to_string()
    } else {
        format!(
            "{}_{}",
            dir_id_prefix,
            clean_id(rel_path.to_str().unwrap())
        )
    };

    if !files.is_empty() {
        wix_components.push_str(&format!("    <DirectoryRef Id=\"{}\">\n", dir_id));

        for (i, file) in files.iter().enumerate() {
            let rel_path = dir.strip_prefix(root).unwrap();
            let is_control_panel_exe = rel_path.as_os_str().is_empty() && file.file_name().unwrap() == "ControlPanel.exe";
            
            let comp_id = if is_control_panel_exe {
                "C_ControlPanelExe".to_string()
            } else {
                format!(
                    "{}_{}_{}",
                    comp_id_prefix,
                    clean_id(rel_path.to_str().unwrap()),
                    i
                )
            };

            let file_id = if is_control_panel_exe {
                "ControlPanelExe".to_string()
            } else {
                format!("F_{}", comp_id)
            };
            
            let is_exe_like = file.extension().map_or(false, |ext| {
                let ext = ext.to_string_lossy().to_lowercase();
                ext == "exe" || ext == "dll" || ext == "mui" || ext == "sys" || ext == "ocx"
            });
            
            wix_components.push_str(&format!(
                "      <Component Id=\"{}\" Guid=\"*\" Win64=\"yes\">\n",
                comp_id
            ));

            if is_control_panel_exe {
                wix_components.push_str(&format!(
                    "        <File Id=\"{}\" Source=\"{}\" KeyPath=\"yes\" Checksum=\"yes\" Language=\"1033\">\n",
                    file_id,
                    file.to_str().unwrap()
                ));
                wix_components.push_str("          <Shortcut Id=\"startmenuControlPanel\" Directory=\"ApplicationProgramsFolder\" Name=\"Space Thumbnails\" WorkingDirectory=\"APPLICATIONROOTDIRECTORY\" Icon=\"icon.ico\" Advertise=\"yes\" />\n");
                wix_components.push_str("        </File>\n");
            } else {
                let language_attr = if is_exe_like { " Language=\"0\"" } else { "" };
                wix_components.push_str(&format!(
                    "        <File Id=\"{}\" Source=\"{}\" KeyPath=\"yes\"{} />\n",
                    file_id,
                    file.to_str().unwrap(),
                    language_attr
                ));
            }

            wix_components.push_str("      </Component>\n");
            wix_feature_refs.push_str(&format!("      <ComponentRef Id=\"{}\" />\n", comp_id));
        }

        wix_components.push_str("    </DirectoryRef>\n");
    }

    for subdir in subdirs {
        let name = subdir.file_name().unwrap().to_str().unwrap();
        let sub_rel_path = subdir.strip_prefix(root).unwrap();
        let sub_dir_id = format!(
            "{}_{}",
            dir_id_prefix,
            clean_id(sub_rel_path.to_str().unwrap())
        );

        wix_dirs.push_str(&format!("        <Directory Id=\"{}\" Name=\"{}\">\n", sub_dir_id, name));
        
        walk_dir(
            &subdir,
            root,
            wix_dirs,
            wix_components,
            wix_feature_refs,
            dir_id_prefix,
            comp_id_prefix,
        );

        wix_dirs.push_str("        </Directory>\n");
    }
}

fn main() {
    let project_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_owned();

    let assets_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets");
    let out_dir = project_dir.join("target").join("installer");
    let download_dir = out_dir.join("download");
    fs::create_dir_all(download_dir).unwrap();

    let build_dir = out_dir.join("build");
    fs::create_dir_all(&build_dir).unwrap();

    let registy_keys: Vec<RegistryKey> = PROVIDERS.iter().flat_map(|m| m.register("[#MainDLLFile]")).collect();

    let control_panel_dir = project_dir.join("control-panel\\bin\\x64\\Release\\net8.0-windows10.0.19041.0\\publish");

    let version = env!("CARGO_PKG_VERSION");

    let mut wix = String::new();
    wix.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    wix.push_str("<Wix xmlns=\"http://schemas.microsoft.com/wix/2006/wi\" xmlns:util=\"http://schemas.microsoft.com/wix/UtilExtension\">\n");
    wix.push_str(&format!("  <Product Id=\"*\" UpgradeCode=\"1C589985-B4C6-53EC-8483-112D02E6DCD2\" Version=\"{}\" Language=\"1033\" Name=\"Space Thumbnails\" Manufacturer=\"EYHN\">\n", version));
    wix.push_str(
        "    <Package InstallerVersion=\"300\" Compressed=\"yes\" InstallScope=\"perMachine\"/>\n",
    );
    wix.push_str("    <Media Id=\"1\" Cabinet=\"cab1.cab\" EmbedCab=\"yes\" />\n");
    wix.push_str("    <Directory Id=\"TARGETDIR\" Name=\"SourceDir\">\n");
    wix.push_str("      <Directory Id=\"ProgramMenuFolder\">\n");
    wix.push_str("        <Directory Id=\"ApplicationProgramsFolder\" Name=\"Space Thumbnails\"/>\n");
    wix.push_str("      </Directory>\n");
    wix.push_str("      <Directory Id=\"ProgramFiles64Folder\">\n");
    wix.push_str(
        "        <Directory Id=\"APPLICATIONROOTDIRECTORY\" Name=\"Space Thumbnails\"/>\n",
    );
    wix.push_str("      </Directory>\n");
    wix.push_str("    </Directory>\n");
    
    let mut cp_dirs = String::new();
    let mut cp_comps = String::new();
    let mut cp_feature_refs = String::new();
    walk_dir(
        &control_panel_dir,
        &control_panel_dir,
        &mut cp_dirs,
        &mut cp_comps,
        &mut cp_feature_refs,
        "D",
        "C",
    );

    wix.push_str("    <DirectoryRef Id=\"APPLICATIONROOTDIRECTORY\">\n");
    wix.push_str(&cp_dirs);
    wix.push_str(
        "      <Component Id=\"MainApplication\" Guid=\"9cfa17d1-9a2a-40aa-ba6f-57a2adbdc8dc\" Win64=\"yes\">\n",
    );
    wix.push_str(&format!(
        "        <File Id=\"MainDLLFile\" Source=\"{}\" KeyPath=\"yes\" Checksum=\"yes\" Language=\"0\"/>\n",
        project_dir
            .join("target\\release\\space_thumbnails_windows.dll")
            .to_str()
            .unwrap()
    ));
    wix.push_str(&format!(
        "        <File Id=\"LicenceFile\" Source=\"{}\" Checksum=\"yes\"/>\n",
        assets_dir.join("Licence.rtf").to_str().unwrap()
    ));
    wix.push_str(&format!(
        "        <File Id=\"ReadmeFile\" Source=\"{}\" Checksum=\"yes\"/>\n",
        project_dir.join("README.md").to_str().unwrap()
    ));
    wix.push_str("        <util:EventSource EventMessageFile=\"[#MainDLLFile]\" Log=\"Application\" Name=\"Space Thumbnails\"/>\n");

    for key in registy_keys {
        for val in &key.values {
            let (val_type, val_data) = match &val.1 {
                RegistryData::Str(data) => ("string", data.clone()),
                RegistryData::U32(data) => ("integer", data.to_string()),
            };

            wix.push_str(&format!(
                "        <RegistryValue Root=\"HKCR\" Key=\"{}\" {} Type=\"{}\" Value=\"{}\" />\n",
                key.path,
                if val.0.is_empty() { "".to_string() } else { format!("Name=\"{}\"", val.0) },
                val_type,
                val_data,
            ));
        }
    }
    wix.push_str("      </Component>\n");
    wix.push_str("    </DirectoryRef>\n");
    
    wix.push_str("    <DirectoryRef Id=\"ApplicationProgramsFolder\">\n");
    wix.push_str("      <Component Id=\"CleanupShortcutComp\" Guid=\"*\" Win64=\"yes\">\n");
    wix.push_str("        <RegistryValue Root=\"HKMU\" Key=\"Software\\EYHN\\SpaceThumbnails\" Name=\"shortcut\" Type=\"integer\" Value=\"1\" KeyPath=\"yes\" />\n");
    wix.push_str("        <RemoveFolder Id=\"CleanupShortcut\" On=\"uninstall\" />\n");
    wix.push_str("      </Component>\n");
    wix.push_str("    </DirectoryRef>\n");

    wix.push_str(&cp_comps);

    wix.push_str("    <Feature Id=\"MainApplication\" Title=\"Space Thumbnails\" Level=\"1\">\n");
    wix.push_str("      <ComponentRef Id=\"MainApplication\" />\n");
    wix.push_str("      <ComponentRef Id=\"CleanupShortcutComp\" />\n");
    wix.push_str(&cp_feature_refs);
    wix.push_str("    </Feature>\n");
    wix.push_str("    <UIRef Id=\"WixUI_Minimal\" />\n");
    wix.push_str("    <UIRef Id=\"WixUI_ErrorProgressText\" />\n");
    wix.push_str(&format!(
        "    <Icon Id=\"icon.ico\" SourceFile=\"{}\"/>\n",
        assets_dir.join("icon.ico").to_str().unwrap()
    ));
    wix.push_str("    <Property Id=\"ARPPRODUCTICON\" Value=\"icon.ico\" />\n");
    wix.push_str("    <Property Id=\"MSIINSTALLPERUSER\" Value=\"1\" />\n");
    wix.push_str(&format!(
        "    <WixVariable Id=\"WixUIDialogBmp\" Value=\"{}\" />\n",
        assets_dir.join("UIDialog.bmp").to_str().unwrap()
    ));
    wix.push_str(&format!(
        "    <WixVariable Id=\"WixUIBannerBmp\" Value=\"{}\" />\n",
        assets_dir.join("UIBanner.bmp").to_str().unwrap()
    ));
    wix.push_str(&format!(
        "    <WixVariable Id=\"WixUILicenseRtf\" Value=\"{}\" />\n",
        assets_dir.join("Licence.rtf").to_str().unwrap()
    ));
    wix.push_str("    <MajorUpgrade AllowDowngrades=\"no\" AllowSameVersionUpgrades=\"no\" DowngradeErrorMessage=\"A newer version of [ProductName] is already installed.  If you are sure you want to downgrade, remove the existing installation via the Control Panel\" />\n");
    wix.push_str("  </Product>\n");
    wix.push_str("</Wix>\n");

    let installerwxs = build_dir.join("installer.wxs");

    fs::write(&installerwxs, wix).unwrap();

    let wixzip = download(
        out_dir.join("download").join("wix311-binaries.zip"),
        "https://github.com/wixtoolset/wix3/releases/download/wix3112rtm/wix311-binaries.zip",
    )
    .unwrap();

    let wixdir = unzip(&wixzip, out_dir.join("wix")).unwrap();

    let mut candle_command = Command::new(wixdir.join("candle.exe"));
    candle_command
        .current_dir(&build_dir)
        .arg(installerwxs.to_str().unwrap())
        .args(["-arch", "x64"])
        .args(["-ext", "WixUtilExtension"]);

    run_command(&mut candle_command, "candle.exe");

    let mut light_command = Command::new(wixdir.join("light.exe"));
    light_command
        .current_dir(&build_dir)
        .arg(build_dir.join("installer.wixobj"))
        .args(["-ext", "WixUIExtension"])
        .args(["-ext", "WixUtilExtension"]);

    run_command(&mut light_command, "light.exe");

    fs::copy(
        build_dir.join("installer.msi"),
        out_dir.join("space-thumbnails-installer.msi"),
    )
    .unwrap();
}
