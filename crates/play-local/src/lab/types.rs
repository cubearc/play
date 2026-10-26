use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use cube_core::{
    operations::{Descriptor, ParameterValue},
    pipeline,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub const ADAPTER_API: u32 = 1;
pub const MAX_INLINE: usize = 64 * 1024;
pub const MAX_INPUT: usize = 4 * 1024 * 1024;
pub const MAX_OUTPUT: usize = 4 * 1024 * 1024;
pub const MAX_WIRE: usize = 12 * 1024 * 1024;
pub const INLINE_RESULT: usize = 16 * 1024;
pub const TIMEOUT_MS: u64 = 5000;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Error {
    pub code: String,
    pub message: String,
    pub operation: Option<String>,
    pub parameter: Option<String>,
    pub offset: Option<u64>,
}
pub type Result<T> = std::result::Result<T, Error>;
impl Error {
    pub fn new(code: &str, message: &str) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            operation: None,
            parameter: None,
            offset: None,
        }
    }
    pub fn invalid() -> Self {
        Self::new(
            "invalid_input",
            "Use the documented typed Lab request and bounded values.",
        )
    }
    pub fn limit() -> Self {
        Self::new(
            "resource_limit",
            "The operation exceeds a Lab input, output, memory or work limit.",
        )
    }
    pub fn storage() -> Self {
        Self::new(
            "storage_unavailable",
            "Managed Lab storage is unavailable or inconsistent. Check permissions and free space; existing data was preserved.",
        )
    }
    pub fn cancelled() -> Self {
        Self::new(
            "cancelled",
            "The operation was cancelled; its worker was terminated.",
        )
    }
}
impl From<crate::workspace::FileError> for Error {
    fn from(value: crate::workspace::FileError) -> Self {
        Self::new(value.1, value.2)
    }
}
impl From<cube_core::errors::OperationError> for Error {
    fn from(value: cube_core::errors::OperationError) -> Self {
        // Core categories/context survive; responses never echo arbitrary parser values.
        let mut error = Self::new(
            value.code.as_str(),
            match value.code {
                cube_core::errors::ErrorCode::ResourceLimit => {
                    "The operation exceeds its documented processing budget."
                }
                cube_core::errors::ErrorCode::UnsupportedFormat => {
                    "This representation is unsupported by the selected Core operation."
                }
                cube_core::errors::ErrorCode::Cancelled => "The operation was cancelled.",
                _ => "Core rejected the input or parameters; consult the operation descriptor.",
            },
        );
        error.operation = value.operation.map(str::to_owned);
        error.parameter = value.parameter.map(str::to_owned);
        error.offset = value.offset;
        error
    }
}
impl From<pipeline::Error> for Error {
    fn from(value: pipeline::Error) -> Self {
        let operation = value.operation.clone();
        let parameter = value.parameter.clone();
        let mut error = Self::from(*value.error);
        error.operation = operation;
        error.parameter = parameter;
        error
    }
}
impl IntoResponse for Error {
    fn into_response(self) -> Response {
        let status = match self.code.as_str() {
            "resource_limit" => StatusCode::PAYLOAD_TOO_LARGE,
            "busy" => StatusCode::TOO_MANY_REQUESTS,
            "unknown_operation" | "result_missing" => StatusCode::NOT_FOUND,
            "path_denied" | "symlink_denied" => StatusCode::FORBIDDEN,
            "unsupported_operation" | "contract_mismatch" | "cancelled" | "commit_started" => {
                StatusCode::CONFLICT
            }
            "timeout" => StatusCode::REQUEST_TIMEOUT,
            "storage_unavailable" | "worker_unavailable" => StatusCode::SERVICE_UNAVAILABLE,
            _ => StatusCode::BAD_REQUEST,
        };
        (
            status,
            Json(serde_json::json!({"schema_version":1,"error":self})),
        )
            .into_response()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Binding {
    pub name: String,
    pub value: pipeline::Parameter,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Representation {
    Bytes,
    Text,
    Image,
    Structured,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum Input {
    Text {
        data: String,
    },
    Bytes {
        base64: String,
    },
    File {
        store: crate::workspace::Store,
        path: String,
        representation: Representation,
    },
    Artifact {
        id: Uuid,
    },
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    pub id: Uuid,
    pub operation: String,
    pub version: u32,
    pub input: Input,
    #[serde(default)]
    pub other: Option<Input>,
    #[serde(default)]
    pub parameters: Vec<Binding>,
    #[serde(default = "default_timeout")]
    pub timeout_ms: u64,
    #[serde(default = "default_output")]
    pub max_output_bytes: usize,
    #[serde(default = "default_retain")]
    pub retain_result: bool,
}
fn default_retain() -> bool {
    true
}
fn default_timeout() -> u64 {
    TIMEOUT_MS
}
fn default_output() -> usize {
    MAX_OUTPUT
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Data {
    pub representation: Representation,
    pub semantic_type: String,
    pub base64: String,
    #[serde(default)]
    pub metadata: serde_json::Value,
}
impl Data {
    pub fn bytes(&self, maximum: usize) -> Result<Vec<u8>> {
        if self.base64.len() > maximum.div_ceil(3) * 4 {
            return Err(Error::limit());
        }
        let bytes =
            cube_core::encoding::base64_decode(&self.base64).map_err(|_| Error::invalid())?;
        if bytes.len() > maximum {
            return Err(Error::limit());
        }
        Ok(bytes)
    }
    pub fn new(
        representation: Representation,
        semantic: &str,
        bytes: &[u8],
        metadata: serde_json::Value,
    ) -> Self {
        Self {
            representation,
            semantic_type: semantic.into(),
            base64: cube_core::encoding::base64_encode(bytes),
            metadata,
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InputRecord {
    pub reference: serde_json::Value,
    pub semantic_type: String,
    pub byte_size: usize,
    pub sha256: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Provenance {
    pub operation: String,
    pub adapter_api: u32,
    pub pipeline_adapter: Option<u32>,
    pub engine_api: u32,
    pub core_version: String,
    pub parameters: Vec<serde_json::Value>,
    pub inputs: Vec<InputRecord>,
    pub created_ms: u64,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Record {
    pub id: Uuid,
    pub workspace_id: Uuid,
    pub representation: Representation,
    pub semantic_type: String,
    pub byte_size: usize,
    pub sha256: String,
    pub metadata: serde_json::Value,
    pub provenance: Provenance,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WorkerRequest {
    pub request: Request,
    pub input: Data,
    pub other: Option<Data>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WorkerOutput {
    pub data: Data,
    pub parameters: Vec<serde_json::Value>,
}

pub fn parameters(descriptor: &Descriptor, supplied: &[Binding]) -> Result<Vec<Binding>> {
    if supplied.len() > 8 {
        return Err(Error::limit());
    }
    let lists: Vec<Vec<&str>> = supplied
        .iter()
        .map(|binding| match &binding.value {
            pipeline::Parameter::TextList { values } => values.iter().map(String::as_str).collect(),
            _ => Vec::new(),
        })
        .collect();
    let borrowed: Vec<_> = supplied
        .iter()
        .zip(&lists)
        .map(|(binding, list)| {
            use pipeline::Parameter as P;
            let value = match &binding.value {
                P::Integer(v) => ParameterValue::Integer(*v),
                P::Boolean(v) => ParameterValue::Boolean(*v),
                P::Text(v) => ParameterValue::Text(v),
                P::Bytes { data } => ParameterValue::Bytes(data),
                P::UnsignedList { values } => ParameterValue::UnsignedList(values),
                P::TextList { .. } => ParameterValue::TextList(list),
            };
            (binding.name.as_str(), value)
        })
        .collect();
    descriptor.validate(&borrowed).map_err(|failure| {
        let mut error = Error::new("invalid_parameter", failure.reason);
        error.operation = Some(descriptor.id.into());
        error.parameter = Some(
            if descriptor
                .parameters
                .iter()
                .any(|p| p.name == failure.parameter)
            {
                failure.parameter
            } else {
                "parameters".into()
            },
        );
        error
    })?;
    let mut result = supplied.to_vec();
    for parameter in descriptor.parameters {
        if result.iter().any(|binding| binding.name == parameter.name) {
            continue;
        }
        let Some(value) = parameter.default else {
            continue;
        };
        let value = match value {
            ParameterValue::Integer(v) => pipeline::Parameter::Integer(v),
            ParameterValue::Unsigned(v) => {
                pipeline::Parameter::Integer(v.try_into().map_err(|_| Error::invalid())?)
            }
            ParameterValue::Boolean(v) => pipeline::Parameter::Boolean(v),
            ParameterValue::Text(v) => pipeline::Parameter::Text(v.into()),
            ParameterValue::Bytes(v) => pipeline::Parameter::Bytes { data: v.into() },
            ParameterValue::TextList(v) => pipeline::Parameter::TextList {
                values: v.iter().map(|v| (*v).into()).collect(),
            },
            ParameterValue::UnsignedList(v) => {
                pipeline::Parameter::UnsignedList { values: v.into() }
            }
        };
        result.push(Binding {
            name: parameter.name.into(),
            value,
        });
    }
    result.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(result)
}
