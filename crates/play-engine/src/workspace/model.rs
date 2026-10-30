use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Context {
    pub workspace_id: Uuid,
    pub exploration_id: Uuid,
    pub stage_id: Uuid,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct AssetRevision {
    pub asset_id: Uuid,
    pub revision: i64,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum BlockSource {
    Asset {
        asset: AssetRevision,
        caption: Option<AssetRevision>,
    },
    Note {
        note_id: Uuid,
        revision: i64,
    },
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum CandidateSelection {
    Asset {
        asset_id: Uuid,
        revision: i64,
    },
    RunOutput {
        run_id: Uuid,
        attempt_id: Uuid,
        port: String,
    },
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Command {
    SaveSolution {
        context: Context,
        key: Uuid,
        solution_id: Uuid,
        expected_revision: i64,
        candidate_revision: i64,
        title: String,
        explanation: String,
    },
    ListSolutions {
        context: Context,
    },
    GetSolution {
        context: Context,
        solution_id: Uuid,
        revision: Option<i64>,
    },
    SelectCandidate {
        context: Context,
        key: Uuid,
        expected_revision: i64,
        selection: CandidateSelection,
    },
    GetCandidate {
        context: Context,
        revision: Option<i64>,
    },
    CreatePipeline {
        workspace_id: Uuid,
        key: Uuid,
        title: String,
        definition: serde_json::Value,
    },
    RevisePipeline {
        workspace_id: Uuid,
        key: Uuid,
        pipeline_id: Uuid,
        expected_revision: i64,
        definition: serde_json::Value,
    },
    ForkPipeline {
        workspace_id: Uuid,
        key: Uuid,
        pipeline_id: Uuid,
        revision: i64,
        title: String,
    },
    GetPipeline {
        workspace_id: Uuid,
        pipeline_id: Uuid,
        revision: i64,
    },
    SaveTrail {
        context: Context,
        key: Uuid,
        trail_id: Uuid,
        expected_revision: i64,
        hypothesis: String,
        pipeline: Option<PipelineRef>,
        bindings: std::collections::BTreeMap<String, AssetRevision>,
    },
    AddPage {
        context: Context,
        key: Uuid,
        title: String,
        text: String,
    },
    AddAnnotation {
        context: Context,
        key: Uuid,
        title: String,
        text: String,
    },
    PinRunOutput {
        context: Context,
        key: Uuid,
        expected_revision: i64,
        run_id: Uuid,
        attempt_id: Uuid,
        port: String,
    },
    ArchiveBlock {
        context: Context,
        key: Uuid,
        block_id: Uuid,
        expected_revision: i64,
        source: BlockSource,
    },
    RestoreBlock {
        context: Context,
        key: Uuid,
        block_id: Uuid,
        expected_revision: i64,
        source: BlockSource,
    },
    DuplicateBlock {
        context: Context,
        key: Uuid,
        block_id: Uuid,
        expected_revision: i64,
        source: BlockSource,
    },
    List,
    View {
        workspace_id: Uuid,
    },
    Create {
        key: Uuid,
        title: String,
    },
    Open {
        workspace_id: Uuid,
    },
    ImportChallenge {
        workspace_id: Uuid,
        key: Uuid,
        package: String,
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
    ReviseText {
        context: Context,
        key: Uuid,
        block_id: Uuid,
        asset_id: Uuid,
        expected_revision: i64,
        text: String,
    },
    AddImage {
        context: Context,
        key: Uuid,
        title: String,
        caption: String,
        png: String,
    },
    AddFile {
        context: Context,
        key: Uuid,
        title: String,
        base64: String,
    },
    ListRuns {
        context: Context,
        trail_id: Uuid,
        before: Option<i64>,
    },
    RunDetail {
        context: Context,
        run_id: Uuid,
    },
    ReadImage {
        context: Context,
        asset_id: Uuid,
        revision: i64,
    },
    ReorderBlocks {
        context: Context,
        key: Uuid,
        expected_revision: i64,
        block_ids: Vec<Uuid>,
    },
}

impl Command {
    pub fn mutation_key(&self) -> Option<Uuid> {
        match self {
            Self::SaveSolution { key, .. }
            | Self::SelectCandidate { key, .. }
            | Self::CreatePipeline { key, .. }
            | Self::RevisePipeline { key, .. }
            | Self::ForkPipeline { key, .. }
            | Self::SaveTrail { key, .. }
            | Self::CreateExploration { key, .. }
            | Self::ImportChallenge { key, .. }
            | Self::AddAnnotation { key, .. }
            | Self::AddPage { key, .. }
            | Self::PinRunOutput { key, .. }
            | Self::CreateStage { key, .. }
            | Self::SaveNote { key, .. }
            | Self::ReviseText { key, .. }
            | Self::ReorderBlocks { key, .. }
            | Self::DuplicateBlock { key, .. }
            | Self::ArchiveBlock { key, .. }
            | Self::RestoreBlock { key, .. }
            | Self::AddText { key, .. }
            | Self::AddFile { key, .. }
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
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
pub enum Content {
    Text {
        text: String,
    },
    Image,
    Page {
        text: String,
    },
    File {
        byte_length: usize,
        sha256: String,
        preview: FilePreview,
    },
}

#[derive(Clone, Serialize)]
#[serde(tag = "format", rename_all = "snake_case")]
pub enum FilePreview {
    Utf8 { text: String, truncated: bool },
    Hex { text: String, truncated: bool },
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Asset {
    pub id: String,
    pub exploration_id: String,
    pub revision: i64,
    pub title: String,
    pub content: Content,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub origin: Option<Box<AssetOrigin>>,
}

#[derive(Clone, Serialize)]
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
pub enum AssetOrigin {
    Asset {
        asset_id: String,
        revision: i64,
        title: String,
    },
    RunOutput {
        run_id: String,
        attempt_id: String,
        node_id: String,
        trail_id: String,
        port: String,
        semantic_type: String,
        operation_name: String,
        sequence: i64,
    },
}

#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Block {
    Asset {
        #[serde(flatten)]
        block: Box<AssetBlock>,
    },
    Note {
        id: String,
        note: Note,
    },
}

#[derive(Serialize)]
pub struct AssetBlock {
    pub id: String,
    pub asset: Asset,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub caption: Option<Asset>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ArchivedBlock {
    pub block: Block,
    pub archived_revision: i64,
}

#[derive(Serialize)]
pub struct Note {
    pub id: String,
    pub kind: NoteKind,
    pub title: String,
    pub revision: i64,
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub origin: Option<NoteOrigin>,
}
#[derive(Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum NoteKind {
    Reasoning,
    Annotation,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NoteOrigin {
    pub note_id: String,
    pub revision: i64,
    pub title: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PipelineRef {
    pub id: Uuid,
    pub revision: i64,
}
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PipelineOrigin {
    pub pipeline_id: String,
    pub revision: i64,
}
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Pipeline {
    pub definition: serde_json::Value,
    pub output_node_id: String,
    pub input_type: String,
    pub revisions: Vec<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub origin: Option<PipelineOrigin>,
    pub id: String,
    pub title: String,
    pub revision: i64,
    pub output_operation: String,
    pub operation_version: u32,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Trail {
    pub revision: i64,
    pub id: String,
    pub workspace_id: String,
    pub exploration_id: String,
    pub stage_id: String,
    pub title: String,
    pub hypothesis: String,
    pub source: Asset,
    pub bindings: std::collections::BTreeMap<String, Asset>,
    pub pipeline_id: Option<String>,
    pub pipeline: Option<Pipeline>,
    pub archived: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Stage {
    pub id: String,
    pub title: String,
    pub revision: i64,
    pub introduction: String,
    pub blocks: Vec<Block>,
    pub archived_blocks: Vec<ArchivedBlock>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Exploration {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub origin: Option<super::challenges::Origin>,
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

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RunRequest {
    pub context: Context,
    pub key: Uuid,
    pub trail_id: Uuid,
    pub trail_revision: i64,
    pub target_node_id: Uuid,
    pub pipeline_id: Uuid,
    pub pipeline_revision: i64,
    pub source_asset_id: Uuid,
    pub source_revision: i64,
    pub operation: String,
    pub operation_version: u32,
    pub retain_output: bool,
}
