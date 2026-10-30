//! Closed reads for the context attached to a trusted Cube CLI invocation.
use super::{
    Context, pipelines, queries,
    storage::{self, Owned},
    to_value,
};
use crate::Result;
use serde::Deserialize;
use serde_json::Value;
use uuid::Uuid;

#[derive(Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Read {
    Asset { asset_id: Uuid, revision: i64 },
    Pipeline { pipeline_id: Uuid, revision: i64 },
}
pub async fn read(store: &mut Owned, context: &Context, command: Read) -> Result<Value> {
    let owner = store.identity.workspace_id;
    let connection = store.connection()?;
    queries::context(connection, owner, context).await?;
    let workspace = owner.to_string();
    let exploration = context.exploration_id.to_string();
    match command {
        Read::Asset { asset_id, revision } => {
            let asset = asset_id.to_string();
            let exists: i64 = sqlx::query_scalar("SELECT count(*) FROM blocks WHERE workspace_id=? AND exploration_id=? AND stage_id=? AND (asset_id=? OR caption_id=?)")
                .bind(&workspace).bind(&exploration).bind(context.stage_id.to_string()).bind(&asset).bind(&asset)
                .fetch_one(&mut *connection).await.map_err(storage::failure)?;
            if exists == 0 {
                return Err("not_found");
            }
            to_value(queries::asset(connection, &workspace, &exploration, &asset, revision).await?)
        }
        Read::Pipeline {
            pipeline_id,
            revision,
        } => to_value(
            pipelines::read(connection, &workspace, &pipeline_id.to_string(), revision).await?,
        ),
    }
}
