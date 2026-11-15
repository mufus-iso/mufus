//! Parsing of macOS disk metadata. This module does not authorize disk writes.

use std::collections::BTreeSet;
use std::io::Cursor;

/// Extract APFS store parent identifiers from `diskutil info -plist /` output.
///
/// This identifies only root volume dependencies, not the complete system or
/// boot topology or prove the parents are physical disks. Missing or unfamiliar
/// data must not be treated as safe.
///
/// # Errors
///
/// Returns an error for malformed metadata, unsupported topology, or an
/// invalid physical store identifier.
pub fn root_apfs_store_parents(data: &[u8]) -> Result<BTreeSet<String>, &'static str> {
    let value = plist::Value::from_reader(Cursor::new(data)).map_err(|_| "invalid plist")?;
    let info = value.as_dictionary().ok_or("invalid disk information")?;
    if info.get("MountPoint").and_then(plist::Value::as_string) != Some("/")
        || info.get("FilesystemType").and_then(plist::Value::as_string) != Some("apfs")
        || info.get("RAIDMaster").and_then(plist::Value::as_boolean) != Some(false)
        || info.get("RAIDSlice").and_then(plist::Value::as_boolean) != Some(false)
    {
        return Err("unsupported root topology");
    }
    let stores = info
        .get("APFSPhysicalStores")
        .and_then(plist::Value::as_array)
        .filter(|stores| !stores.is_empty())
        .ok_or("missing APFS physical stores")?;
    stores
        .iter()
        .map(|store| {
            let id = store
                .as_dictionary()
                .and_then(|store| store.get("APFSPhysicalStore"))
                .and_then(plist::Value::as_string)
                .ok_or("invalid APFS physical store")?;
            let suffix = id.strip_prefix("disk").ok_or("invalid disk identifier")?;
            let (disk, partition) = suffix.split_once('s').ok_or("invalid disk identifier")?;
            if disk.is_empty()
                || partition.is_empty()
                || !disk.bytes().all(|byte| byte.is_ascii_digit())
                || !partition.bytes().all(|byte| byte.is_ascii_digit())
            {
                return Err("invalid disk identifier");
            }
            Ok(format!("disk{disk}"))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::root_apfs_store_parents;

    fn root(stores: &str) -> String {
        format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
            <plist version="1.0"><dict>
            <key>MountPoint</key><string>/</string>
            <key>FilesystemType</key><string>apfs</string>
            <key>RAIDMaster</key><false/><key>RAIDSlice</key><false/>
            <key>APFSPhysicalStores</key><array>{stores}</array>
            </dict></plist>"#
        )
    }

    #[test]
    fn resolves_all_stores_and_rejects_ambiguous_metadata() {
        let store =
            |id: &str| format!("<dict><key>APFSPhysicalStore</key><string>{id}</string></dict>");
        let valid = root(&format!("{}{}", store("disk0s2"), store("disk12s3")));
        let disks = root_apfs_store_parents(valid.as_bytes());
        assert_eq!(disks, Ok(["disk0".into(), "disk12".into()].into()));
        assert!(root_apfs_store_parents(root("").as_bytes()).is_err());
        assert!(root_apfs_store_parents(root(&store("disk0s2s1")).as_bytes()).is_err());
        assert!(root_apfs_store_parents(root(&store("disk0s2;echo")).as_bytes()).is_err());
        assert!(root_apfs_store_parents(
            valid.replace("<key>RAIDSlice</key><false/>", "").as_bytes()
        )
        .is_err());
        assert!(root_apfs_store_parents(
            valid
                .replace("<string>/</string>", "<string>/Volumes/Other</string>")
                .as_bytes()
        )
        .is_err());
        assert!(root_apfs_store_parents(
            valid
                .replace("<string>apfs</string>", "<string>hfs</string>")
                .as_bytes()
        )
        .is_err());
        assert!(root_apfs_store_parents(valid.replace("<false/>", "<true/>").as_bytes()).is_err());
        assert!(root_apfs_store_parents(b"not a plist").is_err());
    }
}
