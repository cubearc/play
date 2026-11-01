//! Private, bounded snapshots for a native save intent; no file authority or persistence.
use super::{exports::Payload, model::Context};
use crate::{Result, protocol};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    time::{Duration, Instant},
};
use uuid::Uuid;

struct Grant {
    context: Context,
    payload: Payload,
    expires: Instant,
}
#[derive(Default)]
pub(super) struct Grants(BTreeMap<Uuid, Grant>);
impl Grants {
    pub(super) fn issue(
        &mut self,
        context: Context,
        payload: Payload,
        now: Instant,
    ) -> Result<Value> {
        let limits = protocol::contract();
        self.0.retain(|_, grant| grant.expires > now);
        if self.0.len() >= limits.export_grants {
            return Err("export_limit");
        }
        if payload.bytes.len() > limits.export_bytes {
            return Err("output_limit");
        }
        let id = Uuid::new_v4();
        let result = json!({"exportId":id,"fileName":payload.name,"mime":payload.mime,"size":payload.bytes.len(),"sha256":cube_core::hashing::sha256(&payload.bytes)});
        self.0.insert(
            id,
            Grant {
                context,
                payload,
                expires: now + Duration::from_millis(limits.export_ms),
            },
        );
        Ok(result)
    }
    pub(super) fn read(
        &mut self,
        context: &Context,
        id: Uuid,
        offset: usize,
        now: Instant,
    ) -> Result<Value> {
        let grant = self.0.get(&id).ok_or("not_found")?;
        if &grant.context != context {
            return Err("not_found");
        }
        if grant.expires <= now {
            self.0.remove(&id);
            return Err("export_expired");
        }
        let bytes = &grant.payload.bytes;
        if offset > bytes.len() {
            return Err("invalid_command");
        }
        let end = (offset + protocol::contract().export_chunk_bytes).min(bytes.len());
        Ok(
            json!({"offset":offset,"base64":STANDARD.encode(&bytes[offset..end]),"done":end==bytes.len()}),
        )
    }
    pub(super) fn release(&mut self, context: &Context, id: Uuid) -> Result<()> {
        if self
            .0
            .get(&id)
            .is_some_and(|grant| &grant.context != context)
        {
            return Err("not_found");
        }
        self.0.remove(&id);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn context() -> Context {
        Context {
            workspace_id: Uuid::new_v4(),
            exploration_id: Uuid::new_v4(),
            stage_id: Uuid::new_v4(),
        }
    }
    fn payload(size: usize) -> Payload {
        Payload {
            name: "result.bin".into(),
            mime: "application/octet-stream",
            bytes: vec![42; size],
        }
    }
    #[test]
    fn grants_are_bounded_scoped_chunked_expiring_and_releasable() {
        let mut grants = Grants::default();
        let context = context();
        let foreign = super::tests::context();
        let now = Instant::now();
        let chunk = protocol::contract().export_chunk_bytes;
        let first = grants
            .issue(context.clone(), payload(chunk + 1), now)
            .unwrap();
        let id = serde_json::from_value(first["exportId"].clone()).unwrap();
        let part = grants.read(&context, id, 0, now).unwrap();
        assert_eq!(
            STANDARD
                .decode(part["base64"].as_str().unwrap())
                .unwrap()
                .len(),
            chunk
        );
        assert_eq!(part["done"], false);
        assert_eq!(grants.read(&context, id, chunk, now).unwrap()["done"], true);
        assert_eq!(
            grants.read(&context, id, chunk + 2, now),
            Err("invalid_command")
        );
        assert_eq!(grants.read(&foreign, id, 0, now), Err("not_found"));
        assert_eq!(grants.release(&foreign, id), Err("not_found"));
        grants.issue(context.clone(), payload(0), now).unwrap();
        assert_eq!(
            grants.issue(context.clone(), payload(1), now),
            Err("export_limit")
        );
        grants.release(&context, id).unwrap();
        grants.release(&context, id).unwrap();
        assert_eq!(
            grants.issue(
                context.clone(),
                payload(protocol::contract().export_bytes + 1),
                now
            ),
            Err("output_limit")
        );
        let grant = grants.issue(context.clone(), payload(0), now).unwrap();
        let id = serde_json::from_value(grant["exportId"].clone()).unwrap();
        assert_eq!(grants.read(&context, id, 0, now).unwrap()["base64"], "");
        assert_eq!(
            grants.read(
                &context,
                id,
                0,
                now + Duration::from_millis(protocol::contract().export_ms)
            ),
            Err("export_expired")
        );
    }
}
