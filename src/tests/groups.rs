use super::*;

#[test]
fn load_from_missing_path_returns_empty() {
    let groups = load_groups(Path::new("/nonexistent/vm-curator/groups.toml"));
    assert!(groups.is_empty());
}

#[test]
fn save_then_load_roundtrips_order_and_membership() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("nested").join("groups.toml");

    let groups = vec![
        VmGroup {
            name: "Servers".to_string(),
            vm_ids: vec!["debian-12".to_string(), "ubuntu-24-04".to_string()],
        },
        VmGroup::new("Desktops"),
    ];

    save_groups(&path, &groups).unwrap();
    assert!(path.exists());

    let loaded = load_groups(&path);
    assert_eq!(loaded, groups);
}

#[test]
fn load_from_malformed_toml_returns_empty() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("groups.toml");
    std::fs::write(&path, "this is not = valid = toml = [[[").unwrap();

    assert!(load_groups(&path).is_empty());
}

#[test]
fn vm_group_contains_checks_membership() {
    let group = VmGroup {
        name: "Servers".to_string(),
        vm_ids: vec!["debian-12".to_string()],
    };
    assert!(group.contains("debian-12"));
    assert!(!group.contains("ubuntu-24-04"));
}

#[test]
fn prune_stale_removes_missing_vm_ids_only() {
    let mut groups = vec![
        VmGroup {
            name: "Servers".to_string(),
            vm_ids: vec!["debian-12".to_string(), "deleted-vm".to_string()],
        },
        VmGroup {
            name: "Desktops".to_string(),
            vm_ids: vec!["deleted-vm".to_string()],
        },
    ];
    let valid_ids: std::collections::HashSet<&str> = ["debian-12"].into_iter().collect();

    let changed = prune_stale(&mut groups, &valid_ids);

    assert!(changed);
    assert_eq!(groups[0].vm_ids, vec!["debian-12".to_string()]);
    assert!(groups[1].vm_ids.is_empty());
}

#[test]
fn prune_stale_no_op_when_all_ids_valid() {
    let mut groups = vec![VmGroup {
        name: "Servers".to_string(),
        vm_ids: vec!["debian-12".to_string()],
    }];
    let valid_ids: std::collections::HashSet<&str> =
        ["debian-12", "ubuntu-24-04"].into_iter().collect();

    let changed = prune_stale(&mut groups, &valid_ids);

    assert!(!changed);
    assert_eq!(groups[0].vm_ids, vec!["debian-12".to_string()]);
}

#[test]
fn groups_file_path_ends_with_expected_segments() {
    let path = groups_file_path();
    assert!(path.ends_with("vm-curator/groups.toml"));
}
