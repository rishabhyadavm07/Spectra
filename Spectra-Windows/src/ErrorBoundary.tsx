import { Component, ErrorInfo, ReactNode } from "react";
import { AlertTriangle, RefreshCw } from "lucide-react";

interface Props {
  children?: ReactNode;
  fallback?: ReactNode;
  onReset?: () => void;
}

interface State {
  hasError: boolean;
  error: Error | null;
}

export class ErrorBoundary extends Component<Props, State> {
  public state: State = {
    hasError: false,
    error: null,
  };

  public static getDerivedStateFromError(error: Error): State {
    return { hasError: true, error };
  }

  public componentDidCatch(error: Error, errorInfo: ErrorInfo) {
    console.error("Uncaught error:", error, errorInfo);
  }

  private handleReset = () => {
    this.setState({ hasError: false, error: null });
    if (this.props.onReset) {
      this.props.onReset();
    }
  };

  public render() {
    if (this.state.hasError) {
      if (this.props.fallback) {
        return this.props.fallback;
      }

      return (
        <div style={{
          display: 'flex',
          flexDirection: 'column',
          alignItems: 'center',
          justifyContent: 'center',
          height: '100%',
          width: '100%',
          padding: '2rem',
          backgroundColor: 'var(--bg-main)',
          color: 'var(--text-main)',
          textAlign: 'center',
          fontFamily: 'Inter, sans-serif'
        }}>
          <AlertTriangle size={48} color="var(--danger)" style={{ marginBottom: '1rem' }} />
          <h2 style={{ marginBottom: '0.5rem', fontWeight: 600 }}>Something went wrong</h2>
          <p style={{ color: 'var(--text-muted)', marginBottom: '1.5rem', maxWidth: '400px' }}>
            {this.state.error?.message || "An unexpected error occurred in this component."}
          </p>
          <button
            onClick={this.handleReset}
            style={{
              display: 'flex',
              alignItems: 'center',
              gap: '0.5rem',
              padding: '0.5rem 1rem',
              backgroundColor: 'var(--bg-panel)',
              border: '1px solid var(--border)',
              borderRadius: '6px',
              color: 'var(--text-main)',
              cursor: 'pointer',
              fontWeight: 500
            }}
          >
            <RefreshCw size={16} />
            Try again
          </button>
        </div>
      );
    }

    return this.props.children;
  }
}
