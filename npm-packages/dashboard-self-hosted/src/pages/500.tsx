import { Callout } from "@ui/Callout";

export default function Custom500() {
  return <Fallback error={null} />;
}

export function Fallback({ error }: { error: Error | null }) {
  return (
    <div className="h-full grow">
      <div className="flex h-full flex-col items-center justify-center">
        <Callout variant="error">
          <div className="flex flex-col gap-2">
            <p>We encountered an error loading this page.</p>
            {error && <code>{error.toString()}</code>}
          </div>
        </Callout>
      </div>
    </div>
  );
}
