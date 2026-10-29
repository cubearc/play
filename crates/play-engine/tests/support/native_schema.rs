use sqlx::{Connection, SqliteConnection, sqlite::SqliteConnectOptions};
use std::{fs, path::Path};
use uuid::Uuid;

pub fn snapshot(root: &Path, workspace: Uuid, target: u32) {
    assert!((1..=4).contains(&target));
    let directory = root.join(workspace.to_string());
    let database = directory.join("workspace.sqlite3");
    let source = directory.join("schema-fixture-source.sqlite3");
    fs::rename(&database, &source).unwrap();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(async {
        let mut connection=SqliteConnection::connect_with(&SqliteConnectOptions::new().filename(&database).create_if_missing(true).foreign_keys(false)).await.unwrap();
        sqlx::raw_sql(include_str!("../../src/workspace/schema.sql")).execute(&mut connection).await.unwrap();
        if target>=2 {sqlx::raw_sql(include_str!("../../src/workspace/runs.sql")).execute(&mut connection).await.unwrap();}
        if target>=3 {sqlx::raw_sql(include_str!("../../src/workspace/copies.sql")).execute(&mut connection).await.unwrap();}
        if target>=4 {sqlx::raw_sql(include_str!("../../src/workspace/archive.sql")).execute(&mut connection).await.unwrap();}
        sqlx::query("ATTACH DATABASE ? AS original").bind(source.to_str().unwrap()).execute(&mut connection).await.unwrap();
        let files:i64=sqlx::query_scalar("SELECT count(*) FROM original.assets WHERE kind='file'").fetch_one(&mut connection).await.unwrap();assert_eq!(files,0,"an older native fixture cannot represent File Assets");
        if target<4 {let archived:i64=sqlx::query_scalar("SELECT count(*) FROM original.blocks WHERE archived_revision IS NOT NULL").fetch_one(&mut connection).await.unwrap();assert_eq!(archived,0,"an older native fixture cannot represent archived content");}
        if target<3 {let origins:i64=sqlx::query_scalar("SELECT count(*) FROM original.asset_origins").fetch_one(&mut connection).await.unwrap();assert_eq!(origins,0);}
        if target<2 {let runs:i64=sqlx::query_scalar("SELECT count(*) FROM original.pipeline_runs").fetch_one(&mut connection).await.unwrap();assert_eq!(runs,0);}
        sqlx::raw_sql("BEGIN").execute(&mut connection).await.unwrap();
        // Finished evidence is copied before its parent so admission guards do not reinterpret it.
        if target>=2 {
            for sql in ["INSERT INTO run_inputs SELECT * FROM original.run_inputs", "INSERT INTO node_attempts SELECT * FROM original.node_attempts", "INSERT INTO pipeline_runs SELECT * FROM original.pipeline_runs"] {
                sqlx::raw_sql(sql).execute(&mut connection).await.unwrap();
            }
        }
        for sql in [
            "INSERT INTO explorations SELECT * FROM original.explorations",
            "INSERT INTO stages SELECT * FROM original.stages",
            "INSERT INTO notes SELECT * FROM original.notes",
            "INSERT INTO note_revisions SELECT * FROM original.note_revisions",
            "INSERT INTO assets SELECT * FROM original.assets",
            "INSERT INTO asset_revisions SELECT * FROM original.asset_revisions",
            "INSERT INTO pipelines SELECT * FROM original.pipelines",
            "INSERT INTO pipeline_revisions SELECT * FROM original.pipeline_revisions",
            "INSERT INTO trails SELECT * FROM original.trails",
            "INSERT INTO view_state SELECT * FROM original.view_state",
            "INSERT INTO exploration_view_state SELECT * FROM original.exploration_view_state",
            "INSERT INTO receipts SELECT * FROM original.receipts",
        ] { sqlx::raw_sql(sql).execute(&mut connection).await.unwrap(); }
        let blocks=if target>=4 {"INSERT INTO blocks SELECT * FROM original.blocks"} else {"INSERT INTO blocks(workspace_id,exploration_id,stage_id,id,position,kind,asset_id,asset_revision,caption_id,caption_revision,caption_kind,note_id) SELECT workspace_id,exploration_id,stage_id,id,position,kind,asset_id,asset_revision,caption_id,caption_revision,caption_kind,note_id FROM original.blocks"};
        sqlx::raw_sql(blocks).execute(&mut connection).await.unwrap();
        if target>=3 {sqlx::raw_sql("INSERT INTO asset_origins SELECT * FROM original.asset_origins").execute(&mut connection).await.unwrap();}
        sqlx::query("INSERT INTO workspace_meta SELECT id,generation,? FROM original.workspace_meta").bind(target).execute(&mut connection).await.unwrap();
        sqlx::raw_sql("COMMIT; DETACH DATABASE original;").execute(&mut connection).await.unwrap();
        assert!(sqlx::query("PRAGMA foreign_key_check").fetch_optional(&mut connection).await.unwrap().is_none());
        connection.close().await.unwrap();
    });
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&database, fs::Permissions::from_mode(0o600)).unwrap();
    }
    fs::remove_file(source).unwrap();
}
