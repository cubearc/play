use super::{
    candidates,
    model::{CandidateSelection, Command, Context},
    mutations, solution_lineage, solution_path, storage,
};
use crate::{Result, protocol};
use serde_json::{Value, json};
use sqlx::{Row, SqliteConnection, sqlite::SqliteRow};
use uuid::Uuid;

async fn history(
    connection: &mut SqliteConnection,
    context: &Context,
    id: Uuid,
) -> Result<Vec<SqliteRow>> {
    sqlx::query("SELECT r.* FROM solutions s JOIN solution_revisions r ON (s.workspace_id,s.exploration_id,s.id)=(r.workspace_id,r.exploration_id,r.solution_id) WHERE s.workspace_id=? AND s.exploration_id=? AND s.stage_id=? AND s.id=? ORDER BY r.revision DESC LIMIT ?")
        .bind(context.workspace_id.to_string()).bind(context.exploration_id.to_string()).bind(context.stage_id.to_string()).bind(id.to_string()).bind(protocol::contract().solution_revision_limit as i64 + 1).fetch_all(connection).await.map_err(storage::failure)
}
pub async fn list(connection: &mut SqliteConnection, context: &Context) -> Result<Value> {
    let rows=sqlx::query("SELECT s.id,r.title,r.revision FROM solutions s JOIN solution_revisions r ON (s.workspace_id,s.exploration_id,s.id)=(r.workspace_id,r.exploration_id,r.solution_id) WHERE s.workspace_id=? AND s.exploration_id=? AND s.stage_id=? AND r.revision=(SELECT MAX(h.revision) FROM solution_revisions h WHERE (h.workspace_id,h.exploration_id,h.solution_id)=(s.workspace_id,s.exploration_id,s.id)) ORDER BY s.rowid LIMIT ?")
        .bind(context.workspace_id.to_string()).bind(context.exploration_id.to_string()).bind(context.stage_id.to_string()).bind(protocol::contract().solution_limit as i64 + 1).fetch_all(connection).await.map_err(storage::failure)?;
    if rows.len() > protocol::contract().solution_limit {
        return Err("corrupt_store");
    }
    Ok(
        json!({"solutions":rows.iter().map(|r|json!({"id":r.get::<&str,_>("id"),"title":r.get::<&str,_>("title"),"revision":r.get::<i64,_>("revision")})).collect::<Vec<_>>()}),
    )
}
pub async fn save(connection: &mut SqliteConnection, command: &Command) -> Result<Value> {
    let Command::SaveSolution {
        context,
        solution_id,
        expected_revision,
        candidate_revision,
        title,
        explanation,
        ..
    } = command
    else {
        return Err("invalid_command");
    };
    let title = mutations::title(title)?;
    if *expected_revision < 0
        || *expected_revision == i64::MAX
        || *candidate_revision < 1
        || explanation.trim().is_empty()
    {
        return Err("invalid_command");
    }
    if explanation.len() > protocol::contract().note_bytes {
        return Err("input_limit");
    }
    let rows = history(connection, context, *solution_id).await?;
    let current = rows.first().map_or(0, |r| r.get::<i64, _>("revision"));
    if current != *expected_revision {
        return Err("state_conflict");
    }
    if rows.len() >= protocol::contract().solution_revision_limit {
        return Err("solution_revision_limit");
    }
    if current == 0
        && list(connection, context).await?["solutions"]
            .as_array()
            .ok_or("invalid_state")?
            .len()
            >= protocol::contract().solution_limit
    {
        return Err("solution_limit");
    }
    let candidate = candidates::read(connection, context, Some(*candidate_revision)).await?;
    let selected: CandidateSelection =
        serde_json::from_value(candidate["selection"].clone()).map_err(|_| "corrupt_store")?;
    candidates::evidence(connection, context, &selected).await?;
    let path = solution_path::build(connection, context, &selected).await?;
    let lineage = solution_lineage::build(connection, context, &selected, &path).await?;
    let snapshot = json!({"formatVersion":1,"stageId":context.stage_id,"candidate":{"id":candidate["id"],"revision":candidate_revision},"selection":selected,"path":path,"lineage":lineage});
    let encoded = snapshot.to_string();
    if encoded.len() > protocol::contract().solution_snapshot_bytes {
        return Err("output_limit");
    }
    mutations::bytes(
        connection,
        &context.workspace_id.to_string(),
        &context.exploration_id.to_string(),
        encoded.len() + title.len() + explanation.len(),
    )
    .await?;
    if current == 0 {
        sqlx::query("INSERT INTO solutions VALUES(?,?,?,?)")
            .bind(context.workspace_id.to_string())
            .bind(context.exploration_id.to_string())
            .bind(context.stage_id.to_string())
            .bind(solution_id.to_string())
            .execute(&mut *connection)
            .await
            .map_err(storage::failure)?;
    }
    sqlx::query("INSERT INTO solution_revisions VALUES(?,?,?,?,?,?,?,?,?)")
        .bind(context.workspace_id.to_string())
        .bind(context.exploration_id.to_string())
        .bind(solution_id.to_string())
        .bind(current + 1)
        .bind(candidate["id"].as_str().ok_or("corrupt_store")?)
        .bind(candidate_revision)
        .bind(title)
        .bind(explanation)
        .bind(encoded)
        .execute(connection)
        .await
        .map_err(storage::failure)?;
    Ok(json!({"solutionId":solution_id,"revision":current+1}))
}
pub async fn read(
    connection: &mut SqliteConnection,
    context: &Context,
    id: Uuid,
    revision: Option<i64>,
) -> Result<Value> {
    if revision.is_some_and(|r| r < 1) {
        return Err("invalid_command");
    }
    let rows = history(connection, context, id).await?;
    let latest = rows.first().ok_or("not_found")?.get::<i64, _>("revision");
    if rows.len() > protocol::contract().solution_revision_limit {
        return Err("corrupt_store");
    }
    let selected_revision = revision.unwrap_or(latest);
    let row = rows
        .iter()
        .find(|r| r.get::<i64, _>("revision") == selected_revision)
        .ok_or("not_found")?;
    let snapshot: Value = serde_json::from_str(row.get("snapshot")).map_err(|_| "corrupt_store")?;
    if snapshot["formatVersion"] != 1 {
        return Err("unsupported_solution");
    }
    let selected =
        serde_json::from_value(snapshot["selection"].clone()).map_err(|_| "corrupt_store")?;
    let evidence = available(candidates::evidence(connection, context, &selected).await)?;
    let mut gaps = solution_path::gaps(connection, context, &snapshot["path"]).await?;
    if let Some(lineage) = snapshot.get("lineage") {
        gaps.extend(solution_lineage::gaps(connection, context, lineage).await?);
    }
    let gaps = gaps
        .into_iter()
        .map(|gap| {
            Ok((
                gap["reference"].as_str().ok_or("corrupt_store")?.to_owned(),
                gap,
            ))
        })
        .collect::<Result<std::collections::BTreeMap<_, _>>>()?
        .into_values()
        .collect::<Vec<_>>();
    Ok(
        json!({"id":id,"revision":latest,"selectedRevision":selected_revision,"title":row.get::<&str,_>("title"),"explanation":row.get::<&str,_>("explanation"),"status":"private_unverified","snapshot":snapshot,"evidence":evidence,"gaps":gaps,"history":rows.iter().map(|r|json!({"revision":r.get::<i64,_>("revision"),"title":r.get::<&str,_>("title")})).collect::<Vec<_>>()}),
    )
}
pub(super) fn available(result: Result<Value>) -> Result<Value> {
    match result {
        Ok(value) => Ok(value),
        Err(
            code @ ("not_found"
            | "result_not_available"
            | "contract_mismatch"
            | "retention_policy_unavailable"),
        ) => Ok(json!({"kind":"unavailable","code":code})),
        Err(code) => Err(code),
    }
}
