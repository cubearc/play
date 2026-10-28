use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Context {
    pub workspace_id: Uuid,
    pub exploration_id: Uuid,
    pub stage_id: Uuid,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Command {
    List,
    Create {
        key: Uuid,
        title: String,
    },
    Open {
        workspace_id: Uuid,
    },
    CreateExploration {
        workspace_id: Uuid,
        key: Uuid,
        title: String,
    },
    OpenExploration {
        workspace_id: Uuid,
        exploration_id: Uuid,
        stage_id: Option<Uuid>,
    },
    CreateStage {
        workspace_id: Uuid,
        exploration_id: Uuid,
        key: Uuid,
        title: String,
    },
    SaveNote {
        context: Context,
        key: Uuid,
        note_id: Uuid,
        expected_revision: i64,
        text: String,
    },
    AddText {
        context: Context,
        key: Uuid,
        title: String,
        text: String,
    },
    AddImage {
        context: Context,
        key: Uuid,
        title: String,
        caption: String,
        png: String,
    },
    ReadImage {
        context: Context,
        asset_id: Uuid,
        revision: i64,
    },
}

impl Command {
    pub fn mutation_key(&self) -> Option<Uuid> {
        match self {
            Self::CreateExploration { key, .. }
            | Self::CreateStage { key, .. }
            | Self::SaveNote { key, .. }
            | Self::AddText { key, .. }
            | Self::AddImage { key, .. } => Some(*key),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Identity {
    pub product: String,
    pub generation: u32,
    pub workspace_id: Uuid,
    pub title: String,
    pub creation_key: Uuid,
    pub creation_digest: String,
}

#[derive(Serialize)]
pub struct Summary {
    pub id: String,
    pub title: String,
    pub revision: i64,
}

#[derive(Clone, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Content {
    Text { text: String },
    Image,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Asset {
    pub id: String,
    pub exploration_id: String,
    pub revision: i64,
    pub title: String,
    pub content: Content,
}

#[derive(Serialize)]
pub struct Block {
    pub id: String,
    pub asset: Asset,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub caption: Option<Asset>,
}

#[derive(Serialize)]
pub struct Note {
    pub id: String,
    pub revision: i64,
    pub text: String,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Pipeline {
    pub id: String,
    pub title: String,
    pub revision: i64,
    pub input: String,
    pub initial_operation: String,
    pub operation_version: u32,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Trail {
    pub id: String,
    pub workspace_id: String,
    pub exploration_id: String,
    pub stage_id: String,
    pub title: String,
    pub hypothesis: String,
    pub source: Asset,
    pub pipeline_id: Option<String>,
    pub pipeline: Option<Pipeline>,
}

#[derive(Serialize)]
pub struct Stage {
    pub id: String,
    pub title: String,
    pub revision: i64,
    pub introduction: String,
    pub blocks: Vec<Block>,
    pub note: Note,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Exploration {
    pub id: String,
    pub workspace_id: String,
    pub title: String,
    pub revision: i64,
    pub stages: Vec<Summary>,
    pub stage: Stage,
    pub trails: Vec<Trail>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct View {
    pub id: String,
    pub title: String,
    pub generation: u32,
    pub explorations: Vec<Summary>,
    pub pipelines: Vec<Pipeline>,
    pub selected_exploration_id: Option<String>,
    pub selected_stage_id: Option<String>,
}
