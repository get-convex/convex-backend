import type { experimental_generateVideo, ProviderMetadata } from "ai";

type VideoModel = Extract<
  Parameters<typeof experimental_generateVideo>[0]["model"],
  { specificationVersion: "v4" }
>;
type VideoOptions = Parameters<NonNullable<VideoModel["doGenerate"]>>[0];
type VideoResult = Awaited<ReturnType<NonNullable<VideoModel["doGenerate"]>>>;
type VideoFile = NonNullable<VideoOptions["image"]>;
function fileUrl(file: VideoFile): string {
  if (file.type === "url") return file.url;
  const base64 =
    typeof file.data === "string"
      ? file.data
      : btoa(
          Array.from(file.data, (byte) => String.fromCharCode(byte)).join(""),
        );
  return `data:${file.mediaType};base64,${base64}`;
}

function prepare(options: VideoOptions, modelId: string) {
  if (options.n !== 1)
    throw new Error("The Convex video gateway accepts one video per call");
  const warnings: VideoResult["warnings"] = [];
  if (options.fps !== undefined)
    warnings.push({ type: "unsupported", feature: "fps" });
  const extra = options.providerOptions.convexGateway ?? {};
  const allowedOptions = [
    "resolution",
    "generate_audio",
    "frame_images",
    "input_references",
  ];
  for (const key of Object.keys(extra)) {
    if (!allowedOptions.includes(key))
      throw new Error(`Unsupported Convex video provider option: ${key}`);
  }
  return {
    warnings,
    body: {
      ...extra,
      resolution:
        options.resolution === undefined ? extra.resolution : undefined,
      model: modelId,
      prompt: options.prompt ?? "",
      aspect_ratio: options.aspectRatio,
      size: options.resolution,
      duration: options.duration,
      seed: options.seed,
      ...(options.generateAudio !== undefined && {
        generate_audio: options.generateAudio,
      }),
      ...(options.inputReferences && {
        input_references: options.inputReferences.map((file) => {
          const kind = file.mediaType?.startsWith("video/")
            ? "video"
            : file.mediaType?.startsWith("audio/")
              ? "audio"
              : "image";
          return {
            type: `${kind}_url`,
            [`${kind}_url`]: { url: fileUrl(file) },
          };
        }),
      }),
      ...(options.frameImages
        ? {
            frame_images: options.frameImages.map((frame) => ({
              type: "image_url",
              image_url: { url: fileUrl(frame.image) },
              frame_type: frame.frameType,
            })),
          }
        : options.image && {
            frame_images: [
              {
                type: "image_url",
                image_url: { url: fileUrl(options.image) },
                frame_type: "first_frame",
              },
            ],
          }),
    },
  };
}

function videos(body: Record<string, unknown>): VideoResult["videos"] {
  const data = body.data;
  if (
    !Array.isArray(data) ||
    data.length !== 1 ||
    typeof data[0]?.b64_json !== "string" ||
    !data[0].b64_json ||
    typeof data[0]?.media_type !== "string" ||
    !data[0].media_type.startsWith("video/")
  ) {
    throw new Error("Invalid video response from the Convex AI gateway");
  }
  return [
    { type: "base64", data: data[0].b64_json, mediaType: data[0].media_type },
  ];
}

export function createVideoModel(
  modelId: string,
  baseURL: string,
  fetch: typeof globalThis.fetch,
  usageMetadata: (usage: unknown) => ProviderMetadata | undefined,
): Omit<VideoModel, "doStart" | "doStatus" | "handleWebhookOption"> {
  async function request(
    path: string,
    body: unknown,
    options: {
      headers?: Record<string, string | undefined>;
      abortSignal?: AbortSignal;
    },
  ) {
    const headers = new Headers();
    for (const [key, value] of Object.entries(options.headers ?? {})) {
      if (value !== undefined) headers.set(key, value);
    }
    headers.set("Content-Type", "application/json");
    const timestamp = new Date();
    const response = await fetch(`${baseURL}/${path}`, {
      method: "POST",
      headers,
      signal: options.abortSignal,
      body: JSON.stringify(body),
    });
    const result: unknown = await response.json();
    if (!result || typeof result !== "object" || Array.isArray(result))
      throw new Error("Invalid video response from the Convex AI gateway");
    const parsed = result as Record<string, unknown>;
    if (!response.ok || parsed.error) {
      const error = parsed.error as { message?: string } | undefined;
      throw new Error(
        error?.message ?? `Video generation failed (${response.status})`,
      );
    }
    return {
      body: parsed,
      response: {
        timestamp,
        modelId,
        headers: Object.fromEntries(response.headers),
      },
    };
  }
  return {
    specificationVersion: "v4",
    provider: "convexGateway",
    modelId,
    maxVideosPerCall: 1,
    async doGenerate(options) {
      const prepared = prepare(options, modelId);
      const { body, response } = await request(
        "videos/generations",
        prepared.body,
        options,
      );
      return {
        videos: videos(body),
        warnings: prepared.warnings,
        providerMetadata: usageMetadata(body.usage),
        response,
      };
    },
  };
}
