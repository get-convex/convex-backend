import { useCopyToClipboard } from "react-use";
import { useEffect } from "react";
import { toast } from "./utils";

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
