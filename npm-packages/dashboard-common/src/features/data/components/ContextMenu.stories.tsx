import { StoryObj } from "@storybook/react";
import { useRef, useState } from "react";
import { ExternalLinkIcon, StarIcon, TrashIcon } from "@radix-ui/react-icons";
import { ContextMenu } from "./ContextMenu";
import { useContextMenuTrigger } from "../lib/useContextMenuTrigger";

export default { component: ContextMenu };

type Story = StoryObj<typeof ContextMenu>;

function DemoTrigger({
  onOpenContextMenu,
  onCloseContextMenu,
}: {
  onOpenContextMenu: (position: { x: number; y: number }) => void;
  onCloseContextMenu: () => void;
}) {
  const ref = useRef<HTMLDivElement>(null);
  useContextMenuTrigger(ref, onOpenContextMenu, onCloseContextMenu);
  return (
    <div
      ref={ref}
      className="flex h-20 w-48 items-center justify-center rounded-lg border bg-background-secondary text-center"
    >
      Right-click me!
    </div>
  );
}

function ContextMenuDemo() {
  const [target, setTarget] = useState<{ x: number; y: number } | null>(null);

  return (
    <>
      <DemoTrigger
        onOpenContextMenu={(position) => setTarget(position)}
        onCloseContextMenu={() => setTarget(null)}
      />

      <ContextMenu target={target} onClose={() => setTarget(null)}>
        <ContextMenu.Item
          icon={<ExternalLinkIcon className="h-4 w-4" />}
          label="Open"
          shortcut={["CtrlOrCmd", "O"]}
          action={() => {}}
        />
        <ContextMenu.Submenu
          icon={<StarIcon className="h-4 w-4" />}
          label="Submenu"
          action={() => {}}
        >
          <ContextMenu.Item
            label={
              <>
                Item with <em>formatting</em>
              </>
            }
            action={() => {}}
          />
          <ContextMenu.Submenu
            label={
              <>
                Submenu with <em>formatting</em>
              </>
            }
            action={() => {}}
          >
            {[...Array(5)].map((_, i) => (
              <ContextMenu.Item
                key={i}
                label={`Item ${i + 1}`}
                action={() => {}}
              />
            ))}
          </ContextMenu.Submenu>
          {[...Array(10)].map((_, i) => (
            <ContextMenu.Item
              key={i}
              label={`Item ${i + 1}`}
              action={() => {}}
            />
          ))}
          <ContextMenu.Submenu label="Another submenu" action={() => {}}>
            <ContextMenu.Item label="Other item" action={() => {}} />
          </ContextMenu.Submenu>
        </ContextMenu.Submenu>
        <hr />
        <ContextMenu.Item
          icon={<TrashIcon className="h-4 w-4" />}
          label="Delete"
          shortcut={["Backspace"]}
          variant="danger"
          action={() => {}}
        />
      </ContextMenu>
    </>
  );
}

export const Primary: Story = {
  render() {
    return <ContextMenuDemo />;
  },
};
