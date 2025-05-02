import { cn } from "@ui/cn";

export function Spinner({ className }: { className?: string }) {
  return (
    <svg
      role="status"
      className={cn(
        "ml-auto h-4 w-4 text-content-primary/70 animate-rotate",
        className,
      )}
      viewBox="0 0 100 100"
      fill="none"
      xmlns="http://www.w3.org/2000/svg"
    >
      <path
        d="M 50,50 m 0,-38 a 38,38 0 1,1 0,76 a 38,38 0 1,1 0,-76"
        pathLength="100"
        stroke="currentColor"
        strokeWidth="18"
        fill="none"
        className="animate-dashLength"
      />
    </svg>
  );
}
