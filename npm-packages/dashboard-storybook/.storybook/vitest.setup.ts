import { setProjectAnnotations } from "@storybook/nextjs-vite";
import { beforeAll } from "vitest";
import * as a11yAddonAnnotations from "@storybook/addon-a11y/preview";
import * as previewAnnotations from "./preview";

const annotations = setProjectAnnotations([
  a11yAddonAnnotations,
  previewAnnotations,
]);

beforeAll(annotations.beforeAll);
