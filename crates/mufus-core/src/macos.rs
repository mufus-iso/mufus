//! Parsing of macOS disk metadata. This module does not authorize disk writes.

use std::collections::{BTreeMap, BTreeSet};
use std::io::Cursor;

fn valid_whole_disk_id(id: &str) -> bool {
    id.strip_prefix("disk").is_some_and(|number| {
        !(number.is_empty() || number.len() > 1 && number.starts_with('0'))
            && number.bytes().all(|byte| byte.is_ascii_digit())
    })
}

fn store_parent(id: &str) -> Result<String, &'static str> {
    let suffix = id.strip_prefix("disk").ok_or("invalid disk identifier")?;
    let (number, partition) = suffix.split_once('s').ok_or("invalid disk identifier")?;
    let disk = format!("disk{number}");
    if !valid_whole_disk_id(&disk)
        || partition.is_empty()
        || (partition.len() > 1 && partition.starts_with('0'))
        || !partition.bytes().all(|byte| byte.is_ascii_digit())
    {
        return Err("invalid disk identifier");
    }
    Ok(disk)
}

/// Extract the parent identifiers of every APFS physical store from
/// `diskutil apfs list -plist`. This conservatively includes non-system stores.
///
/// These identifiers do not establish that a device is physical, or that a
/// device absent from this list is safe. System and boot topology still require
/// independent verification.
///
/// # Errors
///
/// Returns an error for malformed containers or ambiguous store identifiers.
pub fn apfs_store_parents(data: &[u8]) -> Result<BTreeSet<String>, &'static str> {
    let value = plist::Value::from_reader(Cursor::new(data)).map_err(|_| "invalid plist")?;
    let containers = value
        .as_dictionary()
        .and_then(|list| list.get("Containers"))
        .and_then(plist::Value::as_array)
        .ok_or("missing APFS containers")?;
    let mut parents = BTreeSet::new();
    for container in containers {
        let stores = container
            .as_dictionary()
            .and_then(|container| container.get("PhysicalStores"))
            .and_then(plist::Value::as_array)
            .filter(|stores| !stores.is_empty())
            .ok_or("missing APFS physical stores")?;
        for store in stores {
            let identifier = store
                .as_dictionary()
                .and_then(|store| store.get("DeviceIdentifier"))
                .and_then(plist::Value::as_string)
                .ok_or("invalid APFS physical store")?;
            parents.insert(store_parent(identifier)?);
        }
    }
    Ok(parents)
}

/// Parse whole disk identifiers from `diskutil list -plist external physical`.
///
/// The list is an untrusted snapshot, not proof that a disk is safe to use.
///
/// # Errors
///
/// Returns an error for invalid, duplicate, or inconsistent disk identifiers.
pub fn external_whole_disk_ids(data: &[u8]) -> Result<BTreeSet<String>, &'static str> {
    let value = plist::Value::from_reader(Cursor::new(data)).map_err(|_| "invalid plist")?;
    let list = value.as_dictionary().ok_or("invalid disk list")?;
    let disks = list
        .get("WholeDisks")
        .and_then(plist::Value::as_array)
        .ok_or("missing whole disk list")?;
    let entries = list
        .get("AllDisksAndPartitions")
        .and_then(plist::Value::as_array)
        .ok_or("missing disk entries")?;
    let mut identifiers = BTreeSet::new();
    for disk in disks {
        let id = disk.as_string().ok_or("invalid disk identifier")?;
        if !valid_whole_disk_id(id) || !identifiers.insert(id.to_owned()) {
            return Err("invalid or duplicate disk identifier");
        }
    }
    let mut entry_ids = BTreeSet::new();
    for entry in entries {
        let id = entry
            .as_dictionary()
            .and_then(|entry| entry.get("DeviceIdentifier"))
            .and_then(plist::Value::as_string)
            .ok_or("invalid disk entry")?;
        if !valid_whole_disk_id(id) || !entry_ids.insert(id.to_owned()) {
            return Err("invalid or duplicate disk entry");
        }
    }
    if identifiers != entry_ids {
        return Err("inconsistent disk list");
    }
    Ok(identifiers)
}

/// Metadata from a whole external physical disk, not an authorization to write.
#[derive(Debug, PartialEq, Eq)]
pub struct ExternalDiskInfo {
    pub identifier: String,
    pub model: String,
    pub transport: String,
    pub size_bytes: u64,
}

/// Screen an external disk snapshot against known APFS root dependencies.
///
/// This is only preliminary screening: matching snapshots do not prove device
/// identity or complete boot topology, and the result must not authorize
/// device selection or writes.
///
/// # Errors
///
/// Returns an error if the snapshot is inconsistent or required data is absent.
pub fn screen_external_snapshot(
    initial_list: &[u8],
    final_list: &[u8],
    root_info: &[u8],
    apfs_list: &[u8],
    disk_info: &BTreeMap<String, Vec<u8>>,
) -> Result<Vec<ExternalDiskInfo>, &'static str> {
    let initial = external_whole_disk_ids(initial_list)?;
    if initial != external_whole_disk_ids(final_list)? {
        return Err("external disk list changed");
    }
    let root = root_apfs_store_parents(root_info)?;
    let protected = apfs_store_parents(apfs_list)?;
    if !root.is_subset(&protected) {
        return Err("inconsistent APFS root topology");
    }
    if disk_info.len() != initial.len() || disk_info.keys().any(|id| !initial.contains(id)) {
        return Err("incomplete disk metadata");
    }
    let mut candidates = Vec::new();
    for id in &initial {
        let info = disk_info.get(id).ok_or("missing disk metadata")?;
        let disk = external_disk_info(info, id)?;
        if !protected.contains(id) {
            candidates.push(disk);
        }
    }
    Ok(candidates)
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
    if !valid_whole_disk_id(expected) {
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
            store_parent(id)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{
        apfs_store_parents, external_disk_info, external_whole_disk_ids, root_apfs_store_parents,
        screen_external_snapshot, ExternalDiskInfo,
    };
    use std::collections::{BTreeMap, BTreeSet};

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
    fn screens_known_apfs_dependencies_and_rejects_changed_snapshots() {
        let list = br#"<?xml version="1.0"?><plist version="1.0"><dict>
            <key>WholeDisks</key><array><string>disk4</string><string>disk5</string></array>
            <key>AllDisksAndPartitions</key><array>
            <dict><key>DeviceIdentifier</key><string>disk4</string></dict>
            <dict><key>DeviceIdentifier</key><string>disk5</string></dict>
            </array></dict></plist>"#;
        let root_info = root("<dict><key>APFSPhysicalStore</key><string>disk4s1</string></dict>");
        let apfs = br#"<?xml version="1.0"?><plist version="1.0"><dict><key>Containers</key><array>
            <dict><key>PhysicalStores</key><array>
            <dict><key>DeviceIdentifier</key><string>disk4s1</string></dict>
            </array></dict></array></dict></plist>"#;
        let info = |id| {
            format!(r#"<?xml version="1.0"?><plist version="1.0"><dict>
            <key>DeviceIdentifier</key><string>{id}</string><key>ParentWholeDisk</key><string>{id}</string>
            <key>VirtualOrPhysical</key><string>Physical</string><key>WholeDisk</key><true/>
            <key>Internal</key><false/><key>RAIDMaster</key><false/><key>RAIDSlice</key><false/>
            <key>MediaName</key><string>Example</string><key>BusProtocol</key><string>USB</string>
            <key>TotalSize</key><integer>1024</integer></dict></plist>"#).into_bytes()
        };
        let disks = BTreeMap::from([
            ("disk4".into(), info("disk4")),
            ("disk5".into(), info("disk5")),
        ]);
        let result = screen_external_snapshot(list, list, root_info.as_bytes(), apfs, &disks);
        assert_eq!(
            result.map(|disks| disks
                .into_iter()
                .map(|disk| disk.identifier)
                .collect::<Vec<_>>()),
            Ok(vec!["disk5".into()])
        );
        let changed = br#"<?xml version="1.0"?><plist version="1.0"><dict>
            <key>WholeDisks</key><array><string>disk4</string></array>
            <key>AllDisksAndPartitions</key><array>
            <dict><key>DeviceIdentifier</key><string>disk4</string></dict>
            </array></dict></plist>"#;
        assert!(
            screen_external_snapshot(list, changed, root_info.as_bytes(), apfs, &disks).is_err()
        );
        assert!(
            screen_external_snapshot(list, list, root_info.as_bytes(), apfs, &BTreeMap::new())
                .is_err()
        );
        assert!(screen_external_snapshot(
            list,
            list,
            root_info.replace("disk4s1", "disk0s2").as_bytes(),
            apfs,
            &disks
        )
        .is_err());
    }

    #[test]
    fn includes_all_apfs_containers_and_rejects_ambiguous_stores() {
        let list = r#"<?xml version="1.0" encoding="UTF-8"?>
        <plist version="1.0"><dict><key>Containers</key><array>
        <dict><key>PhysicalStores</key><array>
        <dict><key>DeviceIdentifier</key><string>disk0s2</string></dict>
        <dict><key>DeviceIdentifier</key><string>disk4s1</string></dict>
        </array></dict>
        <dict><key>PhysicalStores</key><array>
        <dict><key>DeviceIdentifier</key><string>disk0s3</string></dict>
        </array></dict></array></dict></plist>"#;
        assert_eq!(
            apfs_store_parents(list.as_bytes()),
            Ok(["disk0".into(), "disk4".into()].into())
        );
        assert!(apfs_store_parents(list.replace("disk4s1", "disk4s1s1").as_bytes()).is_err());
        assert!(apfs_store_parents(list.replace("PhysicalStores", "Unknown").as_bytes()).is_err());
        assert!(apfs_store_parents(list.replace("disk0s2", "disk00s2").as_bytes()).is_err());
        assert!(apfs_store_parents(b"not a plist").is_err());
    }

    #[test]
    fn validates_external_disk_snapshot_consistency() {
        let list = r#"<?xml version="1.0" encoding="UTF-8"?>
        <plist version="1.0"><dict>
        <key>WholeDisks</key><array><string>disk4</string></array>
        <key>AllDisksAndPartitions</key><array>
        <dict><key>DeviceIdentifier</key><string>disk4</string></dict>
        </array></dict></plist>"#;
        assert_eq!(
            external_whole_disk_ids(list.as_bytes()),
            Ok(["disk4".into()].into())
        );
        assert!(external_whole_disk_ids(
            list.replace(
                "<string>disk4</string></array>",
                "<string>disk5</string></array>"
            )
            .as_bytes()
        )
        .is_err());
        assert!(external_whole_disk_ids(
            list.replace(
                "<string>disk4</string></array>",
                "<string>disk4</string><string>disk4</string></array>"
            )
            .as_bytes()
        )
        .is_err());
        assert!(external_whole_disk_ids(list.replace("disk4", "disk04").as_bytes()).is_err());
        assert!(external_whole_disk_ids(list.replace("disk4", "disk4s1").as_bytes()).is_err());
        assert!(external_whole_disk_ids(b"not a plist").is_err());
        let empty = list
            .replace("<string>disk4</string>", "")
            .replace("<dict><key>DeviceIdentifier</key></dict>", "");
        assert_eq!(
            external_whole_disk_ids(empty.as_bytes()),
            Ok(BTreeSet::new())
        );
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
