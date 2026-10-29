//! Validated, bounded native Pipeline definitions and dependency planning.
use crate::{Result, worker};
use cube_core::operations::contracts::Contract;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
use uuid::Uuid;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Input {
    pub name: String,
    pub semantic_type: String,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Position {
    pub x: u16,
    pub y: u16,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum Source {
    Input { name: String },
    Output { node_id: Uuid, port: String },
    Literal { value: Value },
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Node {
    pub id: Uuid,
    pub operation: String,
    pub operation_version: u32,
    pub parameters: Value,
    pub position: Position,
    pub inputs: BTreeMap<String, Source>,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Definition {
    pub format_version: u32,
    pub inputs: [Input; 1],
    pub nodes: Vec<Node>,
    pub output: Uuid,
}
impl Node {
    pub fn contract(&self) -> Result<Contract> {
        worker::validate_configuration(&self.operation, self.operation_version, &self.parameters)?;
        cube_core::operations::require(&self.operation)
            .map_err(|_| "unsupported_operation")?
            .contract()
            .map_err(|_| "contract_mismatch")
    }
}
impl Definition {
    pub fn parse(value: Value) -> Result<Self> {
        if value.to_string().len() > crate::protocol::contract().pipeline_definition_bytes {
            return Err("input_limit");
        }
        let d: Self = serde_json::from_value(value).map_err(|_| "unsupported_pipeline")?;
        d.validate()?;
        Ok(d)
    }
    pub fn node(&self, id: Uuid) -> Result<&Node> {
        self.nodes
            .iter()
            .find(|n| n.id == id)
            .ok_or("invalid_connection")
    }
    pub fn validate(&self) -> Result<()> {
        if self.format_version != 3
            || self.inputs[0].name != "source"
            || !["text", "image.samples"].contains(&self.inputs[0].semantic_type.as_str())
            || self.nodes.is_empty()
            || self.nodes.len() > crate::protocol::contract().pipeline_node_limit
        {
            return Err("unsupported_pipeline");
        }
        let mut ids = BTreeSet::new();
        for n in &self.nodes {
            if !ids.insert(n.id)
                || n.parameters.to_string().len()
                    > crate::protocol::contract().pipeline_parameter_bytes
                || n.position.x > crate::protocol::contract().pipeline_canvas_limit
                || n.position.y > crate::protocol::contract().pipeline_canvas_limit
            {
                return Err("invalid_pipeline");
            }
            let c = n.contract()?;
            if n.inputs.len() != c.inputs.len() {
                return Err("invalid_connection");
            }
            for port in &c.inputs {
                let source = n.inputs.get(&port.name).ok_or("invalid_connection")?;
                match source {
                    Source::Input { name } => {
                        if name != "source" || self.inputs[0].semantic_type != port.semantic_type {
                            return Err("contract_mismatch");
                        }
                    }
                    Source::Output {
                        node_id,
                        port: name,
                    } => {
                        let producer = self.node(*node_id)?.contract()?;
                        let from = producer
                            .outputs
                            .iter()
                            .find(|p| p.name == *name)
                            .ok_or("invalid_connection")?;
                        if from.semantic_type != port.semantic_type {
                            return Err("contract_mismatch");
                        }
                    }
                    Source::Literal { value } => {
                        // Validate the literal with the same Core schema used at execution.
                        let mut literal_contract = c.clone();
                        literal_contract.inputs.retain(|p| p.name == port.name);
                        literal_contract
                            .validate_inputs(&json!({port.name.clone():value}))
                            .map_err(|_| "contract_mismatch")?;
                        if value.to_string().len()
                            > crate::protocol::contract().pipeline_literal_bytes
                            || port.semantic_type == "image.samples"
                        {
                            return Err("input_limit");
                        }
                    }
                }
            }
        }
        self.node(self.output)?;
        let mut visiting = BTreeSet::new();
        let mut done = BTreeSet::new();
        let mut ordered = Vec::new();
        for node in &self.nodes {
            self.visit(node.id, &mut visiting, &mut done, &mut ordered)?;
        }
        Ok(())
    }
    fn visit(
        &self,
        id: Uuid,
        visiting: &mut BTreeSet<Uuid>,
        done: &mut BTreeSet<Uuid>,
        ordered: &mut Vec<Uuid>,
    ) -> Result<()> {
        if done.contains(&id) {
            return Ok(());
        }
        if !visiting.insert(id) {
            return Err("pipeline_cycle");
        }
        for source in self.node(id)?.inputs.values() {
            if let Source::Output { node_id, .. } = source {
                self.visit(*node_id, visiting, done, ordered)?;
            }
        }
        visiting.remove(&id);
        done.insert(id);
        ordered.push(id);
        Ok(())
    }
    pub fn schedule(&self) -> Result<Vec<Uuid>> {
        let mut ordered = Vec::new();
        self.visit(
            self.output,
            &mut BTreeSet::new(),
            &mut BTreeSet::new(),
            &mut ordered,
        )?;
        Ok(ordered)
    }
    pub fn single(id: Uuid, operation: &str, version: u32, parameters: Value) -> Result<Self> {
        let c = cube_core::operations::require(operation)
            .map_err(|_| "unsupported_operation")?
            .contract()
            .map_err(|_| "contract_mismatch")?;
        let primary = c
            .inputs
            .iter()
            .find(|p| ["text", "image.samples"].contains(&p.semantic_type.as_str()))
            .ok_or("contract_mismatch")?;
        let mut inputs = BTreeMap::from([(
            primary.name.clone(),
            Source::Input {
                name: "source".into(),
            },
        )]);
        if operation == "esolang.brainfuck.execute" {
            inputs.insert("input".into(), Source::Literal { value: json!([]) });
        }
        let d = Self {
            format_version: 3,
            inputs: [Input {
                name: "source".into(),
                semantic_type: primary.semantic_type.clone(),
            }],
            nodes: vec![Node {
                id,
                operation: operation.into(),
                operation_version: version,
                parameters,
                position: Position { x: 80, y: 120 },
                inputs,
            }],
            output: id,
        };
        d.validate()?;
        Ok(d)
    }
}
