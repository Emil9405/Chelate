import React, { useState, useEffect } from 'react';
import { api } from './services/api';
import Login from './components/Login';
import Header from './components/Header';
import Dashboard from './components/Dashboard';
import Reagents from './components/Reagents';
import AdvancedFilters from './components/AdvancedFilters';
import Equipment from './components/Equipment';
import Users from './components/Users';
import Reports from './components/Reports';
import Experiments from './components/Experiments';
import ErrorBoundary from './components/ErrorBoundary';
import LocationManager from './components/storage/LocationManager'; // <-- Изменили импорт на LocationManager
import './index.css';

const App = () => {
  const [isAuthenticated, setIsAuthenticated] = useState(false);
  const [user, setUser] = useState(null);
  const [currentPage, setCurrentPage] = useState(
    () => localStorage.getItem('chelate_page') || 'dashboard'
  );
  const [loading, setLoading] = useState(true);
  const [backendError, setBackendError] = useState(false);

  useEffect(() => {
    initializeApp();
  }, []);

  useEffect(() => {
  localStorage.setItem('chelate_page', currentPage);
  }, [currentPage]);

  const initializeApp = async () => {
    api.init(); // Initialize token from localStorage
    
    if (api.token) {
      try {
        // Try to get user profile with existing token
        const response = await api.getProfile();
        if (response.success) {
          setUser(response.data);
          setIsAuthenticated(true);
          setBackendError(false);
        } else {
          api.clearToken();
        }
      } catch (error) {
        console.warn('Failed to get user profile:', error);
        // Check if it's a backend connection error
        if (error.message && error.message.includes('non-JSON')) {
          setBackendError(true);
        }
        api.clearToken();
      }
    }
    setLoading(false);
  };

  const handleLogin = (userData) => {
    setUser(userData);
    setIsAuthenticated(true);
    setBackendError(false);
  };

  const handleLogout = async () => {
    try {
      await api.logout();
    } catch (error) {
      console.error('Logout failed:', error);
    } finally {
      setIsAuthenticated(false);
      setUser(null);
      setCurrentPage('dashboard');
    }
  };

  if (loading) {
    return (
      <div style={{
        minHeight: '100vh',
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        backgroundColor: '#f7fafc'
      }}>
        <div style={{ textAlign: 'center' }}>
          <div style={{
            width: '40px',
            height: '40px',
            border: '4px solid #e2e8f0',
            borderRadius: '50%',
            borderTopColor: '#667eea',
            animation: 'spin 1s linear infinite',
            margin: '0 auto 1rem'
          }}></div>
          <p>Loading application...</p>
        </div>
      </div>
    );
  }

  // Show backend error message
  if (backendError) {
    return (
      <div style={{
        minHeight: '100vh',
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        backgroundColor: '#f7fafc',
        padding: '20px'
      }}>
        <div style={{
          maxWidth: '500px',
          backgroundColor: 'white',
          borderRadius: '8px',
          padding: '2rem',
          boxShadow: '0 4px 6px rgba(0, 0, 0, 0.1)',
          textAlign: 'center'
        }}>
          <div style={{
            width: '60px',
            height: '60px',
            backgroundColor: '#fee',
            borderRadius: '50%',
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            margin: '0 auto 1rem',
            color: '#dc2626',
            fontSize: '24px'
          }}>⚠️</div>
          <h2 style={{ marginBottom: '1rem', color: '#1f2937' }}>Backend Server Not Available</h2>
          <p style={{ marginBottom: '1.5rem', color: '#6b7280', lineHeight: '1.6' }}>
            Cannot connect to the API server at <code style={{ 
              backgroundColor: '#f3f4f6', 
              padding: '2px 6px', 
              borderRadius: '4px',
              fontSize: '0.9em'
            }}>http://localhost:8080</code>
          </p>
          <div style={{
            backgroundColor: '#f3f4f6',
            padding: '1rem',
            borderRadius: '6px',
            marginBottom: '1.5rem',
            textAlign: 'left'
          }}>
            <p style={{ fontWeight: '600', marginBottom: '0.5rem', color: '#374151' }}>To fix this:</p>
            <ol style={{ marginLeft: '1.5rem', color: '#6b7280', lineHeight: '1.8' }}>
              <li>Make sure your Rust backend is running</li>
              <li>Check that it's listening on port 8080</li>
              <li>Verify the API endpoint: <code style={{ fontSize: '0.85em' }}>http://localhost:8080/api</code></li>
            </ol>
          </div>
          <button
            onClick={() => window.location.reload()}
            style={{
              backgroundColor: '#667eea',
              color: 'white',
              border: 'none',
              padding: '0.75rem 1.5rem',
              borderRadius: '6px',
              fontSize: '1rem',
              cursor: 'pointer',
              fontWeight: '500'
            }}
            onMouseOver={(e) => e.target.style.backgroundColor = '#5568d3'}
            onMouseOut={(e) => e.target.style.backgroundColor = '#667eea'}
          >
            Retry Connection
          </button>
        </div>
      </div>
    );
  }

  if (!isAuthenticated) {
    return <Login onLogin={handleLogin} />;
  }

  const renderPage = () => {
    switch (currentPage) {
      case 'dashboard':
        return <Dashboard user={user} onNavigate={setCurrentPage} />;
      case 'reagents':
        return <Reagents user={user} />;
      case 'equipment':
        return <Equipment user={user} />;
      case 'experiments':  
        return <Experiments user={user} />;
      case 'reports':
        return <Reports user={user} />;
      case 'users':
        return <Users user={user} />;
      case 'storage': // Оставляем маршрут 'storage' как просил, но рендерим LocationManager
        return <LocationManager user={user} />;
      default:
        return <Dashboard user={user} onNavigate={setCurrentPage} />;
    }
  };

  return (
    <ErrorBoundary>
      <div style={{ minHeight: '100vh', backgroundColor: '#f7fafc' }}>
        <Header 
          user={user}
          onLogout={handleLogout}
          currentPage={currentPage}
          setCurrentPage={setCurrentPage}
        />
        <main>
          {renderPage()}
        </main>
      </div>
    </ErrorBoundary>
  );
};

export default App;