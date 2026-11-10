// A plugin is signed over all of its files except plugin.sig and hidden files, in byte order of their
// paths: each file adds "path\0length\0" and its contents. plugin.sig holds the Ed25519 signature in hex.

use std::fs;
use std::path::Path;

use ed25519_dalek::{Signature, VerifyingKey};

// The public keys that may sign plugins, in hex.
const TRUSTED: &[&str] = &["45e0558a9771979ea0d4ddb10c1ff8daba4a89067495feb59e02ffbae086155d"];

fn files(root: &Path, dir: &Path, paths: &mut Vec<String>) -> std::io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let path = entry?.path();
        let hidden = path.file_name().is_some_and(|name| name.to_string_lossy().starts_with('.'));
        if hidden {
            continue;
        }
        if path.is_dir() {
            files(root, &path, paths)?;
        } else {
            let relative: Vec<String> = path
                .strip_prefix(root)
                .unwrap()
                .components()
                .map(|part| part.as_os_str().to_string_lossy().into_owned())
                .collect();
            let relative = relative.join("/");
            if relative != "plugin.sig" {
                paths.push(relative);
            }
        }
    }
    Ok(())
}

fn message(dir: &Path) -> std::io::Result<Vec<u8>> {
    let mut paths = Vec::new();
    files(dir, dir, &mut paths)?;
    paths.sort();
    let mut message = Vec::new();
    for path in paths {
        let body = fs::read(dir.join(&path))?;
        message.extend(format!("{path}\0{}\0", body.len()).as_bytes());
        message.extend(body);
    }
    Ok(message)
}

fn hex(text: &str) -> Option<Vec<u8>> {
    if !text.len().is_multiple_of(2) {
        return None;
    }
    (0..text.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(text.get(i..i + 2)?, 16).ok())
        .collect()
}

pub fn verify(dir: &Path) -> bool {
    let signature = fs::read_to_string(dir.join("plugin.sig"))
        .ok()
        .and_then(|text| hex(text.trim()))
        .and_then(|bytes| Signature::from_slice(&bytes).ok());
    let (Some(signature), Ok(message)) = (signature, message(dir)) else {
        return false;
    };
    TRUSTED
        .iter()
        .filter_map(|key| VerifyingKey::from_bytes(&hex(key)?.try_into().ok()?).ok())
        .any(|key| key.verify_strict(&message, &signature).is_ok())
}
