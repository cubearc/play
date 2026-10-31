//! Explicit Trail input references; reusable Pipeline definitions never own assets.
use super::{
    blocks,
    model::{Asset, AssetRevision, Content, Context, PipelineRef},
    pipelines, queries, storage,
};
use crate::{Result, graph::Definition};
use sqlx::{Row, SqliteConnection};
use std::collections::BTreeMap;
use uuid::Uuid;

pub fn semantic_type(asset: &Asset) -> Result<&'static str> {
    match asset.content {
        Content::Text { .. } | Content::Page { .. } => Ok("text"),
        Content::Image => Ok("image.samples"),
        _ => Err("invalid_asset_type"),
    }
}
pub async fn read(
    c: &mut SqliteConnection,
    w: &str,
    e: &str,
    trail: &str,
) -> Result<BTreeMap<String, Asset>> {
    let rows=sqlx::query("SELECT slot,asset_id,asset_revision FROM trail_input_bindings WHERE workspace_id=? AND exploration_id=? AND trail_id=? ORDER BY slot").bind(w).bind(e).bind(trail).fetch_all(&mut *c).await.map_err(storage::failure)?;
    let mut result = BTreeMap::new();
    for r in rows {
        result.insert(
            r.get("slot"),
            queries::asset(c, w, e, r.get("asset_id"), r.get("asset_revision")).await?,
        );
    }
    Ok(result)
}
pub async fn validate(
    c: &mut SqliteConnection,
    ctx: &Context,
    pipeline: Option<&PipelineRef>,
    bindings: &BTreeMap<String, AssetRevision>,
) -> Result<()> {
    let Some(p) = pipeline else {
        return if bindings.is_empty() {
            Ok(())
        } else {
            Err("invalid_binding")
        };
    };
    let d = Definition::parse(
        pipelines::definition_value(
            c,
            &ctx.workspace_id.to_string(),
            &p.id.to_string(),
            p.revision,
        )
        .await?,
    )?;
    if bindings.len() != d.inputs.len() - 1 || bindings.contains_key("source") {
        return Err("invalid_binding");
    }
    for input in d.inputs.iter().skip(1) {
        let reference = bindings.get(&input.name).ok_or("invalid_binding")?;
        let id = reference.asset_id.to_string();
        blocks::source_active(c, ctx, &id).await?;
        let asset = queries::asset(
            c,
            &ctx.workspace_id.to_string(),
            &ctx.exploration_id.to_string(),
            &id,
            reference.revision,
        )
        .await?;
        if semantic_type(&asset)? != input.semantic_type {
            return Err("invalid_asset_type");
        }
    }
    Ok(())
}
pub async fn replace(
    c: &mut SqliteConnection,
    ctx: &Context,
    trail: Uuid,
    bindings: &BTreeMap<String, AssetRevision>,
) -> Result<()> {
    let w = ctx.workspace_id.to_string();
    let e = ctx.exploration_id.to_string();
    let t = trail.to_string();
    sqlx::query(
        "DELETE FROM trail_input_bindings WHERE workspace_id=? AND exploration_id=? AND trail_id=?",
    )
    .bind(&w)
    .bind(&e)
    .bind(&t)
    .execute(&mut *c)
    .await
    .map_err(storage::failure)?;
    for (slot, asset) in bindings {
        sqlx::query("INSERT INTO trail_input_bindings VALUES(?,?,?,?,?,?)")
            .bind(&w)
            .bind(&e)
            .bind(&t)
            .bind(slot)
            .bind(asset.asset_id.to_string())
            .bind(asset.revision)
            .execute(&mut *c)
            .await
            .map_err(storage::failure)?;
    }
    Ok(())
}
pub async fn copy_for(
    c: &mut SqliteConnection,
    ctx: &Context,
    copies: &[(&AssetRevision, &str)],
) -> Result<()> {
    let w = ctx.workspace_id.to_string();
    let e = ctx.exploration_id.to_string();
    for (original, copied) in copies {
        let trail: String=sqlx::query_scalar("SELECT id FROM trails WHERE workspace_id=? AND exploration_id=? AND stage_id=? AND source_id=? AND source_revision=? ORDER BY rowid LIMIT 1").bind(&w).bind(&e).bind(ctx.stage_id.to_string()).bind(original.asset_id.to_string()).bind(original.revision).fetch_one(&mut *c).await.map_err(storage::failure)?;
        let new_trail: String = sqlx::query_scalar(
            "SELECT id FROM trails WHERE workspace_id=? AND exploration_id=? AND source_id=?",
        )
        .bind(&w)
        .bind(&e)
        .bind(copied)
        .fetch_one(&mut *c)
        .await
        .map_err(storage::failure)?;
        let mut references = BTreeMap::new();
        for (slot, asset) in read(c, &w, &e, &trail).await? {
            let reference = if let Some((_, new_id)) = copies.iter().find(|(old, _)| {
                old.asset_id.to_string() == asset.id && old.revision == asset.revision
            }) {
                AssetRevision {
                    asset_id: Uuid::parse_str(new_id).map_err(|_| "corrupt_store")?,
                    revision: 1,
                }
            } else {
                AssetRevision {
                    asset_id: Uuid::parse_str(&asset.id).map_err(|_| "corrupt_store")?,
                    revision: asset.revision,
                }
            };
            references.insert(slot, reference);
        }
        replace(
            c,
            ctx,
            Uuid::parse_str(&new_trail).map_err(|_| "corrupt_store")?,
            &references,
        )
        .await?;
    }
    Ok(())
}
