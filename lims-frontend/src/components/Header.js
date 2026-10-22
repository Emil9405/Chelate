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
  ChevronDownIcon,
  ArchiveIcon, // <-- Добавили иконку для Storage
  TrashIcon    // <-- Иконка для архива (корзины)
} from './Icons';
import ChelateLogo from './ChelateLogo';
import useIsMobile from './hooks/useIsMobile';
import { ScanIcon } from './labels/LabelIcons';

// Phone layout: these sections get their own tab, the rest go under "More"
const MOBILE_TABS = ['dashboard', 'reagents', 'experiments', 'storage'];

const MoreIcon = ({ size = 22, color = 'currentColor' }) => (
  <svg width={size} height={size} viewBox="0 0 24 24" fill={color} aria-hidden="true">
    <circle cx="5" cy="12" r="2" /><circle cx="12" cy="12" r="2" /><circle cx="19" cy="12" r="2" />
  </svg>
);

const Header = ({ user, onLogout, currentPage, setCurrentPage }) => {
  const [showChangePassword, setShowChangePassword] = useState(false);
  const [scrolled, setScrolled] = useState(false);
  const [moreOpen, setMoreOpen] = useState(false);
  const isMobile = useIsMobile();
  // Full labels need ~1400px; below that the nav shows icons only (with tooltips),
  // and below 1024px the logo text and user name are hidden too.
  const compactNav = useIsMobile('(max-width: 1439px)');
  const narrowHeader = useIsMobile('(max-width: 1023px)');

  // Reserve space under the content for the bottom tab bar
  useEffect(() => {
    document.body.classList.toggle('has-mobile-nav', isMobile);
    return () => document.body.classList.remove('has-mobile-nav');
  }, [isMobile]);

  // Close the "More" sheet when switching pages or leaving phone layout
  useEffect(() => { setMoreOpen(false); }, [currentPage, isMobile]);

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
    { id: 'storage', label: 'Storage', Icon: ArchiveIcon }, // <-- Добавили кнопку Storage
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

  const isAdmin = user?.role?.toLowerCase() === 'admin';

  if (isMobile) {
    const tabItems = navItems.filter(i => MOBILE_TABS.includes(i.id));
    const moreItems = [
      ...navItems.filter(i => !MOBILE_TABS.includes(i.id)),
      ...(isAdmin ? [{ id: 'archive', label: 'Archive', Icon: TrashIcon }] : []),
    ];
    const moreActive = moreItems.some(i => i.id === currentPage);

    const iconButton = {
      display: 'flex', alignItems: 'center', justifyContent: 'center',
      width: '40px', height: '40px', borderRadius: '10px', cursor: 'pointer',
      background: 'white', border: '2px solid #e2e8f0', color: '#718096',
    };

    const tabStyle = (active) => ({
      flex: 1, minWidth: 0, height: '56px',
      display: 'flex', flexDirection: 'column', alignItems: 'center', justifyContent: 'center', gap: '3px',
      background: 'transparent', border: 'none', cursor: 'pointer',
      color: active ? '#3182ce' : '#718096',
      fontSize: '0.68rem', fontWeight: active ? 700 : 500,
      position: 'relative',
    });

    return (
      <>
        <header style={{
          position: 'fixed', top: 0, left: 0, right: 0, zIndex: 1000,
          height: '56px', padding: '0 12px',
          display: 'flex', alignItems: 'center', justifyContent: 'space-between',
          background: 'rgba(255, 255, 255, 0.98)', backdropFilter: 'blur(12px)',
          boxShadow: scrolled ? '0 4px 20px rgba(26, 54, 93, 0.12)' : '0 1px 3px rgba(26, 54, 93, 0.08)',
          borderBottom: '1px solid rgba(226, 232, 240, 0.8)',
        }}>
          <div style={{ display: 'flex', alignItems: 'center', cursor: 'pointer', minWidth: 0 }} onClick={() => setCurrentPage('dashboard')}>
            <ChelateLogo src={`/logo.png?v=${process.env.REACT_APP_BUILD_ID || Date.now()}`} size={34} showText={false} />
          </div>
          <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
            {/* Scan a bottle label: the main phone action in the lab */}
            <button
              onClick={() => setCurrentPage('scan')}
              style={{
                ...iconButton, width: 'auto', padding: '0 12px', gap: '6px', border: 'none',
                background: currentPage === 'scan' ? '#2c5282' : 'linear-gradient(135deg, #3182ce 0%, #38b2ac 100%)',
                color: 'white', fontWeight: 700, fontSize: '0.85rem',
              }}
              title="Scan label"
              aria-label="Scan label"
            >
              <ScanIcon size={18} color="white" />
              Scan
            </button>
            <div
              title={`${user?.username || ''} (${user?.role || ''})`}
              style={{
                width: '36px', height: '36px', borderRadius: '50%',
                background: 'linear-gradient(135deg, #3182ce 0%, #38b2ac 100%)',
                display: 'flex', alignItems: 'center', justifyContent: 'center',
                color: 'white', fontWeight: 700, fontSize: '0.8rem',
              }}
            >
              {getUserInitials()}
            </div>
            <button onClick={() => setShowChangePassword(true)} style={iconButton} title="Change Password" aria-label="Change Password">
              <KeyIcon size={18} />
            </button>
            <button
              onClick={onLogout}
              style={{ ...iconButton, border: 'none', color: '#e53e3e', background: 'rgba(229, 62, 62, 0.1)' }}
              title="Logout"
              aria-label="Logout"
            >
              <LogoutIcon size={18} />
            </button>
          </div>
        </header>

        {/* "More" sheet */}
        {moreOpen && (
          <div
            onClick={() => setMoreOpen(false)}
            style={{ position: 'fixed', inset: 0, zIndex: 999, background: 'rgba(26, 54, 93, 0.35)' }}
          >
            <div
              onClick={e => e.stopPropagation()}
              style={{
                position: 'absolute', left: 0, right: 0,
                bottom: 'calc(64px + env(safe-area-inset-bottom))',
                background: 'white', borderRadius: '16px 16px 0 0', padding: '8px',
                boxShadow: '0 -8px 30px rgba(26, 54, 93, 0.18)',
              }}
            >
              {moreItems.map(item => {
                const Icon = item.Icon;
                const active = currentPage === item.id;
                return (
                  <button
                    key={item.id}
                    onClick={() => setCurrentPage(item.id)}
                    style={{
                      width: '100%', height: '52px', padding: '0 16px',
                      display: 'flex', alignItems: 'center', gap: '14px',
                      border: 'none', borderRadius: '10px', cursor: 'pointer',
                      background: active ? 'rgba(49, 130, 206, 0.08)' : 'transparent',
                      color: active ? '#3182ce' : '#1a365d',
                      fontSize: '1rem', fontWeight: active ? 700 : 500, textAlign: 'left',
                    }}
                  >
                    <Icon size={20} color={active ? '#38b2ac' : '#718096'} />
                    {item.label}
                  </button>
                );
              })}
            </div>
          </div>
        )}

        {/* Bottom tab bar */}
        <nav style={{
          position: 'fixed', left: 0, right: 0, bottom: 0, zIndex: 1000,
          display: 'flex', alignItems: 'stretch',
          height: 'calc(64px + env(safe-area-inset-bottom))',
          paddingBottom: 'env(safe-area-inset-bottom)', paddingTop: '4px',
          background: 'rgba(255, 255, 255, 0.98)', backdropFilter: 'blur(12px)',
          borderTop: '1px solid #e2e8f0', boxShadow: '0 -2px 10px rgba(26, 54, 93, 0.06)',
        }}>
          {tabItems.map(item => {
            const Icon = item.Icon;
            const active = currentPage === item.id;
            return (
              <button key={item.id} onClick={() => setCurrentPage(item.id)} style={tabStyle(active)} aria-current={active ? 'page' : undefined}>
                <Icon size={22} color={active ? '#38b2ac' : '#718096'} />
                <span style={{ maxWidth: '100%', overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>{item.label}</span>
              </button>
            );
          })}
          <button onClick={() => setMoreOpen(o => !o)} style={tabStyle(moreActive || moreOpen)} aria-expanded={moreOpen}>
            <MoreIcon size={22} color={moreActive || moreOpen ? '#38b2ac' : '#718096'} />
            <span>More</span>
          </button>
        </nav>

        {showChangePassword && (
          <ChangePasswordModal
            isOpen={showChangePassword}
            onClose={() => setShowChangePassword(false)}
            onSave={handlePasswordChangeSuccess}
          />
        )}
      </>
    );
  }

  return (
    <>
      <header style={{
        background: scrolled ? 'rgba(255, 255, 255, 0.98)' : 'rgba(255, 255, 255, 0.95)',
        backdropFilter: 'blur(12px)',
        boxShadow: scrolled 
          ? '0 4px 20px rgba(26, 54, 93, 0.12)' 
          : '0 1px 3px rgba(26, 54, 93, 0.08)',
        padding: narrowHeader ? '0 1rem' : '0 2rem',
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
          <ChelateLogo src={`/logo.png?v=${process.env.REACT_APP_BUILD_ID || Date.now()}`} size={42} showText={!narrowHeader} />
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
                title={compactNav ? item.label : undefined}
                aria-label={item.label}
                style={{
                  background: isActive ? 'white' : 'transparent',
                  border: 'none',
                  color: isActive ? '#3182ce' : '#718096',
                  cursor: 'pointer',
                  padding: compactNav ? '10px 12px' : '10px 16px',
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
                {!compactNav && <span>{item.label}</span>}
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
            
            {!narrowHeader && (
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
            )}
            
            <ChevronDownIcon size={16} color="#718096" />
          </div>
          
          {/* Scan label (camera, USB scanner or typed code) */}
          <button
            onClick={() => setCurrentPage('scan')}
            style={{
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
              width: '38px',
              height: '38px',
              background: currentPage === 'scan' ? 'rgba(49, 130, 206, 0.12)' : 'white',
              border: currentPage === 'scan' ? '2px solid #3182ce' : '2px solid #e2e8f0',
              borderRadius: '10px',
              color: currentPage === 'scan' ? '#3182ce' : '#718096',
              cursor: 'pointer',
              transition: 'all 0.2s ease'
            }}
            title="Scan label"
            aria-label="Scan label"
          >
            <ScanIcon size={18} />
          </button>

          {/* Archive (admin only) */}
          {isAdmin && (
            <button 
              onClick={() => setCurrentPage('archive')}
              style={{
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'center',
                width: '38px',
                height: '38px',
                background: currentPage === 'archive'
                  ? 'linear-gradient(135deg, rgba(229, 62, 62, 0.15) 0%, rgba(197, 48, 48, 0.15) 100%)'
                  : 'white',
                border: currentPage === 'archive' ? '2px solid #e53e3e' : '2px solid #e2e8f0',
                borderRadius: '10px',
                color: currentPage === 'archive' ? '#e53e3e' : '#718096',
                cursor: 'pointer',
                transition: 'all 0.2s ease',
                position: 'relative'
              }}
              title="Archive (Trash)"
            >
              <TrashIcon size={18} />
              {/* Red dot indicator could be added here if archive has items */}
            </button>
          )}

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