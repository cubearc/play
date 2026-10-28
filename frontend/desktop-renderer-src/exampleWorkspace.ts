// Read-only presentation fixtures. Native ownership/storage lands with scoped engine commands.
export interface AssetView {
  readonly id: string;
  readonly explorationId: string;
  readonly revision: number;
  readonly title: string;
  readonly content:
    | { readonly kind: "image" }
    | { readonly kind: "text"; readonly text: string };
}

export interface BlockView {
  readonly id: string;
  readonly asset: AssetView;
}

export interface StageView {
  readonly id: string;
  readonly title: string;
  readonly introduction: string;
  readonly blocks: readonly BlockView[];
}

export interface TrailView {
  readonly id: string;
  readonly explorationId: string;
  readonly stageId: string;
  readonly title: string;
  readonly hypothesis: string;
  readonly source: AssetView;
  readonly pipelineId: string | null;
}

export interface ExplorationView {
  readonly id: string;
  readonly title: string;
  readonly stage: StageView;
  readonly trails: readonly TrailView[];
}

export const textPipeline = {
  id: "example-text-pipeline",
  title: "Text transform",
  revision: 1,
  input: "source",
  initialOperation: "cipher.rot13",
} as const;

function exploration(id: string, title: string): ExplorationView {
  const image: AssetView = {
    id: `${id}-image`,
    explorationId: id,
    revision: 1,
    title: "signal.png",
    content: { kind: "image" },
  };
  const caption: AssetView = {
    id: `${id}-caption`,
    explorationId: id,
    revision: 1,
    title: "Image caption",
    content: { kind: "text", text: "Gur xrl vf va gur cvkryf." },
  };
  const stageId = `${id}-stage-1`;
  return {
    id,
    title,
    stage: {
      id: stageId,
      title: "Image and caption",
      introduction:
        "An image and a short caption. Inspect each source, follow a hypothesis and keep the evidence beside your reasoning.",
      blocks: [
        { id: `${id}-block-image`, asset: image },
        { id: `${id}-block-caption`, asset: caption },
      ],
    },
    trails: [
      {
        id: `${id}-image-trail`,
        explorationId: id,
        stageId,
        title: "Inspect the image",
        hypothesis: "Could the arrangement of pixels carry a message?",
        source: image,
        pipelineId: null,
      },
      {
        id: `${id}-caption-trail`,
        explorationId: id,
        stageId,
        title: "Decode the caption",
        hypothesis: "The caption may use a simple text substitution.",
        source: caption,
        pipelineId: textPipeline.id,
      },
    ],
  };
}

export const exampleWorkspace = {
  id: "example-workspace",
  title: "Signal studies",
  explorations: [
    exploration("signal", "Spiral signal"),
    exploration("alternative", "Alternative hypothesis"),
  ],
  pipelines: [textPipeline],
} as const;
