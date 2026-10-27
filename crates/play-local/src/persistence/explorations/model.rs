//! Reusable graph configuration is separate from immutable execution evidence.
use crate::workspace::{self, Result, Store};
use cube_core::operations;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Context {
    pub id: Uuid,
    pub revision: i64,
    pub step: Uuid,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Scope {
    pub challenge: String,
    pub stage: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub(crate) enum Source {
    Literal {
        value: Value,
    },
    File {
        store: Store,
        path: String,
        representation: String,
    },
    Artifact {
        id: Uuid,
        port: String,
    },
    Step {
        step: Uuid,
        port: String,
    },
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Step {
    pub id: Uuid,
    pub operation: String,
    pub operation_version: u32,
    pub inputs: BTreeMap<String, Source>,
    pub parameters: Value,
    #[serde(default)]
    pub private_parameters: Vec<String>,
    pub position: [i32; 2],
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Definition {
    pub schema_version: u32,
    pub id: Uuid,
    pub revision: i64,
    pub title: String,
    pub hypothesis: String,
    pub scope: Option<Scope>,
    pub steps: Vec<Step>,
}
impl Definition {
    /// Check persisted structure without requiring the current operation version.
    pub(super) fn validate_archive(&self) -> Result<()> {
        if self.schema_version != 1
            || self.id.get_version_num() != 4
            || self.revision < 1
            || self.title.trim().is_empty()
            || self.title.len() > 512
            || self.title.contains('\0')
            || self.hypothesis.len() > 8192
            || self.steps.len() > 64
            || serde_json::to_vec(self)
                .map_err(|_| super::invalid())?
                .len()
                > 65536
        {
            return Err(super::invalid());
        }
        let mut ids = BTreeSet::new();
        for step in &self.steps {
            if step.id.get_version_num() != 4
                || !ids.insert(step.id)
                || step.operation.is_empty()
                || step.operation.len() > 128
                || step.operation_version == 0
                || step.inputs.len() > 32
                || !step.parameters.is_object()
                || step.private_parameters.len() > 32
                || step
                    .position
                    .iter()
                    .any(|p| !(-100000..=100000).contains(p))
            {
                return Err(super::invalid());
            }
            for (port, source) in &step.inputs {
                if port.is_empty() || port.len() > 128 {
                    return Err(super::invalid());
                }
                match source {
                    Source::File {
                        path,
                        representation,
                        ..
                    } => {
                        workspace::validate(path, false)?;
                        if !matches!(representation.as_str(), "text" | "bytes" | "image") {
                            return Err(super::invalid());
                        }
                    }
                    Source::Artifact { id, port } | Source::Step { step: id, port } => {
                        if id.get_version_num() != 4 || port.is_empty() || port.len() > 128 {
                            return Err(super::invalid());
                        }
                    }
                    Source::Literal { .. } => {}
                }
            }
        }
        Ok(())
    }
    pub(super) fn validate(&self) -> Result<()> {
        if self.schema_version != 1
            || self.id.get_version_num() != 4
            || self.revision < 0
            || self.revision == i64::MAX
            || self.title.trim().is_empty()
            || self.title.len() > 512
            || self.title.contains('\0')
            || self.hypothesis.len() > 8192
            || self.steps.len() > 64
        {
            return Err(super::invalid());
        }
        let mut ids = BTreeMap::new();
        for step in &self.steps {
            if step.id.get_version_num() != 4
                || ids.insert(step.id, step).is_some()
                || step.inputs.len() > 32
                || step.private_parameters.len() > 32
                || step
                    .position
                    .iter()
                    .any(|p| !(-100000..=100000).contains(p))
            {
                return Err(super::invalid());
            }
            let contract = operations::describe(&step.operation)
                .ok_or_else(super::invalid)?
                .contract()
                .map_err(|_| super::invalid())?;
            if step.operation_version != contract.operation_version {
                return Err(super::invalid());
            }
            let parameters = step.parameters.as_object().ok_or_else(super::invalid)?;
            // Save ordinary replay parameters only. Sensitive values are entered
            // for a particular run and never included in the configuration.
            if contract
                .policy
                .sensitive_parameters
                .iter()
                .any(|name| parameters.contains_key(name))
                || step
                    .private_parameters
                    .iter()
                    .collect::<BTreeSet<_>>()
                    .len()
                    != step.private_parameters.len()
            {
                return Err(super::invalid());
            }
            let schema = contract.parameters.as_value();
            for name in &step.private_parameters {
                if !contract.policy.sensitive_parameters.contains(name)
                    || parameters.contains_key(name)
                {
                    return Err(super::invalid());
                }
            }
            if schema.get("properties").is_none() {
                if !parameters.is_empty() {
                    contract
                        .validate_parameters(&step.parameters)
                        .map_err(|_| super::invalid())?;
                }
            } else {
                for (name, value) in parameters {
                    let property = schema
                        .get("properties")
                        .and_then(|p| p.get(name))
                        .ok_or_else(super::invalid)?;
                    // Validate a property in its complete standalone definition scope.
                    let mut partial = schema.clone();
                    partial["required"] = serde_json::json!([]);
                    let _ = property;
                    let schema = serde_json::from_value(partial).map_err(|_| super::invalid())?;
                    operations::contracts::validate_value(
                        &schema,
                        &serde_json::json!({name:value}),
                    )
                    .map_err(|_| super::invalid())?;
                }
            }
            for (name, source) in &step.inputs {
                let port = contract
                    .inputs
                    .iter()
                    .find(|p| p.name == *name)
                    .ok_or_else(super::invalid)?;
                match source {
                    Source::Literal { value } => {
                        if contract.policy.private_input {
                            return Err(super::invalid());
                        }
                        operations::contracts::validate_value(&port.schema, value)
                            .map_err(|_| super::invalid())?
                    }
                    Source::File {
                        path,
                        representation,
                        ..
                    } => {
                        workspace::validate(path, false)?;
                        let semantic = match representation.as_str() {
                            "text" => "text",
                            "bytes" => "bytes",
                            "image" => "image.samples",
                            _ => return Err(super::invalid()),
                        };
                        if port.semantic_type != semantic
                            && !(semantic == "bytes"
                                && (port.semantic_type == "byte_reader"
                                    || (step.operation == "file.info.inspect"
                                        && port.semantic_type == "file_path")))
                        {
                            return Err(super::invalid());
                        }
                    }
                    Source::Artifact { id, port } => {
                        if id.get_version_num() != 4 || port.is_empty() || port.len() > 128 {
                            return Err(super::invalid());
                        }
                    }
                    Source::Step { step, port } => {
                        if step.get_version_num() != 4 || port.is_empty() || port.len() > 128 {
                            return Err(super::invalid());
                        }
                    }
                }
            }
        }
        fn visit(
            id: Uuid,
            ids: &BTreeMap<Uuid, &Step>,
            active: &mut BTreeSet<Uuid>,
            done: &mut BTreeSet<Uuid>,
        ) -> Result<()> {
            if done.contains(&id) {
                return Ok(());
            }
            if !active.insert(id) {
                return Err(super::invalid());
            }
            let step = ids.get(&id).ok_or_else(super::invalid)?;
            let contract = operations::describe(&step.operation)
                .ok_or_else(super::invalid)?
                .contract()
                .map_err(|_| super::invalid())?;
            for (input, source) in &step.inputs {
                if let Source::Step {
                    step: upstream,
                    port,
                } = source
                {
                    let producer = ids.get(upstream).ok_or_else(super::invalid)?;
                    let producer = operations::describe(&producer.operation)
                        .ok_or_else(super::invalid)?
                        .contract()
                        .map_err(|_| super::invalid())?;
                    let output = producer
                        .outputs
                        .iter()
                        .find(|p| p.name == *port)
                        .ok_or_else(super::invalid)?;
                    if !contract.accepts(input, &output.semantic_type) {
                        return Err(super::invalid());
                    }
                    visit(*upstream, ids, active, done)?;
                }
            }
            active.remove(&id);
            done.insert(id);
            Ok(())
        }
        let mut done = BTreeSet::new();
        for id in ids.keys() {
            visit(*id, &ids, &mut BTreeSet::new(), &mut done)?;
        }
        if serde_json::to_vec(self)
            .map_err(|_| super::invalid())?
            .len()
            > 65536
        {
            return Err(super::invalid());
        }
        Ok(())
    }
}
