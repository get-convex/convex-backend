import { useReactFlow, useStore } from "@xyflow/react";
import { PlusIcon, MinusIcon, ResetIcon } from "@radix-ui/react-icons";
import { ViewfinderCircleIcon } from "@heroicons/react/24/outline";
import { Button } from "@ui/Button";
import { Tooltip } from "@ui/Tooltip";

export function SchemaControls({
  onResetLayout,
}: {
  onResetLayout: () => void;
}) {
  const { zoomIn, zoomOut, fitView } = useReactFlow();
  const zoom = useStore((s) => s.transform[2]);

  return (
    <div className="absolute bottom-3 left-3 z-10 flex items-center gap-1 rounded-lg border bg-background-secondary p-1 shadow-sm">
      <div className="flex items-center gap-0.5">
        <Tooltip tip="Zoom out" side="top">
          <Button
            size="xs"
            variant="neutral"
            inline
            className="border border-transparent"
            onClick={() => zoomOut()}
            icon={<MinusIcon />}
            aria-label="Zoom out"
          />
        </Tooltip>
        <span className="text-xs tabular-nums select-none">
          {Math.round(zoom * 100)}%
        </span>
        <Tooltip tip="Zoom in" side="top">
          <Button
            size="xs"
            variant="neutral"
            inline
            className="border border-transparent"
            onClick={() => zoomIn()}
            icon={<PlusIcon />}
            aria-label="Zoom in"
          />
        </Tooltip>
      </div>
      <div className="mx-0.5 h-5 border-l" />
      <Tooltip tip="Fit to view" side="top">
        <Button
          size="xs"
          variant="neutral"
          inline
          className="border border-transparent"
          onClick={() => fitView()}
          icon={<ViewfinderCircleIcon className="size-4" />}
          aria-label="Fit to view"
        />
      </Tooltip>
      <Tooltip tip="Reset layout" side="top">
        <Button
          size="xs"
          variant="neutral"
          inline
          className="border border-transparent"
          onClick={onResetLayout}
          icon={<ResetIcon />}
          aria-label="Reset layout"
        />
      </Tooltip>
    </div>
  );
}
