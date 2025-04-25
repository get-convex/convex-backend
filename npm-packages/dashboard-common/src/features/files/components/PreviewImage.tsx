import { useState } from "react";
import Image from "next/image";
import { Spinner } from "@ui/Spinner";

export function PreviewImage({ url }: { url: string }) {
  const [[width, height], setSize] = useState<
    [number | undefined, number | undefined]
  >([undefined, undefined]);
  return (
    <div className="relative">
      {(!width || !height) && <Spinner className="animate-fadeInFromLoading" />}
      <Image
        src={url}
        alt="image preview"
        // Hack to correctly size the preview image to the appropriate dimensions:
        // Start with fill set to true, then set the width and height to the natural width and height of the image.
        fill={!width || !height}
        objectFit="contain"
        width={width}
        height={height}
        onLoadingComplete={({ naturalWidth, naturalHeight }) => {
          setSize([naturalWidth, naturalHeight]);
        }}
      />
    </div>
  );
}
