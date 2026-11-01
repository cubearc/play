//! Exact immutable Core schemas are stored once, outside bounded Run plans.
use super::{model::Context, mutations, storage};
use crate::{Result, protocol};
use cube_core::operations::{contracts::Contract, execution};
use serde_json::{Value, json};
use sqlx::SqliteConnection;

pub fn digest(contract: &Contract) -> Result<String> {
    Ok(cube_core::hashing::sha256(
        &serde_json::to_vec(contract).map_err(|_| "invalid_state")?,
    ))
}
pub fn reference(contract: &Contract) -> Result<Value> {
    let executable =
        execution::describe(&contract.operation, Default::default()).map_err(|e| e.0)?;
    let ports = |ports: &[cube_core::operations::contracts::PortContract],
                 representations: &std::collections::BTreeMap<
        String,
        execution::Representation,
    >| {
        ports.iter().map(|port| Ok(json!({"name":port.name,"semantic_type":port.semantic_type,"representation":representations.get(&port.name).ok_or("contract_mismatch")?}))).collect::<Result<Vec<_>>>()
    };
    Ok(
        json!({"kind":"snapshot","digest":digest(contract)?,"operation":contract.operation,"operation_version":contract.operation_version,"inputs":ports(&contract.inputs,&executable.inputs)?,"outputs":ports(&contract.outputs,&executable.outputs)?}),
    )
}
pub async fn retain(
    connection: &mut SqliteConnection,
    context: &Context,
    contract: &Contract,
) -> Result<()> {
    let document = serde_json::to_string(contract).map_err(|_| "invalid_state")?;
    if document.len() > protocol::contract().frame_bytes {
        return Err("contract_snapshot_limit");
    }
    let digest = digest(contract)?;
    let workspace = context.workspace_id.to_string();
    let exploration = context.exploration_id.to_string();
    let existing: Option<String> = sqlx::query_scalar("SELECT document FROM contract_snapshots WHERE workspace_id=? AND exploration_id=? AND digest=?")
        .bind(&workspace).bind(&exploration).bind(&digest).fetch_optional(&mut *connection).await.map_err(storage::failure)?;
    if let Some(existing) = existing {
        return if existing == document {
            Ok(())
        } else {
            Err("corrupt_store")
        };
    }
    mutations::bytes(connection, &workspace, &exploration, document.len()).await?;
    sqlx::query("INSERT INTO contract_snapshots VALUES(?,?,?,?)")
        .bind(workspace)
        .bind(exploration)
        .bind(digest)
        .bind(document)
        .execute(connection)
        .await
        .map_err(storage::failure)?;
    Ok(())
}
