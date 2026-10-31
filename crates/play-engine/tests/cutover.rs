use play_engine::workspace::{Command, Manager};
use serde_json::json;
use std::{fs, path::Path};
use uuid::Uuid;

fn write(path: &Path, bytes: &[u8]) {
    fs::write(path, bytes).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o600)).unwrap();
    }
}
fn files(path: &Path) -> Vec<(String, Vec<u8>)> {
    let mut result = fs::read_dir(path)
        .unwrap()
        .map(|e| {
            let e = e.unwrap();
            (
                e.file_name().to_string_lossy().into_owned(),
                fs::read(e.path()).unwrap(),
            )
        })
        .collect::<Vec<_>>();
    result.sort_by(|a, b| a.0.cmp(&b.0));
    result
}
#[test]
fn unsupported_schema_versions_are_rejected_without_touching_any_archived_bytes() {
    for version in 1u32..16 {
        let t = tempfile::tempdir().unwrap();
        let root = t.path().join("private");
        let mut m = Manager::new(root.clone()).unwrap();
        let id = Uuid::parse_str("11111111-1111-4111-8111-111111111111").unwrap();
        let d = root.join(id.to_string());
        fs::create_dir(&d).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&d, fs::Permissions::from_mode(0o700)).unwrap();
        }
        let mut db = include_bytes!("fixtures/unsupported-store.sqlite3").to_vec();
        db[60..64].copy_from_slice(&version.to_be_bytes());
        write(&d.join("workspace.sqlite3"), &db);
        write(&d.join("owner.lock"), b"");
        write(&d.join("workspace.json"),&serde_json::to_vec(&json!({"product":"cubearc.play.workspace","generation":2,"workspace_id":id,"title":"Archived workspace","creation_key":Uuid::new_v4(),"creation_digest":"archived"})).unwrap());
        let before = files(&d);
        assert_eq!(
            m.handle(Command::Open { workspace_id: id }).unwrap_err(),
            "unsupported_store",
            "schema {version}"
        );
        assert_eq!(files(&d), before, "schema {version} was mutated");
    }
}
#[test]
fn an_archived_upgrade_marker_never_authorizes_cleanup_or_conversion() {
    let t = tempfile::tempdir().unwrap();
    let root = t.path().join("private");
    let mut m = Manager::new(root.clone()).unwrap();
    let v = m
        .handle(
            serde_json::from_value(json!({"kind":"create","key":Uuid::new_v4(),"title":"Current"}))
                .unwrap(),
        )
        .unwrap();
    let id: Uuid = serde_json::from_value(v["id"].clone()).unwrap();
    drop(m);
    let d = root.join(id.to_string());
    let temporary = Uuid::new_v4();
    write(
        &d.join(format!(".upgrade-{temporary}.sqlite3")),
        b"Preserve this archived copy",
    );
    write(&d.join("upgrade.json"),&serde_json::to_vec(&json!({"product":"cubearc.play.workspace.upgrade","workspace_id":id,"generation":2,"from_schema":15,"to_schema":16,"temporary":temporary})).unwrap());
    let before = files(&d);
    let mut m = Manager::new(root).unwrap();
    assert_eq!(
        m.handle(Command::Open { workspace_id: id }).unwrap_err(),
        "unsupported_store"
    );
    assert_eq!(files(&d), before);
}
