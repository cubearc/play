use super::{
    model::{Context, RunRequest},
    mutations, queries, run_plan,
    storage::{self, Owned},
};
use crate::{
    Result,
    protocol::{self, Invocation},
};
use serde_json::{Value, json};
use sqlx::{Connection, Row, SqliteConnection, sqlite::SqliteRow};
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

#[derive(Clone)]
pub struct RunIdentity {
    pub context: Context,
    pub id: Uuid,
    pub node_id: Uuid,
}
pub struct PreparedRun {
    pub identity: RunIdentity,
    pub invocation: Invocation,
}
fn now() -> Result<i64> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|value| i64::try_from(value.as_millis()).ok())
        .ok_or("clock_unavailable")
}
fn digest(request: &RunRequest) -> Result<String> {
    Ok(cube_core::hashing::sha256(
        &serde_json::to_vec(request).map_err(|_| "invalid_command")?,
    ))
}
async fn authorize(store: &mut Owned, context: &Context) -> Result<()> {
    let owner = store.identity.workspace_id;
    queries::context(store.connection()?, owner, context).await
}

pub async fn existing(store: &mut Owned, request: &RunRequest) -> Result<Option<Value>> {
    authorize(store, &request.context).await?;
    let row=sqlx::query("SELECT id,intent_digest FROM pipeline_runs WHERE workspace_id=? AND exploration_id=? AND intent_key=?")
  .bind(request.context.workspace_id.to_string()).bind(request.context.exploration_id.to_string()).bind(request.key.to_string()).fetch_optional(store.connection()?).await.map_err(storage::failure)?;
    if let Some(row) = row {
        if row.get::<String, _>("intent_digest") != digest(request)? {
            return Err("idempotency_conflict");
        }
        let id = Uuid::parse_str(row.get("id")).map_err(|_| "corrupt_store")?;
        return detail(store, &request.context, id).await.map(Some);
    }
    Ok(None)
}

pub async fn prepare(store: &mut Owned, request: &RunRequest) -> Result<PreparedRun> {
    authorize(store, &request.context).await?;
    let context = &request.context;
    let workspace = context.workspace_id.to_string();
    let exploration = context.exploration_id.to_string();
    let mut tx = store
        .connection()?
        .begin()
        .await
        .map_err(storage::failure)?;
    for (sql, maximum) in [
        (
            "SELECT count(*) FROM pipeline_runs WHERE workspace_id=?",
            protocol::contract().workspace_run_limit,
        ),
        (
            "SELECT count(*) FROM pipeline_runs WHERE workspace_id=? AND exploration_id=?",
            protocol::contract().exploration_run_limit,
        ),
    ] {
        let mut query = sqlx::query_scalar::<_, i64>(sql).bind(&workspace);
        if sql.contains("exploration_id") {
            query = query.bind(&exploration);
        }
        if query.fetch_one(&mut *tx).await.map_err(storage::failure)? >= maximum as i64 {
            return Err("storage_quota");
        }
    }
    mutations::bytes(
        &mut tx,
        &workspace,
        &exploration,
        protocol::contract().run_reserve_bytes + 16384,
    )
    .await?;
    let plan = run_plan::build(&mut tx, request).await?;
    let identity = RunIdentity {
        context: context.clone(),
        id: Uuid::new_v4(),
        node_id: plan.node_id,
    };
    sqlx::query("INSERT INTO pipeline_runs(workspace_id,exploration_id,stage_id,id,trail_id,pipeline_id,pipeline_revision,intent_key,intent_digest,caller,status,created_ms,retain_output,plan,reservation) VALUES(?,?,?,?,?,?,?,?,?,'desktop','queued',?,?,?,zeroblob(?))")
  .bind(&workspace).bind(&exploration).bind(context.stage_id.to_string()).bind(identity.id.to_string()).bind(request.trail_id.to_string()).bind(plan.pipeline_id).bind(plan.pipeline_revision).bind(request.key.to_string()).bind(digest(request)?).bind(now()?).bind(request.retain_output).bind(plan.value.to_string()).bind(protocol::contract().run_reserve_bytes as i64).execute(&mut *tx).await.map_err(storage::failure)?;
    sqlx::query("INSERT INTO run_inputs VALUES(?,?,?,?,?,?)")
        .bind(&workspace)
        .bind(&exploration)
        .bind(identity.id.to_string())
        .bind(plan.input_port)
        .bind(plan.asset_id)
        .bind(plan.asset_revision)
        .execute(&mut *tx)
        .await
        .map_err(storage::failure)?;
    tx.commit().await.map_err(storage::failure)?;
    Ok(PreparedRun {
        identity,
        invocation: plan.invocation,
    })
}

pub async fn started(store: &mut Owned, run: &RunIdentity) -> Result<()> {
    authorize(store, &run.context).await?;
    let mut tx = store
        .connection()?
        .begin()
        .await
        .map_err(storage::failure)?;
    let target:Option<String>=sqlx::query_scalar("SELECT json_extract(plan,'$.targetNodes[0]') FROM pipeline_runs WHERE workspace_id=? AND exploration_id=? AND stage_id=? AND id=? AND status='queued'").bind(run.context.workspace_id.to_string()).bind(run.context.exploration_id.to_string()).bind(run.context.stage_id.to_string()).bind(run.id.to_string()).fetch_optional(&mut *tx).await.map_err(storage::failure)?;
    if target.as_deref() != Some(run.node_id.to_string().as_str()) {
        return Err("state_conflict");
    }
    let time = now()?;
    sqlx::query("INSERT INTO node_attempts VALUES(?,?,?,?,?,1,?,NULL,'running')")
        .bind(run.context.workspace_id.to_string())
        .bind(run.context.exploration_id.to_string())
        .bind(run.id.to_string())
        .bind(Uuid::new_v4().to_string())
        .bind(run.node_id.to_string())
        .bind(time)
        .execute(&mut *tx)
        .await
        .map_err(storage::failure)?;
    let updated=sqlx::query("UPDATE pipeline_runs SET status='running' WHERE workspace_id=? AND exploration_id=? AND id=? AND status='queued'").bind(run.context.workspace_id.to_string()).bind(run.context.exploration_id.to_string()).bind(run.id.to_string()).execute(&mut *tx).await.map_err(storage::failure)?;
    if updated.rows_affected() != 1 {
        return Err("state_conflict");
    }
    tx.commit().await.map_err(storage::failure)
}

pub async fn finish(store: &mut Owned, run: &RunIdentity, result: Value) -> Result<Value> {
    authorize(store, &run.context).await?;
    let mut tx = store
        .connection()?
        .begin()
        .await
        .map_err(storage::failure)?;
    let row=sqlx::query("SELECT status,retain_output,plan FROM pipeline_runs WHERE workspace_id=? AND exploration_id=? AND stage_id=? AND id=?").bind(run.context.workspace_id.to_string()).bind(run.context.exploration_id.to_string()).bind(run.context.stage_id.to_string()).bind(run.id.to_string()).fetch_optional(&mut *tx).await.map_err(storage::failure)?.ok_or("not_found")?;
    if !matches!(row.get::<&str, _>("status"), "queued" | "running") {
        return Err("state_conflict");
    }
    let plan: Value = serde_json::from_str(row.get("plan")).map_err(|_| "corrupt_store")?;
    let result = normalize(result, &plan, row.get::<bool, _>("retain_output"))?;
    let status = result["status"].as_str().ok_or("invalid_response")?;
    let time = now()?;
    sqlx::query("UPDATE pipeline_runs SET status=?,finished_ms=?,result=?,reservation=x'' WHERE workspace_id=? AND exploration_id=? AND id=?")
  .bind(status).bind(time).bind(result.to_string()).bind(run.context.workspace_id.to_string()).bind(run.context.exploration_id.to_string()).bind(run.id.to_string()).execute(&mut *tx).await.map_err(storage::failure)?;
    sqlx::query("UPDATE node_attempts SET status=?,finished_ms=? WHERE workspace_id=? AND exploration_id=? AND run_id=? AND status='running'").bind(status).bind(time).bind(run.context.workspace_id.to_string()).bind(run.context.exploration_id.to_string()).bind(run.id.to_string()).execute(&mut *tx).await.map_err(storage::failure)?;
    tx.commit().await.map_err(storage::failure)?;
    detail(store, &run.context, run.id).await
}

fn normalize(result: Value, plan: &Value, retain: bool) -> Result<Value> {
    let status = result["status"].as_str().ok_or("invalid_response")?;
    match status {
        "completed" => {
            let operation = plan["operation"]["operation"]
                .as_str()
                .ok_or("corrupt_store")?;
            let outputs = crate::worker::named_outputs(operation, &result["output"])?;
            let contract = cube_core::operations::require(operation)
                .map_err(|_| "contract_mismatch")?
                .contract()
                .map_err(|_| "contract_mismatch")?;
            contract
                .validate_outputs(&outputs)
                .map_err(|_| "invalid_response")?;
            if outputs.to_string().len() > protocol::contract().run_reserve_bytes {
                return Err("output_limit");
            }
            Ok(if retain {
                json!({"status":status,"outputs":outputs})
            } else {
                json!({"status":status,"replayGap":"output_not_retained"})
            })
        }
        "failed" | "cancelled" | "interrupted" => {
            let code = result["code"].as_str().unwrap_or("operation_stopped");
            if code.len() > 64 || !code.bytes().all(|c| c.is_ascii_lowercase() || c == b'_') {
                return Err("invalid_response");
            }
            Ok(json!({"status":status,"code":code}))
        }
        _ => Err("invalid_response"),
    }
}

fn summary(row: &SqliteRow) -> Result<Value> {
    let plan: Value = serde_json::from_str(row.get("plan")).map_err(|_| "corrupt_store")?;
    Ok(
        json!({"id":row.get::<String,_>("id"),"sequence":row.get::<i64,_>("sequence"),"trailId":row.get::<String,_>("trail_id"),"pipelineId":row.get::<String,_>("pipeline_id"),"pipelineRevision":row.get::<i64,_>("pipeline_revision"),"status":row.get::<String,_>("status"),"createdMs":row.get::<i64,_>("created_ms"),"finishedMs":row.get::<Option<i64>,_>("finished_ms"),"retainOutput":row.get::<bool,_>("retain_output"),"operation":plan["operation"]}),
    )
}

pub async fn detail(store: &mut Owned, context: &Context, id: Uuid) -> Result<Value> {
    authorize(store, context).await?;
    let row=sqlx::query("SELECT rowid AS sequence,* FROM pipeline_runs WHERE workspace_id=? AND exploration_id=? AND stage_id=? AND id=?")
  .bind(context.workspace_id.to_string()).bind(context.exploration_id.to_string()).bind(context.stage_id.to_string()).bind(id.to_string()).fetch_optional(store.connection()?).await.map_err(storage::failure)?.ok_or("not_found")?;
    let mut value = summary(&row)?;
    value["plan"] = serde_json::from_str(row.get("plan")).map_err(|_| "corrupt_store")?;
    value["result"] = row
        .get::<Option<&str>, _>("result")
        .map(serde_json::from_str)
        .transpose()
        .map_err(|_| "corrupt_store")?
        .unwrap_or(Value::Null);
    let attempts=sqlx::query("SELECT id,node_id,attempt,started_ms,finished_ms,status FROM node_attempts WHERE workspace_id=? AND exploration_id=? AND run_id=? ORDER BY attempt")
  .bind(context.workspace_id.to_string()).bind(context.exploration_id.to_string()).bind(id.to_string()).fetch_all(store.connection()?).await.map_err(storage::failure)?;
    value["attempts"]=json!(attempts.into_iter().map(|row|json!({"id":row.get::<String,_>(0),"nodeId":row.get::<String,_>(1),"attempt":row.get::<i64,_>(2),"startedMs":row.get::<i64,_>(3),"finishedMs":row.get::<Option<i64>,_>(4),"status":row.get::<String,_>(5)})).collect::<Vec<_>>());
    Ok(value)
}

pub async fn list(
    store: &mut Owned,
    context: &Context,
    trail: Uuid,
    before: Option<i64>,
) -> Result<Value> {
    authorize(store, context).await?;
    if before.is_some_and(|value| value <= 0) {
        return Err("invalid_command");
    }
    let exists:i64=sqlx::query_scalar("SELECT count(*) FROM trails WHERE workspace_id=? AND exploration_id=? AND stage_id=? AND id=?").bind(context.workspace_id.to_string()).bind(context.exploration_id.to_string()).bind(context.stage_id.to_string()).bind(trail.to_string()).fetch_one(store.connection()?).await.map_err(storage::failure)?;
    if exists != 1 {
        return Err("not_found");
    }
    let rows=sqlx::query("SELECT rowid AS sequence,* FROM pipeline_runs WHERE workspace_id=? AND exploration_id=? AND stage_id=? AND trail_id=? AND rowid<? ORDER BY rowid DESC LIMIT ?")
  .bind(context.workspace_id.to_string()).bind(context.exploration_id.to_string()).bind(context.stage_id.to_string()).bind(trail.to_string()).bind(before.unwrap_or(i64::MAX)).bind(protocol::contract().run_page_limit as i64+1).fetch_all(store.connection()?).await.map_err(storage::failure)?;
    let more = rows.len() > protocol::contract().run_page_limit;
    let records = rows
        .iter()
        .take(protocol::contract().run_page_limit)
        .map(summary)
        .collect::<Result<Vec<_>>>()?;
    let cursor = if more {
        records.last().and_then(|v| v["sequence"].as_i64())
    } else {
        None
    };
    Ok(json!({"runs":records,"before":cursor}))
}

pub async fn recover(connection: &mut SqliteConnection) -> Result<()> {
    let time = now()?;
    let mut tx = connection.begin().await.map_err(storage::failure)?;
    sqlx::query("UPDATE pipeline_runs SET status='interrupted',finished_ms=?,result='{\"status\":\"interrupted\",\"code\":\"owner_stopped\"}',reservation=x'' WHERE status IN ('queued','running')").bind(time).execute(&mut *tx).await.map_err(storage::failure)?;
    sqlx::query(
        "UPDATE node_attempts SET status='interrupted',finished_ms=? WHERE status='running'",
    )
    .bind(time)
    .execute(&mut *tx)
    .await
    .map_err(storage::failure)?;
    tx.commit().await.map_err(storage::failure)
}
