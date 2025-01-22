import React, { Component, FormEvent, ReactNode } from "react";
import { ConvexError } from "convex/values";

interface Props {
  children?: ReactNode;
  clearMessages: () => void;
}

interface State {
  hasError: boolean;
  error: Error | null;
}

class ErrorBoundary extends Component<Props, State> {
  public state: State = {
    hasError: false,
    error: null,
  };

  public static getDerivedStateFromError(error: Error): State {
    // Update state so the next render will show the fallback UI.
    return { hasError: true, error };
  }

  public render() {
    if (this.state.hasError) {
      return this.state.error instanceof ConvexError ? (
        <div className="error" role="alert">
          <p>Something went wrong:</p>
          <pre
            style={{ color: "red" }}
          >{`${this.state.error.data.message}: ${this.state.error.data.length}`}</pre>
          <button
            onClick={(_e: FormEvent) => {
              const handleClearMessages = async (error: Error) => {
                if (
                  error instanceof ConvexError &&
                  error.data.code === "MESSAGE_LIMIT"
                ) {
                  await this.props.clearMessages();
                  this.setState({ hasError: false, error: null });
                }
              };
              if (this.state.error) {
                handleClearMessages(this.state.error);
              }
            }}
          >
            Retry
          </button>
        </div>
      ) : (
        <div>Unexpected error occurred!</div>
      );
    }

    return this.props.children;
  }
}

export default ErrorBoundary;
