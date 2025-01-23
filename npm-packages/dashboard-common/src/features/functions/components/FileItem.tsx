import { Disclosure } from "@headlessui/react";
import { CodeIcon } from "@radix-ui/react-icons";
import type { File } from "../../../lib/functions/types";
import { DirectoryItem, FunctionItem } from "./FunctionItem";
import { useCurrentOpenFunction } from "../../../lib/functions/FunctionsProvider";

export function FileItem({
  file,
  onChangeFunction,
  defaultOpen,
  nestingLevel,
}: {
  file: File;
  onChangeFunction: () => void;
  defaultOpen?: boolean;
  nestingLevel: number;
}) {
  const currentOpenFunction = useCurrentOpenFunction();
  const childIsCurrentFunction =
    currentOpenFunction?.file.identifier.startsWith(file.identifier);
  // If there is only one function in the module collapse the function into
  // the module.
  return (
    <>
      {file.functions.length > 1 && (
        <Disclosure
          key={file.name}
          defaultOpen={childIsCurrentFunction || defaultOpen}
        >
          {({ open }) => (
            <>
              <DirectoryItem
                disclosure
                nestingLevel={nestingLevel}
                isActive={!open && childIsCurrentFunction}
              >
                <CodeIcon className="size-4 shrink-0 text-content-tertiary" />
                <div className="truncate">{file.name}</div>
              </DirectoryItem>
              <Disclosure.Panel className="flex flex-col">
                {file.functions.map((f) => (
                  <FunctionItem
                    key={f.name}
                    item={f}
                    onChangeFunction={onChangeFunction}
                    nestingLevel={nestingLevel + 1}
                  />
                ))}
              </Disclosure.Panel>
            </>
          )}
        </Disclosure>
      )}{" "}
      {file.functions.length === 1 && (
        <FunctionItem
          key={file.functions[0].identifier}
          item={file.functions[0]}
          showFileName
          onChangeFunction={onChangeFunction}
          nestingLevel={nestingLevel}
        />
      )}
    </>
  );
}
