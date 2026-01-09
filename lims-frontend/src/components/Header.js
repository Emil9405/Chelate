// components/Header.js - CHELATE LIMS Style with SVG Icons
import React, { useState, useEffect, useCallback } from 'react';
import { ChangePasswordModal } from './Modals';
import { 
  HomeIcon, 
  FlaskIcon, 
  MicroscopeIcon, 
  CogsIcon, 
  ChartBarIcon, 
  UsersIcon,
  KeyIcon,
  LogoutIcon,
  ChevronDownIcon
} from './Icons';
import ChelateLogo from './ChelateLogo';

const Header = ({ user, onLogout, currentPage, setCurrentPage }) => {
  const [showChangePassword, setShowChangePassword] = useState(false);
  const [scrolled, setScrolled] = useState(false);

  useEffect(() => {
    const handleScroll = () => {
      setScrolled(window.scrollY > 10);
    };
    window.addEventListener('scroll', handleScroll);
    return () => window.removeEventListener('scroll', handleScroll);
  }, []);

  const navItems = [
    { id: 'dashboard', label: 'Dashboard', Icon: HomeIcon },
    { id: 'reagents', label: 'Reagents', Icon: FlaskIcon },
    { id: 'experiments', label: 'Experiments', Icon: MicroscopeIcon },
    { id: 'equipment', label: 'Equipment', Icon: CogsIcon },
    { id: 'reports', label: 'Reports', Icon: ChartBarIcon },
    { id: 'users', label: 'Users', Icon: UsersIcon }
  ];

  const handlePasswordChangeSuccess = useCallback(() => {
    setShowChangePassword(false);
  }, []);

  const getUserInitials = useCallback(() => {
    if (!user?.username) return 'U';
    return user.username.slice(0, 2).toUpperCase();
  }, [user?.username]);

  return (
    <>
      <header style={{
        background: scrolled ? 'rgba(255, 255, 255, 0.98)' : 'rgba(255, 255, 255, 0.95)',
        backdropFilter: 'blur(12px)',
        boxShadow: scrolled 
          ? '0 4px 20px rgba(26, 54, 93, 0.12)' 
          : '0 1px 3px rgba(26, 54, 93, 0.08)',
        padding: '0 2rem',
        position: 'fixed',
        top: 0,
        left: 0,
        right: 0,
        zIndex: 1000,
        height: scrolled ? '60px' : '70px',
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'space-between',
        borderBottom: '1px solid rgba(226, 232, 240, 0.8)',
        transition: 'all 0.3s cubic-bezier(0.4, 0, 0.2, 1)'
      }}>
        {/* Logo */}
        <div 
          style={{
            display: 'flex',
            alignItems: 'center',
            gap: '12px',
            cursor: 'pointer'
          }}
          onClick={() => setCurrentPage('dashboard')}
        >
          <ChelateLogo src="/logo.png" size={42} showText={true} />
        </div>
        
        {/* Navigation */}
        <nav style={{ 
          display: 'flex', 
          gap: '4px',
          background: 'rgba(247, 250, 252, 0.8)',
          padding: '4px',
          borderRadius: '12px'
        }}>
          {navItems.map(item => {
            const IconComponent = item.Icon;
            const isActive = currentPage === item.id;
            
            return (
              <button
                key={item.id}
                onClick={() => setCurrentPage(item.id)}
                style={{
                  background: isActive ? 'white' : 'transparent',
                  border: 'none',
                  color: isActive ? '#3182ce' : '#718096',
                  cursor: 'pointer',
                  padding: '10px 16px',
                  borderRadius: '8px',
                  fontSize: '0.875rem',
                  fontWeight: isActive ? '600' : '500',
                  transition: 'all 0.2s ease',
                  display: 'flex',
                  alignItems: 'center',
                  gap: '8px',
                  boxShadow: isActive ? '0 2px 8px rgba(49, 130, 206, 0.15)' : 'none',
                  position: 'relative'
                }}
              >
                <IconComponent 
                  size={18} 
                  color={isActive ? '#38b2ac' : '#718096'} 
                />
                <span>{item.label}</span>
                {isActive && (
                  <span style={{
                    position: 'absolute',
                    bottom: '4px',
                    left: '50%',
                    transform: 'translateX(-50%)',
                    width: '20px',
                    height: '3px',
                    background: 'linear-gradient(90deg, #3182ce, #38a169)',
                    borderRadius: '2px'
                  }}></span>
                )}
              </button>
            );
          })}
        </nav>
        
        {/* User Section */}
        <div style={{ display: 'flex', alignItems: 'center', gap: '12px' }}>
          {/* User Info */}
          <div style={{ 
            display: 'flex', 
            alignItems: 'center', 
            gap: '12px',
            padding: '6px 12px',
            background: 'rgba(49, 130, 206, 0.05)',
            borderRadius: '10px',
            cursor: 'pointer',
            transition: 'all 0.2s ease'
          }}>
            {/* User Avatar */}
            <div style={{
              width: '36px',
              height: '36px',
              background: 'linear-gradient(135deg, #3182ce 0%, #38b2ac 100%)',
              borderRadius: '50%',
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
              color: 'white',
              fontWeight: '700',
              fontSize: '0.875rem',
              boxShadow: '0 2px 8px rgba(49, 130, 206, 0.3)'
            }}>
              {getUserInitials()}
            </div>
            
            <div style={{ textAlign: 'left' }}>
              <div style={{ 
                fontSize: '0.875rem', 
                fontWeight: '600',
                color: '#1a365d'
              }}>
                {user?.username}
              </div>
              <div style={{
                fontSize: '0.7rem',
                color: '#718096',
                textTransform: 'uppercase',
                letterSpacing: '0.05em'
              }}>
                {user?.role}
              </div>
            </div>
            
            <ChevronDownIcon size={16} color="#718096" />
          </div>
          
          {/* Change Password Button */}
          <button 
            onClick={() => setShowChangePassword(true)}
            style={{
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
              width: '38px',
              height: '38px',
              background: 'white',
              border: '2px solid #e2e8f0',
              borderRadius: '10px',
              color: '#718096',
              cursor: 'pointer',
              transition: 'all 0.2s ease'
            }}
            title="Change Password"
          >
            <KeyIcon size={18} />
          </button>
          
          {/* Logout Button */}
          <button 
            onClick={onLogout}
            style={{
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
              width: '38px',
              height: '38px',
              background: 'linear-gradient(135deg, rgba(229, 62, 62, 0.1) 0%, rgba(197, 48, 48, 0.1) 100%)',
              border: 'none',
              borderRadius: '10px',
              color: '#e53e3e',
              cursor: 'pointer',
              transition: 'all 0.2s ease'
            }}
            title="Logout"
          >
            <LogoutIcon size={18} />
          </button>
        </div>
      </header>
      
      {showChangePassword && (
        <ChangePasswordModal 
          isOpen={showChangePassword}
          onClose={() => setShowChangePassword(false)}
          onSave={handlePasswordChangeSuccess}
        />
      )}
    </>
  );
};

export default Header;
