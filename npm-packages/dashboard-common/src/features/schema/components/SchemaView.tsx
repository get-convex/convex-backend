import { useContext, useEffect, useMemo, useRef, useState } from "react";
import { useQuery } from "convex/react";
import { useRouter } from "next/router";
import { Share2Icon, CubeIcon } from "@radix-ui/react-icons";
import udfs from "@common/udfs";
import {
  DeploymentInfoContext,
  PermissionsContext,
} from "@common/lib/deploymentContext";
import { ComponentId, useNents } from "@common/lib/useNents";
import { useTableShapes } from "@common/lib/deploymentApi";
import { SchemaJson } from "@common/lib/format";
import { DeploymentPageTitle } from "@common/elements/DeploymentPageTitle";
import { NoPermissionMessage } from "@common/elements/NoPermissionMessage";
import { NentSwitcher } from "@common/elements/NentSwitcher";
import { EmptySection } from "@common/elements/EmptySection";
import { Loading } from "@ui/Loading";
import { Button } from "@ui/Button";
import { Modal } from "@ui/Modal";
import { Panel, PanelGroup, PanelResizeHandle } from "react-resizable-panels";
import { ResizeHandle } from "@common/layouts/SidebarDetailLayout";
import { useMediaQuery } from "@common/lib/useMediaQuery";
import { ShowSchema } from "@common/features/data/components/ShowSchema";
import {
  buildGraphFromSchema,
  buildGraphFromShapes,
  SchemaGraph as SchemaGraphData,
} from "@common/features/schema/lib/buildSchemaGraph";
import { SchemaFlow } from "@common/features/schema/components/SchemaFlow";
import { SchemaSidePanel } from "@common/features/schema/components/SchemaSidePanel";
import {
  SafariLargeSchemaWarning,
  SAFARI_LARGE_SCHEMA_TABLE_COUNT,
} from "@common/features/schema/components/SafariLargeSchemaWarning";
import { useIsSafari } from "@common/lib/useIsSafari";

export function SchemaView() {
  const { useIsOperationAllowed } = useContext(PermissionsContext);
  const canViewData = useIsOperationAllowed("ViewData");

  const componentId = useNents().selectedNent?.id;

  const schemas = useQuery(
    udfs.getSchemas.default,
    canViewData ? { componentId: componentId ?? null } : "skip",
  );
  const schemaValidationProgress = useQuery(
    udfs.getSchemas.schemaValidationProgress,
    canViewData ? { componentId: componentId ?? null } : "skip",
  );
  const { tables: shapes, hadError } = useTableShapes();

  const { activeSchema, inProgressSchema } = useMemo(() => {
    if (schemas === undefined) {
      return { activeSchema: undefined, inProgressSchema: undefined };
    }
    return {
      activeSchema: schemas.active
        ? (JSON.parse(schemas.active) as SchemaJson)
        : null,
      inProgressSchema: schemas.inProgress
        ? (JSON.parse(schemas.inProgress) as SchemaJson)
        : null,
    };
  }, [schemas]);

  const graph = useMemo<SchemaGraphData | null>(() => {
    // Prefer the saved schema (it has the developer's intended relationships),
    // but merge in any tables that exist in the data without a schema entry so
    // the diagram still shows everything in the deployment (flagged as not in
    // the schema).
    if (activeSchema && activeSchema.tables.length > 0) {
      return buildGraphFromSchema(activeSchema, shapes);
    }
    // Otherwise fall back to relationships inferred from the data shapes.
    if (shapes && shapes.size > 0) {
      return buildGraphFromShapes(shapes);
    }
    return null;
  }, [activeSchema, shapes]);

  const [isShowingSchema, setIsShowingSchema] = useState(false);

  // Safari can struggle to render large schema graphs, so we gate rendering
  // behind an explicit opt-in for those users once the schema is large enough.
  const isSafari = useIsSafari();
  const [renderAnyway, setRenderAnyway] = useState(false);

  // The schema page is feature-flagged; if it's off, don't render it even when
  // reached by a direct URL — send the user to the data page.
  const router = useRouter();
  const { schemaPageEnabled, deploymentsURI } = useContext(
    DeploymentInfoContext,
  );
  useEffect(() => {
    if (!schemaPageEnabled) {
      void router.replace(`${deploymentsURI}/data`);
    }
  }, [schemaPageEnabled, deploymentsURI, router]);
  if (!schemaPageEnabled) {
    return null;
  }

  if (!canViewData) {
    return (
      <>
        <DeploymentPageTitle title="Schema" />
        <NoPermissionMessage
          message="You do not have permission to view the schema in this deployment."
          missingPermission="deployment:data:view"
        />
      </>
    );
  }

  // Shapes stay `undefined` when the shapes fetch errors; treat that as loaded
  // (not a perpetual spinner) so we fall through to the graph or error state.
  const isLoading =
    activeSchema === undefined || (shapes === undefined && !hadError);

  const shouldWarnLargeSchemaInSafari =
    isSafari &&
    !renderAnyway &&
    graph !== null &&
    graph.nodes.length > SAFARI_LARGE_SCHEMA_TABLE_COUNT;

  return (
    <div className="flex h-full max-w-full grow flex-col overflow-hidden p-6">
      <DeploymentPageTitle title="Schema" />
      {isShowingSchema && shapes && (
        <Modal
          onClose={() => setIsShowingSchema(false)}
          title={<div className="px-3">Schema</div>}
          size="md"
        >
          <ShowSchema
            activeSchema={activeSchema}
            inProgressSchema={inProgressSchema}
            shapes={shapes}
            hasShapeError={hadError}
            schemaValidationProgress={schemaValidationProgress}
          />
        </Modal>
      )}
      <SchemaHeader
        // The schema modal needs shapes; keep the button disabled until they
        // load (and when their fetch errored), regardless of the graph state.
        onShowSchemaCode={shapes ? () => setIsShowingSchema(true) : undefined}
      />
      <div className="mt-4 min-h-0 grow overflow-hidden">
        {isLoading ? (
          <div className="h-full rounded-lg border">
            <Loading />
          </div>
        ) : graph === null || graph.nodes.length === 0 ? (
          <EmptySchema hadError={hadError} />
        ) : shouldWarnLargeSchemaInSafari ? (
          <div className="h-full overflow-hidden rounded-lg border">
            <SafariLargeSchemaWarning
              onRenderAnyway={() => setRenderAnyway(true)}
            />
          </div>
        ) : (
          <div className="h-full overflow-hidden rounded-lg border">
            <SchemaGraphWithNavigation
              graph={graph}
              componentId={componentId}
            />
          </div>
        )}
      </div>
    </div>
  );
}

function SchemaHeader({
  onShowSchemaCode,
}: {
  onShowSchemaCode: (() => void) | undefined;
}) {
  return (
    <div className="flex flex-wrap items-center justify-between gap-4">
      <div className="flex max-w-full flex-col gap-1">
        <h3>Schema</h3>
      </div>
      <div className="flex items-center gap-2">
        <div className="w-64">
          <NentSwitcher />
        </div>
        <Button
          variant="neutral"
          icon={<CubeIcon />}
          onClick={onShowSchemaCode}
          disabled={onShowSchemaCode === undefined}
        >
          View schema file
        </Button>
      </div>
    </div>
  );
}

function SchemaGraphWithNavigation({
  graph,
  componentId,
}: {
  graph: SchemaGraphData;
  componentId: ComponentId | undefined;
}) {
  const router = useRouter();
  const { deploymentsURI, useCurrentDeployment } = useContext(
    DeploymentInfoContext,
  );
  const deployment = useCurrentDeployment();
  const [selectedTable, setSelectedTable] = useState<string | null>(null);

  // On narrow screens the side panel is too cramped beside the graph, so stack
  // it below the graph (full width) instead of splitting horizontally.
  const isMobile = useMediaQuery("(max-width: 768px)");

  // A bump-on-each-request signal that asks the graph to pan a table into view.
  // The nonce makes repeated requests for the same table re-trigger the pan.
  const [focusRequest, setFocusRequest] = useState<{
    table: string;
    nonce: number;
  } | null>(null);
  const focusNonceRef = useRef(0);

  // Focused when a table is selected from the keyboard, so a keyboard/screen
  // reader user lands in the details instead of staying on the canvas.
  const panelRef = useRef<HTMLDivElement>(null);

  const selectTable = (table: string, opts?: { fromKeyboard?: boolean }) => {
    setSelectedTable(table);
    if (opts?.fromKeyboard) {
      // The panel mounts on this state change; focus it once it's committed.
      window.requestAnimationFrame(() => panelRef.current?.focus());
    }
  };

  // Persist manual layout per deployment and component.
  const storageKey = `schemaLayout/${deployment?.name}/${componentId ?? "_root"}`;

  const openTableData = (table: string) => {
    void router.push({
      pathname: `${deploymentsURI}/data`,
      // Carry the component through so the Data page opens the table in the
      // same component the graph is showing, not the root app.
      query: componentId ? { table, component: componentId } : { table },
    });
  };

  const focusTable = (table: string) => {
    setSelectedTable(table);
    focusNonceRef.current += 1;
    setFocusRequest({ table, nonce: focusNonceRef.current });
  };

  const selectedNode =
    graph.nodes.find((node) => node.table === selectedTable) ?? null;

  useEffect(() => {
    if (!selectedNode) {
      return undefined;
    }
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        setSelectedTable(null);
      }
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [selectedNode]);

  return (
    <PanelGroup
      // Stack vertically on mobile so the side panel gets full width. Vary the
      // autoSaveId by orientation so a stored horizontal width % isn't applied
      // as a vertical height % (and vice versa).
      direction={isMobile ? "vertical" : "horizontal"}
      autoSaveId={`schema-side-panel/${deployment?.name}/${componentId ?? "_root"}/${isMobile ? "v" : "h"}`}
      className="size-full"
    >
      <Panel
        id="schema-graph"
        order={1}
        minSize={30}
        className="relative flex min-w-0 flex-col"
      >
        <SchemaFlow
          graph={graph}
          storageKey={storageKey}
          selectedTable={selectedTable}
          focusRequest={focusRequest}
          onSelectNode={selectTable}
          onFocusTable={focusTable}
          onClearSelection={() => setSelectedTable(null)}
        />
      </Panel>
      {selectedNode && (
        <>
          {isMobile ? (
            // The shared ResizeHandle is built for a left/right split; on mobile
            // we want a full-width grabber between the stacked panels.
            <PanelResizeHandle className="flex h-2 w-full items-center justify-center border-t bg-background-secondary/70 transition-colors data-[resize-handle-state=drag]:bg-util-accent/10">
              <div className="h-0.5 w-8 rounded-full bg-content-tertiary/50" />
            </PanelResizeHandle>
          ) : (
            <ResizeHandle collapsed={false} direction="left" />
          )}
          <Panel
            id="schema-panel"
            order={2}
            defaultSize={isMobile ? 45 : 28}
            minSize={15}
            maxSize={isMobile ? 75 : 60}
            className="min-w-0"
          >
            <SchemaSidePanel
              ref={panelRef}
              node={selectedNode}
              onClose={() => setSelectedTable(null)}
              onOpenData={openTableData}
              onFocusTable={focusTable}
            />
          </Panel>
        </>
      )}
    </PanelGroup>
  );
}

function EmptySchema({ hadError }: { hadError: boolean }) {
  return (
    <EmptySection
      Icon={hadError ? Share2Icon : CubeIcon}
      header={
        hadError
          ? "Couldn't load the schema."
          : "This deployment doesn't have any tables"
      }
      body={
        hadError
          ? "We ran into an error loading the schema for this deployment."
          : "Create a table and add a convex/schema.ts to see your tables and the relationships between them."
      }
      learnMoreButton={{
        href: "https://docs.convex.dev/database/schemas",
        children: "Learn about schemas",
      }}
    />
  );
}
