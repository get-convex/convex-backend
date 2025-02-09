import useIsBrowser from "@docusaurus/useIsBrowser";
import { useAnalyticsCookies } from "@site/src/components/Analytics/useAnalyticsCookies";
import React from "react";

export default function CookieBanner() {
  const isBrowser = useIsBrowser();
  const { allowsCookies, setAllowsCookies } = useAnalyticsCookies();

  // Don't render during SSR.
  if (!isBrowser) {
    return null;
  }

  // Don't render if the user has accepted or rejected previously.
  if (allowsCookies !== undefined) {
    return null;
  }

  return (
    <div className="fixed z-[100] bottom-4 left-4 right-4 rounded-lg border-solid border shadow-md border-neutral-n6 bg-neutral-white p-3 text-neutral-n10 dark:border-neutral-n9 dark:shadow-lg dark:bg-neutral-n11 dark:text-neutral-white sm:left-auto sm:max-w-[24rem]">
      <p className="mb-2 leading-tight">
        We use third-party cookies to understand how people interact with our
        site.
      </p>
      <p className="mb-4 leading-tight">
        See our{" "}
        <a
          href="https://www.convex.dev/legal/privacy/"
          target="_blank"
          className="decoration-underline underline underline-offset-2 transition-colors hover:text-neutral-n12"
        >
          Privacy Policy
        </a>{" "}
        to learn more.
      </p>
      <div className="flex justify-end gap-3">
        <button
          className="rounded-full bg-neutral-n4 px-4 py-3 text-sm font-bold leading-none border-0 text-neutral-n10 opacity-80 transition-opacity hover:opacity-100 dark:bg-neutral-n10 dark:text-neutral-white cursor-pointer font-sans"
          onClick={() => setAllowsCookies(false)}
        >
          Decline
        </button>
        <button
          className="rounded-full bg-plum-p4 px-4 py-3 text-sm font-bold leading-none border-0 text-neutral-white opacity-80 transition-opacity hover:opacity-100 cursor-pointer font-sans"
          onClick={() => setAllowsCookies(true)}
        >
          Accept
        </button>
      </div>
    </div>
  );
}
