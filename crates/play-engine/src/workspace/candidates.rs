use super::{
    model::{CandidateSelection, Context},
    queries, storage,
};
use crate::{Result, protocol};
use serde_json::{Value, json};
use sqlx::{Row, SqliteConnection, sqlite::SqliteRow};
use uuid::Uuid;

async fn history(connection: &mut SqliteConnection, context: &Context) -> Result<Vec<SqliteRow>> {
    sqlx::query("SELECT c.id,r.* FROM candidates c JOIN candidate_revisions r ON (c.workspace_id,c.exploration_id,c.id)=(r.workspace_id,r.exploration_id,r.candidate_id) WHERE c.workspace_id=? AND c.exploration_id=? AND c.stage_id=? ORDER BY r.revision DESC LIMIT ?")
        .bind(context.workspace_id.to_string()).bind(context.exploration_id.to_string()).bind(context.stage_id.to_string()).bind(protocol::contract().candidate_revision_limit as i64 + 1)
        .fetch_all(connection).await.map_err(storage::failure)
}
fn selection(row: &SqliteRow) -> Result<CandidateSelection> {
    let uuid = |field: &str| Uuid::parse_str(row.get(field)).map_err(|_| "corrupt_store");
    if row.get::<Option<&str>, _>("asset_id").is_some() {
        Ok(CandidateSelection::Asset {
            asset_id: uuid("asset_id")?,
            revision: row.get("asset_revision"),
        })
    } else {
        Ok(CandidateSelection::RunOutput {
            run_id: uuid("run_id")?,
            attempt_id: uuid("attempt_id")?,
            port: row.get("port"),
        })
    }
}

pub async fn select(
    connection: &mut SqliteConnection,
    context: &Context,
    expected: i64,
    selected: &CandidateSelection,
) -> Result<Value> {
    if expected < 0 || expected == i64::MAX {
        return Err("invalid_command");
    }
    let rows = history(connection, context).await?;
    let current = rows.first().map_or(0, |row| row.get::<i64, _>("revision"));
    if current != expected {
        return Err("state_conflict");
    }
    if rows.len() >= protocol::contract().candidate_revision_limit {
        return Err("candidate_revision_limit");
    }
    evidence(connection, context, selected).await?;
    let id = rows.first().map_or_else(
        || Uuid::new_v4().to_string(),
        |row| row.get::<String, _>("id"),
    );
    if rows.is_empty() {
        sqlx::query("INSERT INTO candidates VALUES(?,?,?,?)")
            .bind(context.workspace_id.to_string())
            .bind(context.exploration_id.to_string())
            .bind(context.stage_id.to_string())
            .bind(&id)
            .execute(&mut *connection)
            .await
            .map_err(storage::failure)?;
    }
    let (asset, revision, run, attempt, port) = match selected {
        CandidateSelection::Asset { asset_id, revision } => (
            Some(asset_id.to_string()),
            Some(*revision),
            None,
            None,
            None,
        ),
        CandidateSelection::RunOutput {
            run_id,
            attempt_id,
            port,
        } => (
            None,
            None,
            Some(run_id.to_string()),
            Some(attempt_id.to_string()),
            Some(port.as_str()),
        ),
    };
    sqlx::query("INSERT INTO candidate_revisions VALUES(?,?,?,?,?,?,?,?,?)")
        .bind(context.workspace_id.to_string())
        .bind(context.exploration_id.to_string())
        .bind(&id)
        .bind(current + 1)
        .bind(asset)
        .bind(revision)
        .bind(run)
        .bind(attempt)
        .bind(port)
        .execute(connection)
        .await
        .map_err(storage::failure)?;
    Ok(json!({"candidateId":id,"revision":current + 1}))
}

pub async fn read(
    connection: &mut SqliteConnection,
    context: &Context,
    revision: Option<i64>,
) -> Result<Value> {
    if revision.is_some_and(|r| r < 1) {
        return Err("invalid_command");
    }
    let rows = history(connection, context).await?;
    let Some(current) = rows.first() else {
        return if revision.is_none() {
            Ok(Value::Null)
        } else {
            Err("not_found")
        };
    };
    if rows.len() > protocol::contract().candidate_revision_limit {
        return Err("corrupt_store");
    }
    let current_revision = current.get::<i64, _>("revision");
    let selected_revision = revision.unwrap_or(current_revision);
    let selected = rows
        .iter()
        .find(|r| r.get::<i64, _>("revision") == selected_revision)
        .ok_or("not_found")?;
    let selected = selection(selected)?;
    let resolved = match evidence(connection, context, &selected).await {
        Ok(value) => value,
        Err(
            code @ ("not_found"
            | "result_not_available"
            | "contract_mismatch"
            | "retention_policy_unavailable"),
        ) => json!({"kind":"unavailable","code":code}),
        Err(code) => return Err(code),
    };
    let entries = rows
        .iter()
        .map(|r| Ok(json!({"revision":r.get::<i64,_>("revision"),"selection":selection(r)?})))
        .collect::<Result<Vec<_>>>()?;
    Ok(
        json!({"id":current.get::<String,_>("id"),"revision":current_revision,"selectedRevision":selected_revision,"status":"private_unverified","selection":selected,"history":entries,"evidence":resolved}),
    )
}

pub(super) async fn evidence(
    connection: &mut SqliteConnection,
    context: &Context,
    selected: &CandidateSelection,
) -> Result<Value> {
    match selected {
        CandidateSelection::Asset { asset_id, revision } => {
            if *revision < 1 {
                return Err("invalid_command");
            }
            let exists:i64=sqlx::query_scalar("SELECT count(*) FROM blocks WHERE workspace_id=? AND exploration_id=? AND stage_id=? AND (asset_id=? OR caption_id=?)").bind(context.workspace_id.to_string()).bind(context.exploration_id.to_string()).bind(context.stage_id.to_string()).bind(asset_id.to_string()).bind(asset_id.to_string()).fetch_one(&mut *connection).await.map_err(storage::failure)?;
            if exists == 0 {
                return Err("not_found");
            }
            let asset = queries::asset(
                connection,
                &context.workspace_id.to_string(),
                &context.exploration_id.to_string(),
                &asset_id.to_string(),
                *revision,
            )
            .await?;
            Ok(json!({"kind":"asset","asset":asset}))
        }
        CandidateSelection::RunOutput {
            run_id,
            attempt_id,
            port,
        } => output(connection, context, *run_id, *attempt_id, port).await,
    }
}

async fn output(
    connection: &mut SqliteConnection,
    context: &Context,
    run: Uuid,
    attempt: Uuid,
    port: &str,
) -> Result<Value> {
    if port.is_empty()
        || port.len() > 64
        || !port
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || c == b'_' || c == b'-')
    {
        return Err("invalid_command");
    }
    let row=sqlx::query("SELECT r.rowid AS sequence,r.status,r.retain_output,r.plan,r.trail_id,r.pipeline_id,r.pipeline_revision,n.node_id,n.status AS attempt_status,o.result FROM pipeline_runs r JOIN node_attempts n ON (r.workspace_id,r.exploration_id,r.id)=(n.workspace_id,n.exploration_id,n.run_id) LEFT JOIN node_results o ON (n.workspace_id,n.exploration_id,n.run_id,n.id)=(o.workspace_id,o.exploration_id,o.run_id,o.attempt_id) WHERE r.workspace_id=? AND r.exploration_id=? AND r.stage_id=? AND r.id=? AND n.id=?")
        .bind(context.workspace_id.to_string()).bind(context.exploration_id.to_string()).bind(context.stage_id.to_string()).bind(run.to_string()).bind(attempt.to_string()).fetch_optional(connection).await.map_err(storage::failure)?.ok_or("not_found")?;
    if row.get::<&str, _>("status") != "completed"
        || row.get::<&str, _>("attempt_status") != "completed"
        || !row.get::<bool, _>("retain_output")
    {
        return Err("result_not_available");
    }
    let plan: Value = serde_json::from_str(row.get("plan")).map_err(|_| "corrupt_store")?;
    let result: Value = serde_json::from_str(
        row.get::<Option<&str>, _>("result")
            .ok_or("result_not_available")?,
    )
    .map_err(|_| "corrupt_store")?;
    let node = plan["nodes"]
        .as_array()
        .and_then(|nodes| {
            nodes
                .iter()
                .find(|n| n["id"] == row.get::<&str, _>("node_id"))
        })
        .ok_or("corrupt_store")?;
    let contract = output_contract(&plan, node)?;
    if result["status"] != "completed" || result.get("replayGap").is_some() {
        return Err("result_not_available");
    }
    contract
        .validate_outputs(&result["outputs"])
        .map_err(|_| "corrupt_store")?;
    let definition = contract
        .outputs
        .iter()
        .find(|p| p.name == port)
        .ok_or("not_found")?;
    let value = result["outputs"].get(port).ok_or("result_not_available")?;
    Ok(
        json!({"kind":"run_output","semanticType":definition.semantic_type,"value":value,"origin":{"runId":run,"attemptId":attempt,"nodeId":row.get::<&str,_>("node_id"),"trailId":row.get::<&str,_>("trail_id"),"pipelineId":row.get::<&str,_>("pipeline_id"),"pipelineRevision":row.get::<i64,_>("pipeline_revision"),"sequence":row.get::<i64,_>("sequence"),"operationName":node["operation"]["name"],"port":port}}),
    )
}

pub(super) fn output_contract(
    plan: &Value,
    node: &Value,
) -> Result<cube_core::operations::contracts::Contract> {
    let operation = node["operation"]["operation"]
        .as_str()
        .ok_or("corrupt_store")?;
    let contract = cube_core::operations::require(operation)
        .map_err(|_| "contract_mismatch")?
        .contract()
        .map_err(|_| "contract_mismatch")?;
    let digest = node["contractDigest"].as_str().ok_or("corrupt_store")?;
    let pinned = if node.get("contract").is_some() {
        &node["contract"]
    } else if plan["contractDigest"] == digest {
        &plan["contract"]
    } else {
        &plan["contracts"][digest]
    };
    if *pinned != serde_json::to_value(&contract).map_err(|_| "invalid_state")?
        || cube_core::hashing::sha256(&serde_json::to_vec(&contract).map_err(|_| "invalid_state")?)
            != digest
        || node["operation"]["operationVersion"] != contract.operation_version
    {
        return Err("contract_mismatch");
    }
    if contract.policy.private_input
        || contract.policy.private_output
        || !contract.policy.sensitive_parameters.is_empty()
    {
        return Err("retention_policy_unavailable");
    }
    Ok(contract)
}
