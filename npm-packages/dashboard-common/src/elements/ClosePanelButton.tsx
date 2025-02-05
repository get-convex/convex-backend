import { Cross2Icon } from "@radix-ui/react-icons";
import { forwardRef } from "react";
import { Button } from "@common/elements/Button";

export const ClosePanelButton = forwardRef<
  HTMLElement,
  {
    onClose: () => void;
    className?: string;
  }
>(function ClosePanelButton({ onClose, className }, ref) {
  return (
    <Button
      ref={ref}
      onClick={onClose}
      aria-label="Close panel"
      data-testid="close-panel-button"
      className={className}
      icon={<Cross2Icon aria-hidden="true" />}
      variant="neutral"
      inline
      size="xs"
    />
  );
});
