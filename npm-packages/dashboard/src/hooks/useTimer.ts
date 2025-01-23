import { useRefresh } from "dashboard-common";

export function useTimer(date?: Date) {
  useRefresh(100);
  if (!date) {
    return 0;
  }

  return ((new Date().getTime() - date.getTime()) / 1000).toFixed(1);
}
