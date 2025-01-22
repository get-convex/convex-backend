import { type ClassValue, clsx } from "clsx";
import { twMerge } from "tailwind-merge";

// Duplicated from dashboard-common
// TODO: Refactor to use dashboard-common
export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs));
}
