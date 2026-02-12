import React, { Component } from 'react';

// Error Boundary Component to catch React errors
class ErrorBoundary extends Component {
  constructor(props) {
    super(props);
    this.state = { 
      hasError: false, 
      error: null, 
      errorInfo: null 
    };
  }

  static getDerivedStateFromError(error) {
    // Update state so the next render shows fallback UI
    return { hasError: true };
  }

  componentDidCatch(error, errorInfo) {
    // Log error details
    console.error('Error caught by boundary:', error);
    console.error('Error info:', errorInfo);
    
    this.setState({
      error: error,
      errorInfo: errorInfo
    });

    // You can also log to an error reporting service here
    // logErrorToService(error, errorInfo);
  }

  handleReset = () => {
    this.setState({ 
      hasError: false, 
      error: null, 
      errorInfo: null 
    });
  };

  render() {
    if (this.state.hasError) {
      // Fallback UI
      return (
        <div style={{
          minHeight: '100vh',
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
          background: 'linear-gradient(135deg, #667eea 0%, #764ba2 100%)',
          padding: '20px'
        }}>
          <div style={{
            background: 'white',
            borderRadius: '20px',
            boxShadow: '0 20px 40px rgba(0, 0, 0, 0.1)',
            padding: '40px',
            maxWidth: '600px',
            width: '100%',
            textAlign: 'center'
          }}>
            <h1 style={{
              fontSize: '2rem',
              color: '#e53e3e',
              marginBottom: '20px'
            }}>
              ⚠️ Something went wrong
            </h1>
            
            <p style={{
              color: '#4a5568',
              marginBottom: '30px',
              fontSize: '1rem'
            }}>
              We're sorry, but something unexpected happened. Please try refreshing the page.
            </p>

            {process.env.NODE_ENV === 'development' && this.state.error && (
              <details style={{
                textAlign: 'left',
                background: '#f7fafc',
                padding: '20px',
                borderRadius: '8px',
                marginBottom: '20px',
                fontSize: '0.875rem',
                maxHeight: '300px',
                overflow: 'auto'
              }}>
                <summary style={{ 
                  cursor: 'pointer', 
                  fontWeight: '600',
                  marginBottom: '10px',
                  color: '#2d3748'
                }}>
                  Error Details (Development Mode)
                </summary>
                <pre style={{
                  whiteSpace: 'pre-wrap',
                  wordBreak: 'break-word',
                  color: '#e53e3e'
                }}>
                  {this.state.error.toString()}
                </pre>
                {this.state.errorInfo && (
                  <pre style={{
                    whiteSpace: 'pre-wrap',
                    wordBreak: 'break-word',
                    color: '#4a5568',
                    marginTop: '10px'
                  }}>
                    {this.state.errorInfo.componentStack}
                  </pre>
                )}
              </details>
            )}

            <div style={{ display: 'flex', gap: '10px', justifyContent: 'center' }}>
              <button
                onClick={this.handleReset}
                style={{
                  padding: '12px 24px',
                  background: '#667eea',
                  color: 'white',
                  border: 'none',
                  borderRadius: '8px',
                  fontSize: '1rem',
                  fontWeight: '600',
                  cursor: 'pointer'
                }}
              >
                Try Again
              </button>
              
              <button
                onClick={() => window.location.reload()}
                style={{
                  padding: '12px 24px',
                  background: '#48bb78',
                  color: 'white',
                  border: 'none',
                  borderRadius: '8px',
                  fontSize: '1rem',
                  fontWeight: '600',
                  cursor: 'pointer'
                }}
              >
                Refresh Page
              </button>
            </div>
          </div>
        </div>
      );
    }

    return this.props.children;
  }
}

// Usage Example: Wrap your App component
// 
// function App() {
//   return (
//     <ErrorBoundary>
//       <YourAppContent />
//     </ErrorBoundary>
//   );
// }

export default ErrorBoundary;