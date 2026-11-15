//! Parsing of macOS disk metadata. This module does not authorize disk writes.

use std::collections::BTreeSet;
use std::io::Cursor;

/// Metadata from a whole external physical disk, not an authorization to write.
#[derive(Debug, PartialEq, Eq)]
pub struct ExternalDiskInfo {
    pub identifier: String,
    pub model: String,
    pub transport: String,
    pub size_bytes: u64,
}

/// Parse `diskutil info -plist` for an expected whole disk.
///
/// The result alone does not establish system topology or device identity.
///
/// # Errors
///
/// Returns an error when the metadata is invalid, inconsistent, or does not
/// describe a whole external physical disk.
pub fn external_disk_info(data: &[u8], expected: &str) -> Result<ExternalDiskInfo, &'static str> {
    let number = expected
        .strip_prefix("disk")
        .filter(|number| !number.is_empty() && number.bytes().all(|byte| byte.is_ascii_digit()))
        .ok_or("invalid disk identifier")?;
    if number.len() > 1 && number.starts_with('0') {
        return Err("invalid disk identifier");
    }
    let value = plist::Value::from_reader(Cursor::new(data)).map_err(|_| "invalid plist")?;
    let info = value.as_dictionary().ok_or("invalid disk information")?;
    let string = |key: &str| info.get(key).and_then(plist::Value::as_string);
    let boolean = |key: &str| info.get(key).and_then(plist::Value::as_boolean);
    if string("DeviceIdentifier") != Some(expected)
        || string("ParentWholeDisk") != Some(expected)
        || string("VirtualOrPhysical") != Some("Physical")
        || boolean("WholeDisk") != Some(true)
        || boolean("Internal") != Some(false)
        || boolean("RAIDMaster") != Some(false)
        || boolean("RAIDSlice") != Some(false)
    {
        return Err("unsupported or inconsistent disk metadata");
    }
    let nonempty = |key| {
        string(key)
            .filter(|value| !value.trim().is_empty())
            .map(str::to_owned)
            .ok_or("missing disk metadata")
    };
    let size_bytes = info
        .get("TotalSize")
        .and_then(plist::Value::as_unsigned_integer)
        .filter(|size| *size > 0)
        .ok_or("invalid disk capacity")?;
    Ok(ExternalDiskInfo {
        identifier: expected.to_owned(),
        model: nonempty("MediaName")?,
        transport: nonempty("BusProtocol")?,
        size_bytes,
    })
}

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
                || (disk.len() > 1 && disk.starts_with('0'))
                || (partition.len() > 1 && partition.starts_with('0'))
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
    use super::{external_disk_info, root_apfs_store_parents, ExternalDiskInfo};

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
        assert!(root_apfs_store_parents(root(&store("disk00s2")).as_bytes()).is_err());
        assert!(root_apfs_store_parents(root(&store("disk0s02")).as_bytes()).is_err());
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

    #[test]
    fn parses_external_disks_and_rejects_unsafe_or_inconsistent_metadata() {
        let info = r#"<?xml version="1.0" encoding="UTF-8"?>
        <plist version="1.0"><dict>
        <key>DeviceIdentifier</key><string>disk4</string>
        <key>ParentWholeDisk</key><string>disk4</string>
        <key>VirtualOrPhysical</key><string>Physical</string>
        <key>WholeDisk</key><true/><key>Internal</key><false/>
        <key>RAIDMaster</key><false/><key>RAIDSlice</key><false/>
        <key>MediaName</key><string>Example</string>
        <key>BusProtocol</key><string>USB</string>
        <key>TotalSize</key><integer>30943995904</integer>
        </dict></plist>"#;
        assert_eq!(
            external_disk_info(info.as_bytes(), "disk4"),
            Ok(ExternalDiskInfo {
                identifier: "disk4".into(),
                model: "Example".into(),
                transport: "USB".into(),
                size_bytes: 30_943_995_904,
            })
        );
        assert!(external_disk_info(info.as_bytes(), "disk5").is_err());
        assert!(external_disk_info(info.as_bytes(), "disk04").is_err());
        assert!(external_disk_info(info.as_bytes(), "disk4s1").is_err());
        assert!(external_disk_info(
            info.replace("<key>Internal</key><false/>", "<key>Internal</key><true/>")
                .as_bytes(),
            "disk4"
        )
        .is_err());
        assert!(external_disk_info(
            info.replace("<string>Physical</string>", "<string>Virtual</string>")
                .as_bytes(),
            "disk4"
        )
        .is_err());
        assert!(external_disk_info(
            info.replace("<key>RAIDSlice</key><false/>", "").as_bytes(),
            "disk4"
        )
        .is_err());
        assert!(external_disk_info(info.replace("30943995904", "0").as_bytes(), "disk4").is_err());
        assert!(external_disk_info(b"not a plist", "disk4").is_err());
    }
}
