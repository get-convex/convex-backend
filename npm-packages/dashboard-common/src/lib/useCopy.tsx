import { useCopyToClipboard } from "react-use";
import { toast } from "dashboard-common";
import { useEffect } from "react";

export function useCopy(copying: string) {
  const [copyState, copyToClipboard] = useCopyToClipboard();

  useEffect(() => {
    if (copyState.error) {
      toast("error", `Error copying ${copying}`, undefined);
    } else if (copyState.value) {
      toast("success", `${copying} copied to clipboard.`, undefined, 2000);
    }
  }, [copyState, copying]);
  return copyToClipboard;
}
