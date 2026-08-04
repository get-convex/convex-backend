# Documentation Screenshots

This folder contains Storybook stories used to generate screenshots for the docs
site (`npm-packages/docs`).

## Importing screenshots in docs

```tsx
import { Screenshot } from "@site/src/components/Screenshot";

<Screenshot story="docs/pages/project/deployment/Data" alt="…" />

// For a non-default story:
<Screenshot story="docs/pages/project/deployment/Data#Add Document" alt="…" />
```

## Story types

- **`components/`** — Stories that render a single UI component.
- **`pages/`** — Stories that render an entire Next.js dashboard page. These use
  the decorator in
  `npm-packages/dashboard-storybook/.storybook/docsPageDecorator.tsx` which
  provides the page layout and common mocks. If you need to mock something,
  prefer adding it to the decorator so it's reused across all page stories.
  - Stories under `pages/project/` render pages inside a project.
  - Stories under `pages/project/deployment/` render pages inside a deployment.

## Cropping screenshots

Page stories can be cropped to only show specific element(s) using
`screenshotSelector`:

```ts
export const Default: Story = {
  parameters: {
    screenshotSelector: '[data-testid="table-context-menu"]',
  },
};
```

## Interacting before the screenshot

All stories support a `play` function to interact with elements before the
screenshot is taken:

```ts
import { userEvent, within } from "storybook/test";

export const Default: Story = {
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await userEvent.click(
      canvas.getByRole("button", { name: "Open project settings" }),
    );
  },
};
```

## Workflow

After any change to stories, run:

```sh
just generate-docs-screenshots
```

Then open the changed `.webp` files to visually verify the screenshots look
correct.

### Regenerating only some screenshots

The command above recaptures every `docs/` story, which is slow. To regenerate
only the stories you changed, pass a case-insensitive substring of the story
title as the **first argument** — only stories whose title contains it are
recaptured:

```sh
# Recapture only stories whose title contains "UsageLimits"
just generate-docs-screenshots UsageLimits

# Narrow further with a path-like substring (matches the story title, which is
# its file path under docs/, e.g. "docs/pages/project/deployment/settings/…")
just generate-docs-screenshots settings/usagelimits
```

The substring is matched against the full story title. Use a distinctive part of
the component or path (e.g. `UsageLimits`, `Data`, `deployment/settings`) so you
don't accidentally match unrelated stories. When a filter is passed, the other
screenshots and their manifest entries are left untouched (no stale cleanup
runs), so it's safe to iterate on one screenshot. Omit the argument to
regenerate everything before committing.
