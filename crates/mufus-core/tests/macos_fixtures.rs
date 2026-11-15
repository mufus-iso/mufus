use std::collections::{BTreeMap, BTreeSet};

use mufus_core::macos::{external_whole_disk_ids, screen_external_snapshot};

const EMPTY: &[u8] = include_bytes!("fixtures/diskutil-empty.plist");
const EXTERNAL: &[u8] = include_bytes!("fixtures/diskutil-external.plist");
const INFO: &[u8] = include_bytes!("fixtures/diskutil-info-external.plist");
const ROOT: &str = include_str!("fixtures/diskutil-info-root.plist");
const APFS: &str = include_str!("fixtures/diskutil-apfs.plist");

#[test]
fn screens_external_and_system_disks_from_plist_snapshots() {
    assert_eq!(external_whole_disk_ids(EMPTY), Ok(BTreeSet::new()));
    let metadata = BTreeMap::from([("disk4".to_owned(), INFO.to_vec())]);
    let result = screen_external_snapshot(
        EXTERNAL,
        EXTERNAL,
        ROOT.as_bytes(),
        APFS.as_bytes(),
        &metadata,
    );
    assert_eq!(
        result.map(|disks| disks
            .into_iter()
            .map(|disk| disk.identifier)
            .collect::<Vec<_>>()),
        Ok(vec!["disk4".to_owned()])
    );

    let external_root = ROOT.replace("disk0s2", "disk4s1");
    let external_apfs = APFS.replace("disk0s2", "disk4s1");
    assert_eq!(
        screen_external_snapshot(
            EXTERNAL,
            EXTERNAL,
            external_root.as_bytes(),
            external_apfs.as_bytes(),
            &metadata
        ),
        Ok(Vec::new())
    );
    assert!(
        screen_external_snapshot(EXTERNAL, EMPTY, ROOT.as_bytes(), APFS.as_bytes(), &metadata)
            .is_err()
    );
    assert!(screen_external_snapshot(
        EXTERNAL,
        EXTERNAL,
        ROOT.as_bytes(),
        APFS.as_bytes(),
        &BTreeMap::new()
    )
    .is_err());
}
