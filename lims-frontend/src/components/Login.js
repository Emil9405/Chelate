// components/Login.js - CHELATE LIMS Login with SVG Icons
import React, { useState } from 'react';
import { api } from '../services/api';
import ErrorMessage from './ErrorMessage';
import SuccessMessage from './SuccessMessage';
import { UserIcon, LockIcon, LoginIcon, Spinner } from './Icons';
import ChelateLogo from './ChelateLogo';

const Login = ({ onLogin }) => {
  const [credentials, setCredentials] = useState({ username: '', password: '' });
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState('');
  const [success, setSuccess] = useState('');

  const handleSubmit = async (e) => {
    e.preventDefault();
    setLoading(true);
    setError('');
    setSuccess('');

    try {
      const user = await api.login(credentials);
      setSuccess('Login successful!');
      setTimeout(() => onLogin(user), 1000);
    } catch (err) {
      setError(err.message || 'Login failed');
    } finally {
      setLoading(false);
    }
  };

  // Inline styles for animations
  const animationStyles = `
    @keyframes gradientShift {
      0% { background-position: 0% 50%; }
      50% { background-position: 100% 50%; }
      100% { background-position: 0% 50%; }
    }
    
    @keyframes fadeInUp {
      from {
        opacity: 0;
        transform: translateY(30px);
      }
      to {
        opacity: 1;
        transform: translateY(0);
      }
    }
    
    @keyframes spin {
      to { transform: rotate(360deg); }
    }
    
    @keyframes float {
      0%, 100% { transform: translateY(0) rotate(0deg); }
      50% { transform: translateY(-20px) rotate(10deg); }
    }
  `;

  return (
    <div style={{
      minHeight: '100vh',
      display: 'flex',
      alignItems: 'center',
      justifyContent: 'center',
      background: 'linear-gradient(135deg, #1a365d 0%, #2c5282 30%, #3182ce 60%, #38b2ac 80%, #38a169 100%)',
      backgroundSize: '400% 400%',
      animation: 'gradientShift 15s ease infinite',
      padding: '20px',
      position: 'relative',
      overflow: 'hidden'
    }}>
      <style>{animationStyles}</style>
      
      {/* Hexagonal Pattern Overlay */}
      <div style={{
        position: 'absolute',
        top: 0,
        left: 0,
        right: 0,
        bottom: 0,
        backgroundImage: `url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='60' height='52' viewBox='0 0 60 52'%3E%3Cpath fill='%23ffffff' fill-opacity='0.05' d='M30 0l30 15v22l-30 15L0 37V15L30 0zm0 4.33L4 17.67v16.66L30 47.67l26-13.34V17.67L30 4.33z'/%3E%3C/svg%3E")`,
        pointerEvents: 'none'
      }}></div>

      {/* Floating Molecule SVG - Top Left */}
      <svg 
        width="200" 
        height="200" 
        viewBox="0 0 100 100"
        style={{
          position: 'absolute',
          top: '10%',
          left: '10%',
          opacity: 0.1,
          animation: 'float 6s ease-in-out infinite'
        }}
      >
        <circle cx="50" cy="50" r="15" fill="white"/>
        <circle cx="20" cy="30" r="8" fill="white"/>
        <circle cx="80" cy="30" r="8" fill="white"/>
        <circle cx="35" cy="80" r="8" fill="white"/>
        <circle cx="65" cy="80" r="8" fill="white"/>
        <line x1="50" y1="50" x2="20" y2="30" stroke="white" strokeWidth="3"/>
        <line x1="50" y1="50" x2="80" y2="30" stroke="white" strokeWidth="3"/>
        <line x1="50" y1="50" x2="35" y2="80" stroke="white" strokeWidth="3"/>
        <line x1="50" y1="50" x2="65" y2="80" stroke="white" strokeWidth="3"/>
      </svg>

      {/* Floating Molecule SVG - Bottom Right */}
      <svg 
        width="150" 
        height="150" 
        viewBox="0 0 100 100"
        style={{
          position: 'absolute',
          bottom: '15%',
          right: '8%',
          opacity: 0.08,
          transform: 'rotate(30deg)',
          animation: 'float 8s ease-in-out infinite reverse'
        }}
      >
        <circle cx="50" cy="50" r="12" fill="white"/>
        <circle cx="25" cy="25" r="6" fill="white"/>
        <circle cx="75" cy="25" r="6" fill="white"/>
        <circle cx="25" cy="75" r="6" fill="white"/>
        <circle cx="75" cy="75" r="6" fill="white"/>
        <line x1="50" y1="50" x2="25" y2="25" stroke="white" strokeWidth="2"/>
        <line x1="50" y1="50" x2="75" y2="25" stroke="white" strokeWidth="2"/>
        <line x1="50" y1="50" x2="25" y2="75" stroke="white" strokeWidth="2"/>
        <line x1="50" y1="50" x2="75" y2="75" stroke="white" strokeWidth="2"/>
      </svg>

      {/* Login Card */}
      <div style={{
        background: 'rgba(255, 255, 255, 0.95)',
        backdropFilter: 'blur(20px)',
        borderRadius: '24px',
        boxShadow: '0 25px 60px rgba(26, 54, 93, 0.3)',
        padding: '50px 45px',
        width: '100%',
        maxWidth: '420px',
        textAlign: 'center',
        position: 'relative',
        border: '1px solid rgba(255, 255, 255, 0.5)',
        animation: 'fadeInUp 0.6s ease'
      }}>
        {/* Top gradient border */}
        <div style={{
          position: 'absolute',
          top: 0,
          left: '10%',
          right: '10%',
          height: '4px',
          background: 'linear-gradient(90deg, #3182ce 0%, #38b2ac 50%, #38a169 100%)',
          borderRadius: '0 0 4px 4px'
        }}></div>

        {/* Logo */}
        <div style={{ marginBottom: '30px' }}>
          <div style={{ display: 'flex', justifyContent: 'center', marginBottom: '20px' }}>
            {/* Используй src="/logo.png" если есть файл в public/ */}
            <ChelateLogo src="/logo.png" size={80} />
          </div>

          <h1 style={{
            fontSize: '2.5rem',
            fontWeight: '800',
            background: 'linear-gradient(135deg, #3182ce 0%, #38b2ac 50%, #38a169 100%)',
            WebkitBackgroundClip: 'text',
            WebkitTextFillColor: 'transparent',
            backgroundClip: 'text',
            letterSpacing: '0.08em',
            marginBottom: '8px'
          }}>
            CHELATE
          </h1>
          <p style={{
            color: '#718096',
            fontSize: '0.875rem',
            fontWeight: '500',
            textTransform: 'uppercase',
            letterSpacing: '0.15em'
          }}>
            Laboratory Information Management
          </p>
        </div>
        
        {error && <ErrorMessage message={error} onDismiss={() => setError('')} />}
        {success && <SuccessMessage message={success} onDismiss={() => setSuccess('')} />}
        
        <form onSubmit={handleSubmit} style={{ textAlign: 'left' }}>
          {/* Username Field */}
          <div style={{ marginBottom: '20px' }}>
            <label style={{
              display: 'flex',
              alignItems: 'center',
              gap: '8px',
              marginBottom: '8px',
              color: '#1a365d',
              fontWeight: '600',
              fontSize: '0.875rem'
            }}>
              <UserIcon size={16} color="#3182ce" />
              Username
            </label>
            <input
              type="text"
              value={credentials.username}
              onChange={(e) => setCredentials({ ...credentials, username: e.target.value })}
              placeholder="Enter your username"
              required
              style={{
                width: '100%',
                padding: '14px 16px',
                border: '2px solid #e2e8f0',
                borderRadius: '12px',
                fontSize: '1rem',
                transition: 'all 0.2s ease',
                outline: 'none',
                background: '#f8fafc',
                boxSizing: 'border-box'
              }}
              onFocus={(e) => {
                e.target.style.borderColor = '#3182ce';
                e.target.style.boxShadow = '0 0 0 4px rgba(49, 130, 206, 0.1)';
                e.target.style.background = 'white';
              }}
              onBlur={(e) => {
                e.target.style.borderColor = '#e2e8f0';
                e.target.style.boxShadow = 'none';
                e.target.style.background = '#f8fafc';
              }}
            />
          </div>
          
          {/* Password Field */}
          <div style={{ marginBottom: '25px' }}>
            <label style={{
              display: 'flex',
              alignItems: 'center',
              gap: '8px',
              marginBottom: '8px',
              color: '#1a365d',
              fontWeight: '600',
              fontSize: '0.875rem'
            }}>
              <LockIcon size={16} color="#38b2ac" />
              Password
            </label>
            <input
              type="password"
              value={credentials.password}
              onChange={(e) => setCredentials({ ...credentials, password: e.target.value })}
              placeholder="Enter your password"
              required
              style={{
                width: '100%',
                padding: '14px 16px',
                border: '2px solid #e2e8f0',
                borderRadius: '12px',
                fontSize: '1rem',
                transition: 'all 0.2s ease',
                outline: 'none',
                background: '#f8fafc',
                boxSizing: 'border-box'
              }}
              onFocus={(e) => {
                e.target.style.borderColor = '#38b2ac';
                e.target.style.boxShadow = '0 0 0 4px rgba(56, 178, 172, 0.1)';
                e.target.style.background = 'white';
              }}
              onBlur={(e) => {
                e.target.style.borderColor = '#e2e8f0';
                e.target.style.boxShadow = 'none';
                e.target.style.background = '#f8fafc';
              }}
            />
          </div>
          
          {/* Submit Button */}
          <button
            type="submit"
            disabled={loading}
            style={{
              width: '100%',
              padding: '16px',
              background: loading 
                ? '#a0aec0'
                : 'linear-gradient(135deg, #3182ce 0%, #38b2ac 50%, #38a169 100%)',
              color: 'white',
              border: 'none',
              borderRadius: '12px',
              fontSize: '1rem',
              fontWeight: '700',
              cursor: loading ? 'not-allowed' : 'pointer',
              transition: 'all 0.3s ease',
              boxShadow: loading ? 'none' : '0 8px 25px rgba(49, 130, 206, 0.35)',
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
              gap: '10px',
              letterSpacing: '0.02em'
            }}
          >
            {loading ? (
              <>
                <Spinner size={20} color="white" />
                Signing In...
              </>
            ) : (
              <>
                <LoginIcon size={20} color="white" />
                Sign In
              </>
            )}
          </button>
        </form>

        {/* Footer */}
        <div style={{
          marginTop: '30px',
          paddingTop: '20px',
          borderTop: '1px solid #e2e8f0'
        }}>
          <p style={{
            color: '#a0aec0',
            fontSize: '0.75rem',
            margin: 0
          }}>
            © 2024 CHELATE LIMS • Secure Laboratory Management
          </p>
        </div>
      </div>
    </div>
  );
};

export default Login;
