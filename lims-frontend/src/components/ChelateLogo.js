// components/ChelateLogo.js - Поддержка PNG/SVG логотипа
import React from 'react';

/**
 * CHELATE Logo Component
 * 
 * Использование:
 * 1. С внешним изображением: <ChelateLogo src="/logo.png" size={50} />
 * 2. Встроенный SVG (fallback): <ChelateLogo size={50} />
 * 
 * Положите логотип в: public/logo.png или public/logo.svg
 */

const ChelateLogo = ({ 
  src,              // путь к изображению: "/logo.png" или "/logo.svg"
  size = 40, 
  showText = false,
  textSize,         // размер текста (по умолчанию size * 0.4)
  className = '',
  style = {}
}) => {
  const [imageError, setImageError] = React.useState(false);
  
  // Fallback SVG если изображение не загрузилось
  const FallbackSVG = () => (
    <svg width={size} height={size} viewBox="0 0 100 100">
      <defs>
        <linearGradient id="chelateGradient" x1="0%" y1="0%" x2="100%" y2="100%">
          <stop offset="0%" stopColor="#3182ce" />
          <stop offset="50%" stopColor="#38b2ac" />
          <stop offset="100%" stopColor="#38a169" />
        </linearGradient>
      </defs>
      
      {/* Background */}
      <rect x="5" y="5" width="90" height="90" rx="20" fill="url(#chelateGradient)" />
      
      {/* Flask */}
      <path 
        d="M40 25 L40 45 L25 75 Q22 82 28 85 L72 85 Q78 82 75 75 L60 45 L60 25 Z" 
        fill="none" 
        stroke="white" 
        strokeWidth="4"
        strokeLinejoin="round"
      />
      
      {/* Flask top */}
      <line x1="35" y1="25" x2="65" y2="25" stroke="white" strokeWidth="4" strokeLinecap="round" />
      
      {/* Liquid level */}
      <path d="M30 65 Q50 60 70 65" fill="none" stroke="rgba(255,255,255,0.5)" strokeWidth="2" />
      
      {/* Bubbles */}
      <circle cx="42" cy="70" r="3" fill="rgba(255,255,255,0.6)" />
      <circle cx="55" cy="68" r="2" fill="rgba(255,255,255,0.5)" />
      <circle cx="48" cy="75" r="2.5" fill="rgba(255,255,255,0.4)" />
      
      {/* Molecular dots */}
      <circle cx="15" cy="50" r="5" fill="rgba(255,255,255,0.3)" />
      <circle cx="85" cy="50" r="5" fill="rgba(255,255,255,0.3)" />
      <circle cx="50" cy="10" r="4" fill="rgba(255,255,255,0.3)" />
    </svg>
  );

  const actualTextSize = textSize || size * 0.4;

  return (
    <div 
      className={className}
      style={{ 
        display: 'flex', 
        alignItems: 'center', 
        gap: '12px',
        ...style 
      }}
    >
      {/* Logo Image or Fallback SVG */}
      {src && !imageError ? (
        <img 
          src={src} 
          alt="CHELATE" 
          width={size} 
          height={size}
          style={{ 
            objectFit: 'contain',
            borderRadius: size * 0.2
          }}
          onError={() => setImageError(true)}
        />
      ) : (
        <FallbackSVG />
      )}
      
      {/* Text */}
      {showText && (
        <div>
          <div style={{
            fontSize: actualTextSize,
            fontWeight: 800,
            background: 'linear-gradient(135deg, #3182ce 0%, #38b2ac 50%, #38a169 100%)',
            WebkitBackgroundClip: 'text',
            WebkitTextFillColor: 'transparent',
            backgroundClip: 'text',
            letterSpacing: '0.05em',
            lineHeight: 1
          }}>
            CHELATE
          </div>
          <div style={{
            fontSize: actualTextSize * 0.375,
            color: '#718096',
            textTransform: 'uppercase',
            letterSpacing: '0.15em',
            marginTop: '2px'
          }}>
            Laboratory Management
          </div>
        </div>
      )}
    </div>
  );
};

export default ChelateLogo;
