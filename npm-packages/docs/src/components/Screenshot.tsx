import React from "react";
import { useColorMode } from "@docusaurus/theme-common";
import { screenshots } from "../generated/screenshotManifest";

type StoryTitle = (typeof screenshots)[number]["storyTitle"];

const screenshotMap = new Map(screenshots.map((s) => [s.storyTitle, s]));

export function Screenshot({ story, alt }: { story: StoryTitle; alt: string }) {
  const { colorMode } = useColorMode();
  const entry = screenshotMap.get(story);
  if (!entry) throw new Error(`Screenshot not found: ${story}`);

  const img = colorMode === "dark" ? entry.dark : entry.light;

  return (
    <img
      src={`/screenshots/storybook/${img.filename}`}
      alt={alt}
      width={img.width / 2}
      height={img.height / 2}
      style={{ maxWidth: "100%", height: "auto", borderRadius: 8 }}
    />
  );
}
