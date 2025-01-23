export function Favicon() {
  const isDev = process.env.NEXT_PUBLIC_ENVIRONMENT === "development";
  const svgHref = isDev
    ? "/convex-logo-only-inverted.svg"
    : "/convex-logo-only.svg";
  return (
    <>
      {/* https://evilmartians.com/chronicles/how-to-favicon-in-2021-six-files-that-fit-most-needs */}

      <link rel="icon" href="/favicon.ico" sizes="any" />

      <link rel="icon" href={svgHref} type="image/svg+xml" />
      <link rel="apple-touch-icon" href="/apple-touch-icon.png" />

      {/* Only add favicons for Android, no PWA capabilities */}
      <link rel="manifest" href="/manifest.webmanifest" />
    </>
  );
}
