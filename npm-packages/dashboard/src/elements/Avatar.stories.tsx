import { StoryObj } from "@storybook/nextjs";
import React from "react";
import { Avatar, colorSchemes } from "./Avatar";

export function AvatarGrid({ rotation }: { rotation?: number }) {
  return (
    <div className="bg-background-secondary" style={{ padding: 24 }}>
      <header className="mb-4">
        <h1 className="font-bold">All Avatar Designs</h1>
      </header>
      <div style={{ display: "flex", flexDirection: "column", gap: 32 }}>
        {[0, 1, 2, 3].map((patternIdx) => (
          <div key={patternIdx}>
            <div className="mb-2 font-semibold">Pattern {patternIdx + 1}</div>
            <div style={{ display: "flex", gap: 24 }}>
              {colorSchemes.map((__, colorSchemeIdx) => (
                <div key={colorSchemeIdx} style={{ textAlign: "center" }}>
                  <Avatar
                    name="Ava"
                    hashKey={`tar${patternIdx}${colorSchemeIdx}`}
                    patternIdx={patternIdx}
                    colorSchemeIdx={colorSchemeIdx}
                    rotation={rotation}
                  />
                  <div style={{ fontSize: 12, marginTop: 6 }}>
                    Scheme {colorSchemeIdx + 1}
                  </div>
                </div>
              ))}
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}

export default { component: Avatar };

type AvatarStory = StoryObj<typeof Avatar>;

type AvatarGridStory = StoryObj<typeof AvatarGrid>;

export const Initials: AvatarStory = {
  args: {
    name: "Zepp Williams",
  },
};

// AvatarGrid stories
export const Grid: AvatarGridStory = {
  render: (args) => <AvatarGrid {...args} />,
  args: {
    rotation: undefined,
  },
  argTypes: {
    rotation: {
      control: { type: "number", min: 0, max: 359 },
      description: "Rotation in degrees (optional)",
    },
  },
};

export const GridWithRotation: AvatarGridStory = {
  render: (args) => <AvatarGrid {...args} />,
  args: {
    rotation: 45,
  },
  argTypes: {
    rotation: {
      control: { type: "number", min: 0, max: 359 },
      description: "Rotation in degrees (optional)",
    },
  },
};
