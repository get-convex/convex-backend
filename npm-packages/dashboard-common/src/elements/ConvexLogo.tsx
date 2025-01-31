"use client";

import { useTheme } from "next-themes";
import Image from "next/image";
import { useState, useEffect } from "react";

type LogoProps = {
  width?: number;
  height?: number;
};

export function ConvexLogo({ width, height }: LogoProps) {
  const { resolvedTheme: currentTheme } = useTheme();
  const prefersDark = currentTheme === "dark";

  const [mounted, setMounted] = useState(false);
  useEffect(() => setMounted(true), []);
  if (!mounted) return null;

  return (
    <Image
      src={prefersDark ? "/convex-dark.svg" : "/convex-light.svg"}
      height={height ?? 76}
      width={width ?? 228}
      alt="Convex Logo"
    />
  );
}
