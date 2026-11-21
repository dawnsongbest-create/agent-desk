use std::{
    fs::OpenOptions,
    io::{self, Write},
    path::Path,
};

use flate2::{write::DeflateEncoder, Compression};
use serde_json::json;
use tauri_plugin_dialog::DialogExt;

const MAX_EXECUTABLE: u64 = 96 * 1024 * 1024;

pub(crate) async fn export_package(
    app: tauri::AppHandle,
    port: u16,
    profile: String,
) -> Result<bool, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let selected = app
            .dialog()
            .file()
            .set_title("保存 ChatGPT 本地安装包")
            .set_file_name("agent-desk-chatgpt.zip")
            .add_filter("ZIP", &["zip"])
            .blocking_save_file();
        let Some(selected) = selected else {
            return Ok(false);
        };
        let path = selected
            .into_path()
            .map_err(|_| "请选择本机 ZIP 文件位置。")?;
        if !path
            .extension()
            .and_then(|ext| ext.to_str())
            .is_some_and(|ext| ext.eq_ignore_ascii_case("zip"))
        {
            return Err("请使用 .zip 文件名。".to_owned());
        }
        let executable = std::env::current_exe().map_err(|_| "找不到当前 Agent Desk 程序。")?;
        let bytes = read_program(&executable).map_err(|_| "无法读取当前 Agent Desk 程序。")?;
        if contains_private_build_path(&bytes) {
            return Err(
                "这个构建包含开发路径，不能导出安装包。请使用正式构建的 Agent Desk。".into(),
            );
        }
        let zip = package(port, &profile, &bytes, cfg!(target_os = "windows"))
            .map_err(|_| "无法生成 ChatGPT 安装包。")?;
        write_new(&path, &zip).map_err(|e| {
            if e.kind() == io::ErrorKind::AlreadyExists {
                "该文件已存在，请选另一个文件名。"
            } else {
                "无法保存安装包。"
            }
        })?;
        Ok(true)
    })
    .await
    .map_err(|_| "无法导出 ChatGPT 安装包。".to_owned())?
}

fn read_program(path: &Path) -> io::Result<Vec<u8>> {
    use std::io::Read;
    let mut program = Vec::new();
    std::fs::File::open(path)?
        .take(MAX_EXECUTABLE + 1)
        .read_to_end(&mut program)?;
    if program.is_empty() || program.len() as u64 > MAX_EXECUTABLE {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "program size"));
    }
    Ok(program)
}

fn contains_private_build_path(bytes: &[u8]) -> bool {
    [
        b"/sresU/".as_slice(),
        b"\\sresU\\",
        b"/emoh/",
        b"\\sgnitteS dna stnemucoD\\",
        b"/sredlof/rav/etavirp/",
    ]
    .iter()
    .any(|reversed| {
        // Do not place a forward search needle in the executable being scanned.
        let marker: Vec<_> = std::hint::black_box(*reversed)
            .iter()
            .rev()
            .copied()
            .collect();
        let wide: Vec<_> = marker.iter().flat_map(|byte| [*byte, 0]).collect();
        [marker.as_slice(), wide.as_slice()].iter().any(|pattern| {
            bytes
                .windows(pattern.len())
                .any(|window| window.eq_ignore_ascii_case(pattern))
        })
    })
}

fn write_new(path: &Path, zip: &[u8]) -> io::Result<()> {
    let mut file = OpenOptions::new().write(true).create_new(true).open(path)?;
    let result = file.write_all(zip).and_then(|()| file.sync_all());
    if result.is_err() {
        drop(file);
        let _ = std::fs::remove_file(path);
    }
    result
}

fn package(port: u16, profile: &str, program: &[u8], windows: bool) -> io::Result<Vec<u8>> {
    if port == 0 || !super::valid_profile(profile) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "invalid connector target",
        ));
    }
    let program_path = if windows {
        "server/agent-desk-chatgpt.exe"
    } else {
        "server/agent-desk-chatgpt"
    };
    let manifest = serde_json::to_vec_pretty(&json!({
        "$schema": "https://agent-plugins.org/schemas/1.0.0/plugin.schema.json",
        "name": "agent-desk-chatgpt",
        "version": env!("CARGO_PKG_VERSION"),
        "description": "Send reading and pending task suggestions to Agent Desk on this computer.",
        "extensions": {"com.openai": {"requires_local_executor": true, "interface": {
            "displayName": "Agent Desk for ChatGPT",
            "shortDescription": "Reading and task suggestions",
            "longDescription": "Local only. Submits reading or suggestions you ask to send; cannot read Agent Desk data or create tasks without your confirmation.",
            "developerName": "Agent Desk"
        }}}
    }))?;
    let mcp = serde_json::to_vec_pretty(&json!({
        "$schema": "https://agent-plugins.org/schemas/1.0.0/mcp.schema.json",
        "mcpServers": {"agent-desk-chatgpt": {
            "type": "stdio",
            // The portable importer accepts a bare executable or a contained
            // ./ path here. Placeholder expansion is not done during validation.
            "command": format!("./{program_path}"),
            "args": ["--chatgpt-mcp", &port.to_string(), profile]
        }}
    }))?;
    archive([
        ("plugin.json", manifest.as_slice(), 0o100644),
        ("mcp.json", mcp.as_slice(), 0o100644),
        (program_path, program, 0o100755),
    ])
}

// Classic ZIP with only the three fixed entries above (no links, absolute
// paths or metadata copied from the filesystem). Unix mode permits stdio
// execution on a host that preserves it when unpacking on macOS.
fn archive<const N: usize>(entries: [(&str, &[u8], u32); N]) -> io::Result<Vec<u8>> {
    const FLAGS: u16 = 1 << 11;
    const DATE: u16 = 33; // 1980-01-01
    let mut out = Vec::new();
    let mut central = Vec::new();
    for (name, contents, mode) in entries {
        let mut encoder = DeflateEncoder::new(Vec::new(), Compression::fast());
        encoder.write_all(contents)?;
        let compressed = encoder.finish()?;
        let crc = crc32fast::hash(contents);
        let offset = out.len() as u32;
        le32(&mut out, 0x04034b50);
        fields(&mut out, &[20, FLAGS, 8, 0, DATE]);
        for field in [crc, compressed.len() as u32, contents.len() as u32] {
            le32(&mut out, field);
        }
        fields(&mut out, &[name.len() as u16, 0]);
        out.extend_from_slice(name.as_bytes());
        out.extend_from_slice(&compressed);

        le32(&mut central, 0x02014b50);
        fields(&mut central, &[0x0314, 20, FLAGS, 8, 0, DATE]);
        for field in [crc, compressed.len() as u32, contents.len() as u32] {
            le32(&mut central, field);
        }
        fields(&mut central, &[name.len() as u16, 0, 0, 0, 0]);
        le32(&mut central, mode << 16);
        le32(&mut central, offset);
        central.extend_from_slice(name.as_bytes());
    }
    let offset = out.len() as u32;
    out.extend_from_slice(&central);
    le32(&mut out, 0x06054b50);
    fields(&mut out, &[0, 0, N as u16, N as u16]);
    le32(&mut out, central.len() as u32);
    le32(&mut out, offset);
    fields(&mut out, &[0]);
    Ok(out)
}

fn le32(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_le_bytes());
}
fn fields(out: &mut Vec<u8>, values: &[u16]) {
    for value in values {
        out.extend_from_slice(&value.to_le_bytes());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use flate2::read::DeflateDecoder;
    use std::io::Read;

    #[test]
    fn package_has_only_relative_manifest_config_and_program() {
        for windows in [true, false] {
            let zip = package(47881, "desktop-v1", b"synthetic executable", windows).unwrap();
            let mut cursor = zip.as_slice();
            let mut entries = Vec::new();
            while cursor.starts_with(b"PK\x03\x04") {
                let size = u32::from_le_bytes(cursor[18..22].try_into().unwrap()) as usize;
                let name_len = u16::from_le_bytes(cursor[26..28].try_into().unwrap()) as usize;
                let start = 30 + name_len;
                let name = std::str::from_utf8(&cursor[30..start]).unwrap();
                let mut contents = Vec::new();
                DeflateDecoder::new(&cursor[start..start + size])
                    .read_to_end(&mut contents)
                    .unwrap();
                entries.push((name, contents));
                cursor = &cursor[start + size..];
            }
            assert!(cursor.starts_with(b"PK\x01\x02"));
            assert_eq!(entries.len(), 3);
            assert_eq!(entries[0].0, "plugin.json");
            assert_eq!(entries[1].0, "mcp.json");
            assert_eq!(entries[2].1, b"synthetic executable");
            let config: serde_json::Value = serde_json::from_slice(&entries[1].1).unwrap();
            let server = &config["mcpServers"]["agent-desk-chatgpt"];
            let command = server["command"].as_str().unwrap();
            let member = command
                .strip_prefix("./")
                .expect("portable executable path");
            assert_eq!(member, entries[2].0);
            assert!(member
                .split('/')
                .all(|part| !part.is_empty() && !matches!(part, "." | "..")));
            assert_eq!(
                server["args"],
                json!(["--chatgpt-mcp", "47881", "desktop-v1"])
            );
            assert!(server.get("env").is_none());
            assert!(server.get("cwd").is_none());
            let manifest: serde_json::Value = serde_json::from_slice(&entries[0].1).unwrap();
            let subtitle = manifest["extensions"]["com.openai"]["interface"]["shortDescription"]
                .as_str()
                .unwrap();
            assert!(subtitle.chars().count() <= 30);
        }
        assert!(package(0, "desktop-v1", b"x", true).is_err());
        assert!(package(47881, "invalid profile", b"x", true).is_err());
        assert!(contains_private_build_path(
            br"C:\Users\Example\dependency.rs"
        ));
        assert!(contains_private_build_path(b"/home/synthetic/lib.rs"));
        assert!(!contains_private_build_path(
            b"/agent-desk-cargo/synthetic/lib.rs"
        ));
    }

    #[test]
    #[cfg(target_os = "windows")]
    #[ignore = "writes an isolated acceptance ZIP to an explicit local destination"]
    fn export_isolated_portable_zip() {
        let audit = crate::audit_data_dir().expect("isolated AppData required");
        assert!(audit.is_dir());
        let profile = crate::chatgpt::profile();
        assert!(profile.starts_with("audit-"));
        let port = std::env::var("AGENT_DESK_BRIDGE_PORT")
            .unwrap()
            .parse()
            .unwrap();
        let executable = std::env::var_os("AGENT_DESK_M5_C_ISOLATED_EXE").unwrap();
        let destination = std::env::var_os("AGENT_DESK_M5_C_ISOLATED_ZIP").unwrap();
        let program = read_program(Path::new(&executable)).unwrap();
        assert!(!contains_private_build_path(&program));
        let zip = package(port, &profile, &program, true).unwrap();
        write_new(Path::new(&destination), &zip).unwrap();
    }
}
