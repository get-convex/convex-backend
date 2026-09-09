import {
  ChatBubbleIcon,
  QuestionMarkCircledIcon,
  ResetIcon,
} from "@radix-ui/react-icons";
import { GiftIcon } from "@heroicons/react/24/outline";
import { useContext, useState } from "react";
import { ConfirmationDialog } from "@ui/ConfirmationDialog";
import { Menu, MenuItem } from "@ui/Menu";
import { DeploymentInfoContext } from "@common/lib/deploymentContext";
import { useNewDataFilters } from "@common/features/data/lib/useNewDataFilters";
import { GiftParcel } from "@common/elements/GiftWrap";
import { useGiftWrap } from "./useGiftWrap";
import { WhyChangedDialog } from "./WhyChangedDialog";

const FEEDBACK_CONTEXT = {
  event: "Data filter bar feedback",
  title: "Filter bar feedback",
  prompt:
    "You're trying the new filter bar on the Data page. Tell us how it's going.",
  question: "What's working, and what isn't?",
};

export function BetaMenu({ tableName }: { tableName: string }) {
  const { openFeedbackForm, useCurrentMemberName, captureEvent } = useContext(
    DeploymentInfoContext,
  );
  const { setOptedOut } = useNewDataFilters();
  const { rewrap } = useGiftWrap();
  const [confirming, setConfirming] = useState(false);
  const [explaining, setExplaining] = useState(false);

  const sendFeedback =
    openFeedbackForm && (() => openFeedbackForm(FEEDBACK_CONTEXT));
  const authorForExample = useCurrentMemberName() ?? "Me";

  const track = (event: string, action: () => void) => () => {
    captureEvent?.(event);
    action();
  };

  return (
    <>
      <Menu
        placement="bottom-end"
        buttonProps={{
          size: "sm",
          variant: "neutral",
          className: "flex size-8.5 overflow-hidden p-0",
          "aria-label": "New filter bar options",
          tip: "You're trying the new filter bar. Send feedback, or switch back.",
          tipSide: "bottom",
          children: <GiftParcel className="size-full rounded-none border-0" />,
        }}
      >
        {sendFeedback ? (
          <MenuItem
            action={track("data_filters_feedback_opened", sendFeedback)}
          >
            <ChatBubbleIcon />
            Send feedback
          </MenuItem>
        ) : null}
        <MenuItem
          action={track("data_filters_opt_out_clicked", () =>
            setConfirming(true),
          )}
        >
          <ResetIcon />
          Switch to legacy filter UI
        </MenuItem>
        <MenuItem action={track("data_filters_rewrapped", rewrap)}>
          <GiftIcon className="size-4" />
          Wrap it up again
        </MenuItem>
        <MenuItem
          action={track("data_filters_why_changed_opened", () =>
            setExplaining(true),
          )}
        >
          <QuestionMarkCircledIcon />
          Why did the filter UI change?
        </MenuItem>
      </Menu>
      {explaining && (
        <WhyChangedDialog
          onClose={() => setExplaining(false)}
          onSendFeedback={sendFeedback}
          tableName={tableName}
          values={[authorForExample]}
        />
      )}
      {confirming && (
        <ConfirmationDialog
          onClose={() => setConfirming(false)}
          onConfirm={async () => {
            captureEvent?.("data_filters_opt_out_confirmed");
            await setOptedOut(true);
          }}
          variant="primary"
          confirmText="Switch to legacy UI"
          dialogTitle="Switch to legacy filter UI"
          dialogBody="This disables the new filter experience on the Data page. The legacy filter experience is available to you until the new UI is out of beta."
        />
      )}
    </>
  );
}
