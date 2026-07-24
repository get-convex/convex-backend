import { Command } from "cmdk";
import { Title as DialogTitle } from "@radix-ui/react-dialog";
import { ErrorBoundary } from "@sentry/nextjs";
import React, { useCallback, useRef, useState } from "react";
import { useRouter } from "next/router";
import { useHotkeys } from "react-hotkeys-hook";
import { createGlobalState, useClickAway } from "react-use";
import { Spinner } from "@ui/Spinner";
import { useLaunchDarkly } from "hooks/useLaunchDarkly";
import { toast } from "@common/lib/utils";
import { NavigationDestination, paletteFilter } from "./navigation";
import { DrillModifierContext, PaletteLoadingContext } from "./items";
import { ComponentsCommands } from "./ComponentCommands";
import { ProjectCommands, SwitchDeploymentCommands } from "./ProjectCommands";
import { DeploymentCommands } from "./DeploymentCommands";
import { PalettePage } from "./pages";
import { Breadcrumbs } from "./Breadcrumbs";
import { Footer } from "./Footer";
import { NoResultsMessage } from "./NoResultsMessage";
import { RootCommands } from "./RootCommands";
import { SwitchProjectCommands } from "./searchGroups";
import { ThemeCommands } from "./ThemeCommands";
import { TeamsCommands } from "./TeamsCommands";
import { handlePaletteKeyDown } from "./keyboard";

export const useCommandPaletteOpen = createGlobalState(false);

export function CommandPalette() {
  const { commandPalette } = useLaunchDarkly();
  const [open, setOpen] = useCommandPaletteOpen();

  useHotkeys(
    ["meta+k", "ctrl+k"],
    (event) => {
      event.preventDefault();
      setOpen((isOpen) => !isOpen);
    },
    // Allows this shortcut to work even if you're focusing a form element
    { enableOnFormTags: true },
  );

  useHotkeys("slash", (event) => {
    event.preventDefault();
    setOpen(true);
  });

  if (!commandPalette || !open) {
    return null;
  }

  return (
    <ErrorBoundary
      onError={() => {
        setOpen(false);
        toast(
          "error",
          "Something went wrong with the command palette. Please try again.",
        );
      }}
    >
      <CommandPaletteDialog onClose={() => setOpen(false)} />
    </ErrorBoundary>
  );
}

function CommandPaletteDialog({ onClose }: { onClose: () => void }) {
  const router = useRouter();
  const [search, setSearch] = useState("");
  // "Drilling" is stepping into a nested view of the palette rather than
  // navigating away (e.g. from the root into a team's list of projects, or
  // from a project into its deployments). Each drill pushes a page onto this
  // stack and clears the search.
  const [pages, setPages] = useState<PalettePage[]>([]);
  // `drillPage` is the view currently shown
  const drillPage = pages[pages.length - 1];

  const [loadingCount, setLoadingCount] = useState(0);
  const beginLoading = useCallback(() => {
    setLoadingCount((count) => count + 1);
    return () => setLoadingCount((count) => count - 1);
  }, []);
  const isSearchPending = loadingCount > 0;

  const pushPage = useCallback((newPage: PalettePage) => {
    setPages((current) => [...current, newPage]);
    setSearch("");
  }, []);

  const popPage = useCallback(() => {
    setPages((current) => current.slice(0, -1));
    setSearch("");
  }, []);

  const onNavigate = useCallback(
    (to: NavigationDestination) => {
      onClose();
      void router.push(to).then(() => {
        // For section targets, scroll the section into view once the
        // destination has rendered. This also covers re-selecting the section
        // you're already on, which is a no-op for the router.
        const hash =
          typeof to === "string" && to.includes("#")
            ? to.split("#")[1]
            : undefined;
        if (hash) {
          setTimeout(() => {
            document
              .getElementById(hash)
              ?.scrollIntoView({ behavior: "smooth", block: "start" });
          }, 100);
        }
      });
    },
    [router, onClose],
  );

  const ref = useRef<HTMLDivElement>(null);
  useClickAway(ref, onClose);

  // Used as a signal to what action should be performed by the selected list item in the palette.
  // Updated when the event handler detects the user is using a modifier key.
  const drillModifier = useRef(false);
  const armDrillModifier = (active: boolean) => {
    drillModifier.current = active;
    setTimeout(() => {
      drillModifier.current = false;
    }, 0);
  };

  const handleKeyDown = (event: React.KeyboardEvent) =>
    handlePaletteKeyDown(event, {
      inSubPage: pages.length > 0,
      search,
      popPage,
      onClose,
      armDrillModifier,
    });

  return (
    <DrillModifierContext.Provider value={drillModifier}>
      <PaletteLoadingContext.Provider value={beginLoading}>
        <Command.Dialog
          open
          ref={ref}
          label="Convex Command Palette"
          loop
          filter={paletteFilter}
          onKeyDown={handleKeyDown}
        >
          {/* cmdk renders a Radix Dialog with only an aria-label; Radix still
            requires a Dialog.Title inside the content for screen readers, so
            provide a visually hidden one. */}
          <DialogTitle className="sr-only">Convex Command Palette</DialogTitle>
          {pages.length > 0 && <Breadcrumbs pages={pages} />}
          {/* Margin bleeds past the dialog padding so the input's divider spans the
              full width. */}
          <div className="relative -mx-2">
            <Command.Input
              autoFocus
              placeholder="Search for anything…"
              value={search}
              onValueChange={setSearch}
            />
            {isSearchPending && (
              <Spinner className="absolute top-2.5 right-5 size-4 animate-fadeInFromLoading" />
            )}
          </div>
          {/* While searching, cmdk re-sorts and reparents every group/item on
              each keystroke, which restarts their load-in fade animation. This
              attribute drives the CSS rule that suppresses that fade so results
              don't flash on every character. */}
          <Command.List data-searching={search ? "" : undefined}>
            {!isSearchPending && (
              <Command.Empty>
                <NoResultsMessage onClose={onClose} />
              </Command.Empty>
            )}
            {drillPage === undefined && (
              <RootCommands
                search={search}
                onNavigate={onNavigate}
                pushPage={pushPage}
                onClose={onClose}
              />
            )}
            {drillPage?.type === "teams" && (
              <TeamsCommands onNavigate={onNavigate} />
            )}
            {drillPage?.type === "projects" && (
              <SwitchProjectCommands
                search={search}
                onNavigate={onNavigate}
                pushPage={pushPage}
              />
            )}
            {drillPage?.type === "components" && (
              <ComponentsCommands onClose={onClose} />
            )}
            {drillPage?.type === "theme" && <ThemeCommands onClose={onClose} />}
            {drillPage?.type === "project" && (
              <ProjectCommands
                project={drillPage.project}
                onNavigate={onNavigate}
                onSelectDeployment={(deployment) =>
                  pushPage({
                    type: "deployment",
                    deployment,
                    projectSlug: drillPage.project.slug,
                  })
                }
              />
            )}
            {drillPage?.type === "deployments" && (
              <SwitchDeploymentCommands
                project={drillPage.project}
                onNavigate={onNavigate}
                onSelectDeployment={(deployment) =>
                  pushPage({
                    type: "deployment",
                    deployment,
                    projectSlug: drillPage.project.slug,
                  })
                }
              />
            )}
            {drillPage?.type === "deployment" && (
              <DeploymentCommands
                deployment={drillPage.deployment}
                projectSlug={drillPage.projectSlug}
                onNavigate={onNavigate}
              />
            )}
          </Command.List>
          <Footer inSubPage={pages.length > 0} />
        </Command.Dialog>
      </PaletteLoadingContext.Provider>
    </DrillModifierContext.Provider>
  );
}
