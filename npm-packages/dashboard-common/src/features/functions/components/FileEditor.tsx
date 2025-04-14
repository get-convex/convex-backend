import { useEffect, useRef } from "react";
import { useRouter } from "next/router";
import { useMount } from "react-use";
import { Sheet } from "@ui/Sheet";
import { Loading } from "@ui/Loading";
import { ReadonlyCode } from "@common/elements/ReadonlyCode";
import { useSourceCode } from "@common/lib/functions/useSourceCode";
import { SourceMissingPanel } from "@common/elements/SourceMissingPanel";
import type { ModuleFunction } from "@common/lib/functions/types";

export function FileEditor({
  moduleFunction,
}: {
  moduleFunction: ModuleFunction;
}) {
  const sourceCode = useSourceCode(moduleFunction.file.identifier);

  const ref = useRef<HTMLDivElement>(null);

  const router = useRouter();

  // Scroll into view on first mount if the fragment is "code"
  useMount(() => {
    window.location.hash === "#code" && ref.current?.scrollIntoView();
  });

  // Scroll into view every time the hash changes and is set to code.
  useEffect(() => {
    const onHashChangeStart = (url: string) => {
      const hash = url.split("#")[1];
      if (hash === "code") {
        ref.current?.scrollIntoView();
      }
    };

    router.events.on("hashChangeStart", onHashChangeStart);

    return () => {
      router.events.off("hashChangeStart", onHashChangeStart);
    };
  }, [router.events]);

  return (
    <Sheet
      className="h-full w-full overflow-y-auto py-2"
      padding={false}
      ref={ref}
    >
      <div className="h-full">
        {sourceCode === undefined ? (
          <div className="my-20">
            <Loading />
          </div>
        ) : sourceCode === null ? (
          <div className="my-20">
            <SourceMissingPanel />
          </div>
        ) : (
          <ReadonlyCode
            path={moduleFunction.displayName + sourceCode}
            code={sourceCode}
            language="javascript"
            highlightLines={
              moduleFunction.lineno
                ? {
                    startLineNumber: moduleFunction.lineno,
                    endLineNumber: moduleFunction.lineno,
                  }
                : undefined
            }
            height={{ type: "parent" }}
          />
        )}
      </div>
    </Sheet>
  );
}
