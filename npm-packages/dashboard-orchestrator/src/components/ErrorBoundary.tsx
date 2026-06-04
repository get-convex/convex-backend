import { ExitIcon } from "@radix-ui/react-icons";
import React, { ErrorInfo, ReactNode } from "react";
import { Button } from "@ui/Button";
import { Sheet } from "@ui/Sheet";

type Props = { children: ReactNode };
type State = { error?: Error };

export class ErrorBoundary extends React.Component<Props, State> {
  constructor(props: Props) {
    super(props);
    this.state = {};
  }
  static getDerivedStateFromError(e: Error): State {
    return { error: e };
  }
  componentDidCatch(error: Error, info: ErrorInfo) {
    console.error("Uncaught error:", error, info);
  }
  render() {
    const { error } = this.state;
    if (!error) return this.props.children;
    return (
      <div className="flex h-screen w-full flex-col items-center justify-center gap-4">
        <h3>Something went wrong</h3>
        <Button
          className="w-fit"
          icon={<ExitIcon />}
          size="xs"
          onClick={() => window.location.reload()}
          variant="neutral"
        >
          Reload
        </Button>
        <Sheet className="max-h-[50vh] w-200 max-w-[80vw] overflow-auto font-mono text-sm">
          <div>{error.message}</div>
          <pre className="mt-2 text-xs">
            <code>{error.stack}</code>
          </pre>
        </Sheet>
      </div>
    );
  }
}
