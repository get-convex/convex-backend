import * as base64 from "js-base64";
import { ReactNode } from "react";
import { toast as sonnerToast } from "sonner";
import * as IdEncoding from "id-encoding";
import { NextRouter } from "next/router";
import { FilterExpression } from "system-udfs/convex/_system/frontend/lib/filters";
import { captureMessage } from "@sentry/nextjs";

export async function copyTextToClipboard(text: string) {
  if ("clipboard" in navigator) {
    return navigator.clipboard.writeText(text);
  }
  return document.execCommand("copy", true, text);
}

export const isUserTableName = (name: string) => !name.startsWith("_");

/**
 * @param type What type of toast to render (decides which icon and colors to use).
 * @param message The message to display with the toast.
 * @param id If set, we will update the current toast if a toast with `id`
 *           is already displayed instead of opening a new one.
 * @param duration The duration (in ms) before the toast is automatically close.
 *                 Use `false` to never auto-close this toast.
 */
export function toast(
  type: "success" | "error" | "info",
  message: ReactNode,
  id?: string,
  duration?: number | false,
) {
  sonnerToast[type](message, {
    id,
    duration: duration !== false ? duration : Number.POSITIVE_INFINITY,
  });
}

// Backoff numbers are in milliseconds.
const INITIAL_BACKOFF = 500;
const MAX_BACKOFF = 16000;

export const backoffWithJitter = (numRetries: number) => {
  const baseBackoff = INITIAL_BACKOFF * 2 ** (numRetries - 1);
  const actualBackoff = Math.min(baseBackoff, MAX_BACKOFF);
  const jitter = actualBackoff * (Math.random() - 0.5);
  return actualBackoff + jitter;
};

export function getReferencedTableName(
  tableMapping: Record<number, string> | undefined,
  possibleId: any,
): string | null {
  if (!tableMapping) {
    return null;
  }

  if (typeof possibleId !== "string") {
    return null;
  }

  let tableNumber;
  try {
    tableNumber = IdEncoding.decodeId(possibleId).tableNumber;
  } catch {
    return null;
  }

  return tableMapping[tableNumber] ?? null;
}

export function documentHref(
  router: NextRouter,
  tableName: string,
  id: string,
  componentId?: string,
): {
  pathname: string;
  query: { [key: string]: string };
} {
  const projectsURI = `/t/${router.query.team}/${router.query.project}`;
  const { query } = router;

  const filter: FilterExpression = {
    clauses: [
      {
        id: "0",
        field: "_id",
        op: "eq",
        value: id,
      },
    ],
  };

  return {
    pathname: `${projectsURI}/${router.query.deploymentName}/data`,
    query: {
      ...query,
      table: tableName,
      filters: base64.encodeURI(JSON.stringify(filter)),
      ...(componentId ? { component: componentId } : {}),
    },
  };
}

export const reportHttpError = (
  method: string,
  url: string,
  error: { code: string; message: string },
) => {
  captureMessage(
    `failed to request ${method} ${url}: ${error.code} - ${error.message} `,
  );
};
