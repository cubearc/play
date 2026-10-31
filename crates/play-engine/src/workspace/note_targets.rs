use super::{
    candidates,
    model::{CandidateSelection, Context, NoteLink, NoteTarget},
    notes, solutions, storage,
};
use crate::Result;
use serde_json::{Value, json};
use sqlx::{Row, SqliteConnection, sqlite::SqliteRow};
use uuid::Uuid;

fn reference(row: &SqliteRow) -> Result<NoteTarget> {
    let id = |field: &str| Uuid::parse_str(row.get(field)).map_err(|_| "corrupt_store");
    match row.get::<&str, _>("kind") {
        "block" => Ok(NoteTarget::Block {
            block_id: id("block_id")?,
        }),
        "asset" => Ok(NoteTarget::Asset {
            asset_id: id("asset_id")?,
            revision: row.get("asset_revision"),
        }),
        "attempt" => Ok(NoteTarget::Attempt {
            run_id: id("run_id")?,
            attempt_id: id("attempt_id")?,
        }),
        "run_output" => Ok(NoteTarget::RunOutput {
            run_id: id("run_id")?,
            attempt_id: id("attempt_id")?,
            port: row.get("port"),
        }),
        _ => Err("corrupt_store"),
    }
}
pub async fn read(
    connection: &mut SqliteConnection,
    workspace: &str,
    exploration: &str,
    note: &str,
) -> Result<Option<NoteLink>> {
    let row = sqlx::query(
        "SELECT * FROM note_targets WHERE workspace_id=? AND exploration_id=? AND note_id=?",
    )
    .bind(workspace)
    .bind(exploration)
    .bind(note)
    .fetch_optional(connection)
    .await
    .map_err(storage::failure)?;
    row.map(|r| {
        Ok(NoteLink {
            reference: reference(&r)?,
            label: r.get("label"),
        })
    })
    .transpose()
}
pub async fn copy(
    connection: &mut SqliteConnection,
    context: &Context,
    original: &str,
    copy: &str,
) -> Result<()> {
    sqlx::query("INSERT INTO note_targets SELECT workspace_id,exploration_id,stage_id,?,kind,label,block_id,asset_id,asset_revision,run_id,attempt_id,port FROM note_targets WHERE workspace_id=? AND exploration_id=? AND stage_id=? AND note_id=?")
 .bind(copy).bind(context.workspace_id.to_string()).bind(context.exploration_id.to_string()).bind(context.stage_id.to_string()).bind(original).execute(connection).await.map_err(storage::failure)?;
    Ok(())
}
pub async fn annotate(
    connection: &mut SqliteConnection,
    context: &Context,
    title: &str,
    text: &str,
    target: &NoteTarget,
) -> Result<Value> {
    let resolved = resolve(connection, context, target).await?;
    let label = resolved["label"].as_str().ok_or("invalid_state")?;
    let result = notes::add(connection, context, title, text).await?;
    let block = result["blockId"].as_str().ok_or("invalid_state")?;
    let note:String=sqlx::query_scalar("SELECT note_id FROM blocks WHERE workspace_id=? AND exploration_id=? AND stage_id=? AND id=?").bind(context.workspace_id.to_string()).bind(context.exploration_id.to_string()).bind(context.stage_id.to_string()).bind(block).fetch_one(&mut *connection).await.map_err(storage::failure)?;
    let (kind, block, asset, revision, run, attempt, port) = match target {
        NoteTarget::Block { block_id } => (
            "block",
            Some(block_id.to_string()),
            None,
            None,
            None,
            None,
            None,
        ),
        NoteTarget::Asset { asset_id, revision } => (
            "asset",
            None,
            Some(asset_id.to_string()),
            Some(*revision),
            None,
            None,
            None,
        ),
        NoteTarget::Attempt { run_id, attempt_id } => (
            "attempt",
            None,
            None,
            None,
            Some(run_id.to_string()),
            Some(attempt_id.to_string()),
            None,
        ),
        NoteTarget::RunOutput {
            run_id,
            attempt_id,
            port,
        } => (
            "run_output",
            None,
            None,
            None,
            Some(run_id.to_string()),
            Some(attempt_id.to_string()),
            Some(port.as_str()),
        ),
    };
    sqlx::query("INSERT INTO note_targets VALUES(?,?,?,?,?,?,?,?,?,?,?,?)")
        .bind(context.workspace_id.to_string())
        .bind(context.exploration_id.to_string())
        .bind(context.stage_id.to_string())
        .bind(note)
        .bind(kind)
        .bind(label)
        .bind(block)
        .bind(asset)
        .bind(revision)
        .bind(run)
        .bind(attempt)
        .bind(port)
        .execute(connection)
        .await
        .map_err(storage::failure)?;
    Ok(result)
}
pub async fn inspect(
    connection: &mut SqliteConnection,
    context: &Context,
    note: Uuid,
) -> Result<Value> {
    let row=sqlx::query("SELECT * FROM note_targets WHERE workspace_id=? AND exploration_id=? AND stage_id=? AND note_id=?").bind(context.workspace_id.to_string()).bind(context.exploration_id.to_string()).bind(context.stage_id.to_string()).bind(note.to_string()).fetch_optional(&mut *connection).await.map_err(storage::failure)?.ok_or("not_found")?;
    let target = reference(&row)?;
    let detail = solutions::available(resolve(connection, context, &target).await)?;
    if detail["kind"] == "unavailable" {
        Ok(json!({"label":row.get::<&str,_>("label"),"reference":target,"evidence":detail}))
    } else {
        Ok(detail)
    }
}
pub(super) async fn resolve(
    connection: &mut SqliteConnection,
    context: &Context,
    target: &NoteTarget,
) -> Result<Value> {
    let (label, evidence, origin) = match target {
        NoteTarget::Block { block_id } => {
            let row=sqlx::query("SELECT b.kind,b.archived_revision,COALESCE(a.title,n.title) AS title FROM blocks b LEFT JOIN assets a ON (b.workspace_id,b.exploration_id,b.asset_id)=(a.workspace_id,a.exploration_id,a.id) LEFT JOIN notes n ON (b.workspace_id,b.exploration_id,b.note_id)=(n.workspace_id,n.exploration_id,n.id) WHERE b.workspace_id=? AND b.exploration_id=? AND b.stage_id=? AND b.id=?").bind(context.workspace_id.to_string()).bind(context.exploration_id.to_string()).bind(context.stage_id.to_string()).bind(block_id.to_string()).fetch_optional(connection).await.map_err(storage::failure)?.ok_or("not_found")?;
            (
                format!("{} · Block", row.get::<&str, _>("title")),
                json!({"kind":"block","title":row.get::<&str,_>("title"),"archived":row.get::<Option<i64>,_>("archived_revision").is_some()}),
                Value::Null,
            )
        }
        NoteTarget::Asset { asset_id, revision } => {
            let selected = CandidateSelection::Asset {
                asset_id: *asset_id,
                revision: *revision,
            };
            let evidence = candidates::evidence(connection, context, &selected).await?;
            (
                format!(
                    "{} · Asset r{revision}",
                    evidence["asset"]["title"].as_str().ok_or("corrupt_store")?
                ),
                evidence,
                Value::Null,
            )
        }
        NoteTarget::Attempt { run_id, attempt_id } => {
            return attempt(connection, context, *run_id, *attempt_id, None, target).await;
        }
        NoteTarget::RunOutput {
            run_id,
            attempt_id,
            port,
        } => {
            return attempt(
                connection,
                context,
                *run_id,
                *attempt_id,
                Some(port),
                target,
            )
            .await;
        }
    };
    Ok(json!({"reference":target,"label":label,"evidence":evidence,"origin":origin}))
}
async fn attempt(
    connection: &mut SqliteConnection,
    context: &Context,
    run: Uuid,
    attempt: Uuid,
    port: Option<&str>,
    target: &NoteTarget,
) -> Result<Value> {
    let row=sqlx::query("SELECT r.rowid AS sequence,r.plan,r.trail_id,n.node_id,n.status,n.attempt,o.result FROM pipeline_runs r JOIN node_attempts n ON (r.workspace_id,r.exploration_id,r.id)=(n.workspace_id,n.exploration_id,n.run_id) LEFT JOIN node_results o ON (n.workspace_id,n.exploration_id,n.run_id,n.id)=(o.workspace_id,o.exploration_id,o.run_id,o.attempt_id) WHERE r.workspace_id=? AND r.exploration_id=? AND r.stage_id=? AND r.id=? AND n.id=?")
 .bind(context.workspace_id.to_string()).bind(context.exploration_id.to_string()).bind(context.stage_id.to_string()).bind(run.to_string()).bind(attempt.to_string()).fetch_optional(&mut *connection).await.map_err(storage::failure)?.ok_or("not_found")?;
    if row.get::<&str, _>("status") == "running" {
        return Err("attempt_not_finished");
    }
    let plan: Value = serde_json::from_str(row.get("plan")).map_err(|_| "corrupt_store")?;
    let node = plan["nodes"]
        .as_array()
        .and_then(|n| n.iter().find(|n| n["id"] == row.get::<&str, _>("node_id")))
        .ok_or("corrupt_store")?;
    let name = node["operation"]["name"].as_str().ok_or("corrupt_store")?;
    let (suffix, evidence) = if let Some(port) = port {
        if port.is_empty()
            || port.len() > 64
            || !port
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
        {
            return Err("invalid_command");
        }
        let contract = candidates::output_contract(&plan, node)?;
        if !contract.outputs.iter().any(|p| p.name == port) {
            return Err("not_found");
        }
        let value = solutions::available(
            candidates::evidence(
                connection,
                context,
                &CandidateSelection::RunOutput {
                    run_id: run,
                    attempt_id: attempt,
                    port: port.into(),
                },
            )
            .await,
        )?;
        (port.to_owned(), value)
    } else {
        let result: Value = row
            .get::<Option<&str>, _>("result")
            .map(serde_json::from_str)
            .transpose()
            .map_err(|_| "corrupt_store")?
            .unwrap_or(Value::Null);
        (
            format!("Attempt {}", row.get::<i64, _>("attempt")),
            json!({"kind":"attempt","status":row.get::<&str,_>("status"),"code":result["code"]}),
        )
    };
    Ok(
        json!({"reference":target,"label":format!("{name} · {suffix} · Run {}",row.get::<i64,_>("sequence")),"evidence":evidence,"origin":{"runId":run,"trailId":row.get::<&str,_>("trail_id")}}),
    )
}
