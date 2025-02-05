import { forwardRef } from "react";
import { isInCommonUTCTimestampRange } from "@common/features/data/lib/helpers";
import { Tooltip } from "@common/elements/Tooltip";

type DataCellValueProps = {
  isDateField: boolean;
  inferIsDate: boolean;
  value: any;
  isHovered: boolean;
  isReference: boolean;
  detailHeader: string;
  stringValue: string;
};

export const DataCellValue = forwardRef<HTMLSpanElement, DataCellValueProps>(
  function DataCellValue(
    {
      isDateField,
      inferIsDate,
      value,
      isHovered,
      isReference,
      detailHeader,
      stringValue,
    },
    ref,
  ) {
    return (
      <span ref={ref} className="flex-1 truncate">
        {isDateField ||
        (inferIsDate &&
          typeof value === "number" &&
          isInCommonUTCTimestampRange(value)) ? (
          // This tooltip is cheating, it really should be around the whole
          // cell, but that doesn't render correctly, at least with the
          // current version of Radix.
          // Doing this means we no longer have the value focusable,
          // but keyboard users can still copy it via clipboard.
          // Only render tooltip when hovering because it's slow
          isHovered ? (
            <Tooltip tip={value} side="bottom" align="start" wrapsButton>
              <span>{new Date(value).toLocaleString()}</span>
            </Tooltip>
          ) : (
            <span>{new Date(value).toLocaleString()}</span>
          )
        ) : isReference || detailHeader === "_id" ? (
          <span className="font-semibold" aria-label="Document ID">
            {value.toString()}
          </span>
        ) : typeof value === "string" ? (
          <span
            className={`before:text-content-secondary before:content-['"'] after:text-content-secondary after:content-['"']`}
          >
            {stringValue}
          </span>
        ) : value === undefined ? (
          <span className="italic text-content-secondary">unset</span>
        ) : (
          <span>{stringValue}</span>
        )}
      </span>
    );
  },
);
