//! Versioned named-port transport. File and artifact capabilities resolve locally.
use super::{domains, execute, storage, types::*};
use crate::{AppState, workspace};
use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, State},
    routing::{get, post},
};
use cube_core::operations::contracts::{self, Contract};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{collections::BTreeMap, sync::Arc};
use tokio::sync::watch;
use uuid::Uuid;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub(super) enum Binding {
    Literal {
        value: Value,
    },
    File {
        store: workspace::Store,
        path: String,
        representation: Representation,
    },
    Artifact {
        id: Uuid,
        port: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        selection: Option<super::projection::Projection>,
    },
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Invocation {
    pub schema_version: u32,
    pub id: Uuid,
    pub operation: String,
    pub operation_version: u32,
    pub inputs: BTreeMap<String, Binding>,
    pub parameters: Value,
    #[serde(default)]
    pub exploration: Option<crate::persistence::explorations::Context>,
    #[serde(default)]
    pub retain_private_output: bool,
    #[serde(default = "timeout")]
    pub timeout_ms: u64,
}
fn timeout() -> u64 {
    TIMEOUT_MS
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Worker {
    operation: String,
    operation_version: u32,
    inputs: Value,
    parameters: Value,
    #[serde(default)]
    encoded_images: Vec<String>,
}
pub(super) fn routes() -> Router<AppState> {
    Router::new()
        .route("/lab/v2/operations", get(catalog))
        .route("/lab/v2/execute", post(invoke))
        .layer(DefaultBodyLimit::max(256 * 1024))
}
async fn catalog() -> Result<Json<Value>> {
    static CATALOG: tokio::sync::OnceCell<Result<Value>> = tokio::sync::OnceCell::const_new();
    let value = CATALOG
        .get_or_init(|| async {
            tokio::task::spawn_blocking(build_catalog)
                .await
                .map_err(|_| Error::storage())?
        })
        .await
        .clone()?;
    Ok(Json(value))
}
fn build_catalog() -> Result<Value> {
    let document = contracts::document()?;
    let operations = document
        .operations
        .into_iter()
        .map(|contract| {
            let executable = domains::supported(&contract.operation);
            json!({"contract":contract,"execution":{"available":executable,"adapter_version":2,
            "reason":(!executable).then_some("A tested local typed adapter is required."),
            "max_input_bytes":MAX_INPUT,"max_output_bytes":MAX_OUTPUT,"max_parameter_bytes":MAX_INLINE,"budget_representation":"json_wire_bytes","timeout_ms":TIMEOUT_MS}})
        })
        .collect::<Vec<_>>();
    Ok(
        json!({"schema_version":2,"contract_schema_version":1,"core_version":execute::core_version(),"operations":operations}),
    )
}
fn contract(operation: &str, version: u32) -> Result<Contract> {
    let descriptor = execute::descriptor(operation)?;
    let contract = descriptor.contract()?;
    if contract.operation_version != version {
        return Err(Error::new(
            "contract_mismatch",
            "Refresh the operation contract before executing.",
        ));
    }
    if !domains::supported(operation) {
        return Err(Error::new(
            "unsupported_operation",
            "This operation has no validated local typed adapter.",
        ));
    }
    Ok(contract)
}
fn checked(
    value: std::result::Result<(), cube_core::errors::OperationError>,
    operation: &str,
    section: &str,
) -> Result<()> {
    value.map_err(|error|{
        let mut error=Error::from(error);
        error.operation=Some(operation.into());
        error.parameter=Some(section.into());
        if section=="parameters" && error.code=="invalid_input" {error.code="invalid_parameter".into();}
        if section=="outputs" && error.code!="resource_limit" {
            error.code="contract_mismatch".into();
            error.message="The Core result does not match its published output contract; no result was saved.".into();
        }
        error
    })
}
pub(super) fn run(mut worker: Worker) -> Result<WorkerOutput> {
    let contract = contract(&worker.operation, worker.operation_version)?;
    execute::json_bytes(&worker.inputs, MAX_INPUT)?;
    execute::json_bytes(&worker.parameters, MAX_INLINE)?;
    // Only the trusted file resolver can mark encoded image inputs. Decoding
    // remains in this bounded child process, before logical-schema validation.
    if worker.encoded_images.len() > 32 {
        return Err(Error::limit());
    }
    for name in &worker.encoded_images {
        if !contract
            .inputs
            .iter()
            .any(|p| p.name == *name && p.semantic_type == "image.samples")
        {
            return Err(Error::invalid());
        }
        let bytes = domains::bytes(&worker.inputs[name])?;
        let image = cube_core::image::Image::decode(&bytes)?;
        worker.inputs[name] = serde_json::to_value(contracts::values::Image::from(&image))
            .map_err(|_| Error::invalid())?;
    }
    execute::json_bytes(&worker.inputs, MAX_INPUT)?;
    checked(
        contract.validate_inputs(&worker.inputs),
        &worker.operation,
        "inputs",
    )?;
    checked(
        contract.validate_parameters(&worker.parameters),
        &worker.operation,
        "parameters",
    )?;
    let parameters = worker
        .parameters
        .as_object_mut()
        .ok_or_else(Error::invalid)?;
    if let Some(properties) = contract
        .parameters
        .as_value()
        .get("properties")
        .and_then(Value::as_object)
    {
        for (name, schema) in properties {
            if let Some(default) = schema.get("default") {
                parameters
                    .entry(name.clone())
                    .or_insert_with(|| default.clone());
            }
        }
    }
    let output = contracts::canonical_value(domains::execute(
        &worker.operation,
        worker.inputs,
        worker.parameters.clone(),
    )?);
    finish(&contract, &worker.parameters, output)
}
// Filesystem metadata is already bounded to twelve header bytes. The trusted
// capability resolver calls Core directly and reuses the same result validation.
fn finish(
    contract: &Contract,
    effective_parameters: &Value,
    output: Value,
) -> Result<WorkerOutput> {
    let output = contracts::canonical_value(output);
    checked(
        contract.validate_outputs(&output),
        &contract.operation,
        "outputs",
    )?;
    let bytes = execute::json_bytes(&output, MAX_OUTPUT)?;
    // Private parameters stay ephemeral. Ordinary effective parameters are replayable.
    let parameters = effective_parameters
        .as_object()
        .ok_or_else(Error::invalid)?
        .iter()
        .map(|(name, value)| {
            if contract.policy.sensitive_parameters.contains(name) {
                json!({"name":name,"redacted":true,"replay":"required"})
            } else {
                json!({"name":name,"value":value})
            }
        })
        .collect();
    let ports=contract.outputs.iter().map(|p| Ok(json!({"name":p.name,"semantic_type":p.semantic_type,"schema_version":contract.schema_version,"operation_version":contract.operation_version,"schema_sha256":cube_core::hashing::sha256(&execute::json_bytes(&p.schema,128*1024)?)}))).collect::<Result<Vec<_>>>()?;
    Ok(WorkerOutput {
        data: Data::new(
            Representation::Structured,
            "operation.outputs",
            &bytes,
            json!({"schema_version":2,"ports":ports,"private_inputs_redacted":contract.policy.private_input}),
        ),
        parameters,
    })
}
pub(super) async fn resolve(
    app: &AppState,
    storage: Arc<storage::Storage>,
    binding: Binding,
) -> Result<(Value, InputRecord)> {
    match binding {
        Binding::Literal { value } => {
            let bytes = execute::json_bytes(&value, MAX_INLINE)?;
            Ok((
                value,
                InputRecord {
                    reference: json!({"type":"literal","retained":false,"digest_representation":"json_wire_value"}),
                    semantic_type: String::new(),
                    byte_size: bytes.len(),
                    sha256: cube_core::hashing::sha256(&bytes),
                },
            ))
        }
        Binding::File {
            store,
            path,
            representation,
        } => {
            let (data, mut record) = super::input(
                app,
                storage,
                Input::File {
                    store,
                    path,
                    representation,
                },
            )
            .await?;
            record.reference["digest_representation"] = json!("file_bytes");
            let bytes = data.bytes(MAX_INPUT)?;
            let value = match representation {
                Representation::Text => {
                    json!(std::str::from_utf8(&bytes).map_err(|_| Error::invalid())?)
                }
                Representation::Bytes => json!({"base64":data.base64}),
                Representation::Image => json!({"base64":data.base64}),
                Representation::Structured => return Err(Error::invalid()),
            };
            Ok((value, record))
        }
        Binding::Artifact {
            id,
            port,
            selection,
        } => {
            if port.is_empty() || port.len() > 128 {
                return Err(Error::invalid());
            }
            tokio::task::spawn_blocking(move || {
                let (record,bytes)=storage.load(id)?;
                if record.semantic_type!="operation.outputs" {return Err(Error::new("contract_mismatch","Select a named output from a typed execution; legacy results require an explicit conversion."));}
                let value:Value=serde_json::from_slice(&bytes).map_err(|_|Error::storage())?;
                let metadata=record.metadata["ports"].as_array().and_then(|ports|ports.iter().find(|p|p["name"]==port)).ok_or_else(Error::storage)?;
                let value=value.get(&port).cloned().ok_or_else(Error::storage)?;
                let semantic_type = selection.as_ref().map(|s| s.semantic_type().to_owned())
                    .or_else(|| metadata["semantic_type"].as_str().map(str::to_owned)).ok_or_else(Error::storage)?;
                let value = match &selection {Some(s) => s.select(&value)?, None => value};
                let bytes=execute::json_bytes(&value,MAX_INPUT)?;
                let mut reference=json!({"type":"artifact","id":id,"port":port,"digest_representation":"json_wire_value"});
                if let Some(selection)=selection {reference["selection"]=json!(selection);}
                Ok((value,InputRecord {reference,semantic_type,byte_size:bytes.len(),sha256:cube_core::hashing::sha256(&bytes)}))
            }).await.map_err(|_|Error::storage())?
        }
    }
}
async fn authorized_file_info(
    app: &AppState,
    binding: Binding,
) -> Result<(Value, InputRecord, Value)> {
    let Binding::File {
        store,
        path,
        representation: Representation::Bytes,
    } = binding
    else {
        return Err(Error::new(
            "contract_mismatch",
            "Select an authorized workspace file; literal paths and artifact references grant no filesystem access.",
        ));
    };
    let workspace = app
        .workspace
        .as_ref()
        .map_err(|e| Error::from(*e))?
        .clone()
        .ok_or_else(Error::invalid)?;
    let permit = workspace::permit(&workspace).await?;
    tokio::task::spawn_blocking(move || {
        let _permit=permit;
        let mut file=workspace.file(store,&path)?;
        let output=contracts::canonical_value(json!({"output":cube_core::forensics::inspect::file_info_reader(&mut file)?}));
        let digest=execute::json_bytes(&output,MAX_INLINE)?;
        let source=serde_json::to_value(store).map_err(|_|Error::invalid())?;
        let value=json!({"reference":format!("{}:{}",source.as_str().ok_or_else(Error::invalid)?,path)});
        Ok((value,InputRecord {reference:json!({"type":"file","store":store,"path":path,"representation":"bytes","digest_representation":"file_metadata_json"}),semantic_type:"file_path".into(),byte_size:digest.len(),sha256:cube_core::hashing::sha256(&digest)},output))
    }).await.map_err(|_|Error::storage())?
}
async fn invoke(
    State(app): State<AppState>,
    request: axum::extract::Request,
) -> Result<Json<Value>> {
    let request: Invocation = serde_json::from_value(super::request_json(request).await?)
        .map_err(|_| Error::invalid())?;
    if request.schema_version != 2
        || request.id.get_version_num() != 4
        || request.timeout_ms == 0
        || request.timeout_ms > TIMEOUT_MS
        || request.inputs.len() > 32
    {
        return Err(Error::invalid());
    }
    let contract = contract(&request.operation, request.operation_version)?;
    checked(
        contract.validate_parameters(&request.parameters),
        &request.operation,
        "parameters",
    )?;
    let storage = app.lab.storage(&app).await?;
    let permit = app
        .lab
        .slots
        .clone()
        .try_acquire_owned()
        .map_err(|_| Error::new("busy", "One Lab operation is already active."))?;
    let (cancel, rx) = watch::channel(false);
    *app.lab.active.lock().map_err(|_| Error::storage())? =
        Some((request.id, cancel.clone(), false));
    let _cancel = super::CancelOnDrop(cancel);
    let (reply, done) = tokio::sync::oneshot::channel();
    tokio::spawn(async move {
        let _permit = permit;
        let _active = super::ActiveGuard {
            state: app.lab.clone(),
            id: request.id,
        };
        let result = tracked_job(app, storage, request, contract, rx).await;
        let _ = reply.send(result);
    });
    Ok(Json(done.await.map_err(|_| Error::storage())??))
}
async fn tracked_job(
    app: AppState,
    storage: Arc<storage::Storage>,
    request: Invocation,
    contract: Contract,
    cancellation: watch::Receiver<bool>,
) -> Result<Value> {
    let tracking = if let Some(context) = &request.exploration {
        let db = app.database().await?.ok_or_else(Error::storage)?;
        db.begin_exploration_run(
            context,
            request.id,
            &request.operation,
            request.operation_version,
            &serde_json::to_value(&request.inputs).map_err(|_| Error::invalid())?,
            &request.parameters,
        )
        .await?;
        Some(db)
    } else {
        None
    };
    let id = request.id;
    let result = job(app, storage, request, contract, cancellation).await;
    if let Some(db) = tracking {
        db.finish_exploration_run(id, &result).await?;
    }
    result
}
async fn job(
    app: AppState,
    storage: Arc<storage::Storage>,
    request: Invocation,
    contract: Contract,
    mut cancellation: watch::Receiver<bool>,
) -> Result<Value> {
    let mut inputs = serde_json::Map::new();
    let mut records = Vec::new();
    let mut encoded_images = Vec::new();
    let mut file_info_output = None;
    for (name, binding) in request.inputs {
        let port = contract
            .inputs
            .iter()
            .find(|p| p.name == name)
            .ok_or_else(Error::invalid)?;
        if request.operation == "file.info.inspect" {
            let (value, mut record, output) = authorized_file_info(&app, binding).await?;
            record.reference["input_port"] = json!(name);
            record.reference["schema_sha256"] = json!(cube_core::hashing::sha256(
                &execute::json_bytes(&port.schema, 128 * 1024)?
            ));
            inputs.insert(name, value);
            records.push(record);
            file_info_output = Some(output);
            continue;
        }
        if let Binding::File { representation, .. } = &binding {
            if *representation == Representation::Image {
                encoded_images.push(name.clone());
            }
            let semantic = match representation {
                Representation::Text => "text",
                Representation::Bytes => "bytes",
                Representation::Image => "image.samples",
                Representation::Structured => return Err(Error::invalid()),
            };
            if semantic != port.semantic_type
                && !(semantic == "bytes" && port.semantic_type == "byte_reader")
            {
                return Err(Error::new(
                    "contract_mismatch",
                    "Select the exact input representation or add an explicit conversion.",
                ));
            }
        }
        let (value, mut record) = resolve(&app, storage.clone(), binding).await?;
        if !record.semantic_type.is_empty()
            && record.semantic_type != port.semantic_type
            && !(record.semantic_type == "bytes" && port.semantic_type == "byte_reader")
        {
            return Err(Error::new(
                "contract_mismatch",
                "Connect matching semantic ports or add an explicit conversion.",
            ));
        }
        record.semantic_type = port.semantic_type.clone();
        record.reference["input_port"] = json!(name);
        record.reference["schema_sha256"] = json!(cube_core::hashing::sha256(
            &execute::json_bytes(&port.schema, 128 * 1024)?
        ));
        records.push(record);
        inputs.insert(name, value);
    }
    let inputs = Value::Object(inputs);
    if encoded_images.is_empty() {
        checked(
            contract.validate_inputs(&inputs),
            &request.operation,
            "inputs",
        )?;
    }
    if *cancellation.borrow() {
        return Err(Error::cancelled());
    }
    let output = if let Some(output) = file_info_output {
        finish(&contract, &request.parameters, output)?
    } else {
        let wire = execute::json_bytes(
            &json!({"typed":Worker {operation:request.operation.clone(),operation_version:request.operation_version,inputs,parameters:request.parameters,encoded_images}}),
            MAX_WIRE,
        )?;
        super::worker_output(wire, request.timeout_ms, &mut cancellation).await?
    };
    if *cancellation.borrow() {
        return Err(Error::cancelled());
    }
    if contract.policy.private_input {
        records.clear();
    }
    let provenance = Provenance {
        operation: request.operation,
        adapter_api: 2,
        pipeline_adapter: contract.pipeline_adapter,
        engine_api: cube_core::pipeline::ENGINE_API,
        core_version: execute::core_version(),
        parameters: output.parameters.clone(),
        inputs: records,
        created_ms: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|_| Error::invalid())?
            .as_millis()
            .try_into()
            .map_err(|_| Error::invalid())?,
    };
    if contract.policy.private_output && !request.retain_private_output {
        let bytes = output.data.bytes(MAX_OUTPUT)?;
        let record = storage.record(request.id, output, provenance, &bytes)?;
        return super::response(record, &bytes, false);
    }
    {
        let mut active = app.lab.active.lock().map_err(|_| Error::storage())?;
        if *cancellation.borrow() {
            return Err(Error::cancelled());
        }
        if let Some((_, _, committing)) = active.as_mut().filter(|(id, _, _)| *id == request.id) {
            *committing = true;
        }
    }
    tokio::task::spawn_blocking(move || {
        let record = storage.save(request.id, output, provenance)?;
        let (_, bytes) = storage.load(request.id)?;
        super::response(record, &bytes, true)
    })
    .await
    .map_err(|_| Error::storage())?
}
