import type { Meta, StoryObj } from "@storybook/react";
import { useState } from "react";
import { ToastContainer } from "elements/ToastContainer";
import { toast } from "lib/utils";
import { Sheet } from "elements/Sheet";
import { TextInput } from "elements/TextInput";
import { Button } from "elements/Button";

export default {
  component: ToastContainer,
} as Meta<typeof ToastContainer>;

function ToastDemo() {
  const [message, setMessage] = useState("Hello world");

  const types = ["success", "error", "info"] as const;
  const [type, setType] = useState<(typeof types)[number]>("success");
  const [permanent, setPermanent] = useState(false);

  return (
    <>
      <Sheet className="text-sm">
        <form
          onSubmit={(e) => {
            e.preventDefault();
            toast(type, message, undefined, permanent ? false : undefined);
          }}
        >
          <div className="flex flex-col items-start gap-4">
            <TextInput
              id="toast"
              value={message}
              onChange={(e) => setMessage(e.currentTarget.value)}
            />

            <div className="flex flex-wrap gap-4">
              {types.map((t) => (
                <label key={t}>
                  <input
                    type="radio"
                    className="mr-2"
                    checked={type === t}
                    onChange={() => setType(t)}
                  />
                  {t}
                </label>
              ))}
            </div>

            {/* eslint-disable-next-line jsx-a11y/label-has-associated-control */}
            <label>
              <input
                className="mr-1"
                type="checkbox"
                checked={permanent}
                onChange={(e) => {
                  setPermanent(e.target.checked);
                }}
              />{" "}
              Permanent
            </label>

            <Button type="submit">Show Toast</Button>
          </div>
        </form>
      </Sheet>

      <ToastContainer />
    </>
  );
}

export const Demo: StoryObj<typeof ToastContainer> = {
  render: () => <ToastDemo />,
};
