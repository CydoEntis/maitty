import { Component, type ReactNode } from 'react'

type ErrorBoundaryProps = {
  children: ReactNode
}

type ErrorBoundaryState = {
  hasError: boolean
}

export default class ErrorBoundary extends Component<ErrorBoundaryProps, ErrorBoundaryState> {
  override state: ErrorBoundaryState = { hasError: false }

  static getDerivedStateFromError(): ErrorBoundaryState {
    return { hasError: true }
  }

  override render() {
    if (this.state.hasError) {
      return (
        <main className="fatal-error" role="alert">
          <h1>Maitty hit a problem</h1>
          <p>Reload the app to try again.</p>
          <button type="button" onClick={() => { window.location.reload() }}>Reload app</button>
        </main>
      )
    }

    return this.props.children
  }
}
