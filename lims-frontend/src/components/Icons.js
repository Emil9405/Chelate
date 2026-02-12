// components/Icons.js - CHELATE LIMS SVG Icons
import React from 'react';

// Base Icon wrapper
const Icon = ({ children, size = 20, color = 'currentColor', style = {}, ...props }) => (
  <svg
    width={size}
    height={size}
    viewBox="0 0 24 24"
    fill="none"
    stroke={color}
    strokeWidth="2"
    strokeLinecap="round"
    strokeLinejoin="round"
    style={{ display: 'inline-block', verticalAlign: 'middle', ...style }}
    {...props}
  >
    {children}
  </svg>
);

// === Navigation Icons ===

export const HomeIcon = (props) => (
  <Icon {...props}>
    <path d="M3 9l9-7 9 7v11a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z" />
    <polyline points="9 22 9 12 15 12 15 22" />
  </Icon>
);

export const FlaskIcon = (props) => (
  <Icon {...props}>
    <path d="M9 3h6v6l4 9a2 2 0 0 1-1.8 2.9H6.8A2 2 0 0 1 5 18l4-9V3z" />
    <path d="M9 3h6" />
    <path d="M8 15h8" />
  </Icon>
);

export const MicroscopeIcon = (props) => (
  <Icon {...props}>
    <path d="M6 18h8" />
    <path d="M3 22h18" />
    <path d="M14 22a7 7 0 1 0 0-14h-1" />
    <path d="M9 14h2" />
    <path d="M9 12a2 2 0 0 1-2-2V6h6v4a2 2 0 0 1-2 2Z" />
    <path d="M12 6V3a1 1 0 0 0-1-1H9a1 1 0 0 0-1 1v3" />
  </Icon>
);

export const CogsIcon = (props) => (
  <Icon {...props}>
    <circle cx="12" cy="12" r="3" />
    <path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1 0 2.83 2 2 0 0 1-2.83 0l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-2 2 2 2 0 0 1-2-2v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83 0 2 2 0 0 1 0-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1-2-2 2 2 0 0 1 2-2h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 0-2.83 2 2 0 0 1 2.83 0l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 2-2 2 2 0 0 1 2 2v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 0 2 2 0 0 1 0 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 2 2 2 2 0 0 1-2 2h-.09a1.65 1.65 0 0 0-1.51 1z" />
  </Icon>
);

export const ChartBarIcon = (props) => (
  <Icon {...props}>
    <line x1="12" y1="20" x2="12" y2="10" />
    <line x1="18" y1="20" x2="18" y2="4" />
    <line x1="6" y1="20" x2="6" y2="14" />
  </Icon>
);

export const UsersIcon = (props) => (
  <Icon {...props}>
    <path d="M17 21v-2a4 4 0 0 0-4-4H5a4 4 0 0 0-4 4v2" />
    <circle cx="9" cy="7" r="4" />
    <path d="M23 21v-2a4 4 0 0 0-3-3.87" />
    <path d="M16 3.13a4 4 0 0 1 0 7.75" />
  </Icon>
);

// === Action Icons ===

export const UserIcon = (props) => (
  <Icon {...props}>
    <path d="M20 21v-2a4 4 0 0 0-4-4H8a4 4 0 0 0-4 4v2" />
    <circle cx="12" cy="7" r="4" />
  </Icon>
);

export const KeyIcon = (props) => (
  <Icon {...props}>
    <path d="M21 2l-2 2m-7.61 7.61a5.5 5.5 0 1 1-7.778 7.778 5.5 5.5 0 0 1 7.777-7.777zm0 0L15.5 7.5m0 0l3 3L22 7l-3-3m-3.5 3.5L19 4" />
  </Icon>
);

export const LockIcon = (props) => (
  <Icon {...props}>
    <rect x="3" y="11" width="18" height="11" rx="2" ry="2" />
    <path d="M7 11V7a5 5 0 0 1 10 0v4" />
  </Icon>
);

export const LogoutIcon = (props) => (
  <Icon {...props}>
    <path d="M9 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h4" />
    <polyline points="16 17 21 12 16 7" />
    <line x1="21" y1="12" x2="9" y2="12" />
  </Icon>
);

export const LoginIcon = (props) => (
  <Icon {...props}>
    <path d="M15 3h4a2 2 0 0 1 2 2v14a2 2 0 0 1-2 2h-4" />
    <polyline points="10 17 15 12 10 7" />
    <line x1="15" y1="12" x2="3" y2="12" />
  </Icon>
);

export const PlusIcon = (props) => (
  <Icon {...props}>
    <line x1="12" y1="5" x2="12" y2="19" />
    <line x1="5" y1="12" x2="19" y2="12" />
  </Icon>
);

export const EditIcon = (props) => (
  <Icon {...props}>
    <path d="M11 4H4a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2v-7" />
    <path d="M18.5 2.5a2.121 2.121 0 0 1 3 3L12 15l-4 1 1-4 9.5-9.5z" />
  </Icon>
);

export const TrashIcon = (props) => (
  <Icon {...props}>
    <polyline points="3 6 5 6 21 6" />
    <path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2" />
  </Icon>
);

export const SearchIcon = (props) => (
  <Icon {...props}>
    <circle cx="11" cy="11" r="8" />
    <line x1="21" y1="21" x2="16.65" y2="16.65" />
  </Icon>
);

export const FilterIcon = (props) => (
  <Icon {...props}>
    <polygon points="22 3 2 3 10 12.46 10 19 14 21 14 12.46 22 3" />
  </Icon>
);

export const DownloadIcon = (props) => (
  <Icon {...props}>
    <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" />
    <polyline points="7 10 12 15 17 10" />
    <line x1="12" y1="15" x2="12" y2="3" />
  </Icon>
);

export const UploadIcon = (props) => (
  <Icon {...props}>
    <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" />
    <polyline points="17 8 12 3 7 8" />
    <line x1="12" y1="3" x2="12" y2="15" />
  </Icon>
);

export const RefreshIcon = (props) => (
  <Icon {...props}>
    <polyline points="23 4 23 10 17 10" />
    <polyline points="1 20 1 14 7 14" />
    <path d="M3.51 9a9 9 0 0 1 14.85-3.36L23 10M1 14l4.64 4.36A9 9 0 0 0 20.49 15" />
  </Icon>
);

export const CloseIcon = (props) => (
  <Icon {...props}>
    <line x1="18" y1="6" x2="6" y2="18" />
    <line x1="6" y1="6" x2="18" y2="18" />
  </Icon>
);

export const CheckIcon = (props) => (
  <Icon {...props}>
    <polyline points="20 6 9 17 4 12" />
  </Icon>
);

export const SaveIcon = (props) => (
  <Icon {...props}>
    <path d="M19 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h11l5 5v11a2 2 0 0 1-2 2z" />
    <polyline points="17 21 17 13 7 13 7 21" />
    <polyline points="7 3 7 8 15 8" />
  </Icon>
);

export const ChevronDownIcon = (props) => (
  <Icon {...props}>
    <polyline points="6 9 12 15 18 9" />
  </Icon>
);

export const ChevronUpIcon = (props) => (
  <Icon {...props}>
    <polyline points="18 15 12 9 6 15" />
  </Icon>
);

export const ChevronLeftIcon = (props) => (
  <Icon {...props}>
    <polyline points="15 18 9 12 15 6" />
  </Icon>
);

export const ChevronRightIcon = (props) => (
  <Icon {...props}>
    <polyline points="9 18 15 12 9 6" />
  </Icon>
);

// === Status Icons ===

export const AlertCircleIcon = (props) => (
  <Icon {...props}>
    <circle cx="12" cy="12" r="10" />
    <line x1="12" y1="8" x2="12" y2="12" />
    <line x1="12" y1="16" x2="12.01" y2="16" />
  </Icon>
);

export const AlertTriangleIcon = (props) => (
  <Icon {...props}>
    <path d="M10.29 3.86L1.82 18a2 2 0 0 0 1.71 3h16.94a2 2 0 0 0 1.71-3L13.71 3.86a2 2 0 0 0-3.42 0z" />
    <line x1="12" y1="9" x2="12" y2="13" />
    <line x1="12" y1="17" x2="12.01" y2="17" />
  </Icon>
);

export const CheckCircleIcon = (props) => (
  <Icon {...props}>
    <path d="M22 11.08V12a10 10 0 1 1-5.93-9.14" />
    <polyline points="22 4 12 14.01 9 11.01" />
  </Icon>
);

export const InfoIcon = (props) => (
  <Icon {...props}>
    <circle cx="12" cy="12" r="10" />
    <line x1="12" y1="16" x2="12" y2="12" />
    <line x1="12" y1="8" x2="12.01" y2="8" />
  </Icon>
);

// === Data Icons ===

export const CalendarIcon = (props) => (
  <Icon {...props}>
    <rect x="3" y="4" width="18" height="18" rx="2" ry="2" />
    <line x1="16" y1="2" x2="16" y2="6" />
    <line x1="8" y1="2" x2="8" y2="6" />
    <line x1="3" y1="10" x2="21" y2="10" />
  </Icon>
);

export const ClockIcon = (props) => (
  <Icon {...props}>
    <circle cx="12" cy="12" r="10" />
    <polyline points="12 6 12 12 16 14" />
  </Icon>
);

export const FileIcon = (props) => (
  <Icon {...props}>
    <path d="M13 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V9z" />
    <polyline points="13 2 13 9 20 9" />
  </Icon>
);

export const FolderIcon = (props) => (
  <Icon {...props}>
    <path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z" />
  </Icon>
);

export const DatabaseIcon = (props) => (
  <Icon {...props}>
    <ellipse cx="12" cy="5" rx="9" ry="3" />
    <path d="M21 12c0 1.66-4 3-9 3s-9-1.34-9-3" />
    <path d="M3 5v14c0 1.66 4 3 9 3s9-1.34 9-3V5" />
  </Icon>
);

// === Arrow Icons ===

export const ArrowUpIcon = (props) => (
  <Icon {...props}>
    <line x1="12" y1="19" x2="12" y2="5" />
    <polyline points="5 12 12 5 19 12" />
  </Icon>
);

export const ArrowDownIcon = (props) => (
  <Icon {...props}>
    <line x1="12" y1="5" x2="12" y2="19" />
    <polyline points="19 12 12 19 5 12" />
  </Icon>
);

export const TrendUpIcon = (props) => (
  <Icon {...props}>
    <polyline points="23 6 13.5 15.5 8.5 10.5 1 18" />
    <polyline points="17 6 23 6 23 12" />
  </Icon>
);

export const TrendDownIcon = (props) => (
  <Icon {...props}>
    <polyline points="23 18 13.5 8.5 8.5 13.5 1 6" />
    <polyline points="17 18 23 18 23 12" />
  </Icon>
);

// === Misc Icons ===

export const MoreVerticalIcon = (props) => (
  <Icon {...props}>
    <circle cx="12" cy="12" r="1" />
    <circle cx="12" cy="5" r="1" />
    <circle cx="12" cy="19" r="1" />
  </Icon>
);

export const MoreHorizontalIcon = (props) => (
  <Icon {...props}>
    <circle cx="12" cy="12" r="1" />
    <circle cx="5" cy="12" r="1" />
    <circle cx="19" cy="12" r="1" />
  </Icon>
);

export const PrinterIcon = (props) => (
  <Icon {...props}>
    <polyline points="6 9 6 2 18 2 18 9" />
    <path d="M6 18H4a2 2 0 0 1-2-2v-5a2 2 0 0 1 2-2h16a2 2 0 0 1 2 2v5a2 2 0 0 1-2 2h-2" />
    <rect x="6" y="14" width="12" height="8" />
  </Icon>
);

export const CopyIcon = (props) => (
  <Icon {...props}>
    <rect x="9" y="9" width="13" height="13" rx="2" ry="2" />
    <path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1" />
  </Icon>
);

export const ExternalLinkIcon = (props) => (
  <Icon {...props}>
    <path d="M18 13v6a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h6" />
    <polyline points="15 3 21 3 21 9" />
    <line x1="10" y1="14" x2="21" y2="3" />
  </Icon>
);

export const EyeIcon = (props) => (
  <Icon {...props}>
    <path d="M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8z" />
    <circle cx="12" cy="12" r="3" />
  </Icon>
);

export const EyeOffIcon = (props) => (
  <Icon {...props}>
    <path d="M17.94 17.94A10.07 10.07 0 0 1 12 20c-7 0-11-8-11-8a18.45 18.45 0 0 1 5.06-5.94M9.9 4.24A9.12 9.12 0 0 1 12 4c7 0 11 8 11 8a18.5 18.5 0 0 1-2.16 3.19m-6.72-1.07a3 3 0 1 1-4.24-4.24" />
    <line x1="1" y1="1" x2="23" y2="23" />
  </Icon>
);

export const BellIcon = (props) => (
  <Icon {...props}>
    <path d="M18 8A6 6 0 0 0 6 8c0 7-3 9-3 9h18s-3-2-3-9" />
    <path d="M13.73 21a2 2 0 0 1-3.46 0" />
  </Icon>
);

export const SettingsIcon = (props) => (
  <Icon {...props}>
    <circle cx="12" cy="12" r="3" />
    <path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1 0 2.83 2 2 0 0 1-2.83 0l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-2 2 2 2 0 0 1-2-2v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83 0 2 2 0 0 1 0-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1-2-2 2 2 0 0 1 2-2h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 0-2.83 2 2 0 0 1 2.83 0l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 2-2 2 2 0 0 1 2 2v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 0 2 2 0 0 1 0 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 2 2 2 2 0 0 1-2 2h-.09a1.65 1.65 0 0 0-1.51 1z" />
  </Icon>
);

// === CHELATE Brand Logo ===

export const ChelateLogo = ({ size = 40, showText = false }) => (
  <div style={{ display: 'flex', alignItems: 'center', gap: '12px' }}>
    <svg width={size} height={size} viewBox="0 0 100 100">
      <defs>
        <linearGradient id="chelateGradient" x1="0%" y1="0%" x2="100%" y2="100%">
          <stop offset="0%" stopColor="#3182ce" />
          <stop offset="50%" stopColor="#38b2ac" />
          <stop offset="100%" stopColor="#38a169" />
        </linearGradient>
      </defs>
      
      {/* Background circle */}
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
      <path 
        d="M30 65 Q50 60 70 65" 
        fill="none" 
        stroke="rgba(255,255,255,0.5)" 
        strokeWidth="2"
      />
      
      {/* Bubbles */}
      <circle cx="42" cy="70" r="3" fill="rgba(255,255,255,0.6)" />
      <circle cx="55" cy="68" r="2" fill="rgba(255,255,255,0.5)" />
      <circle cx="48" cy="75" r="2.5" fill="rgba(255,255,255,0.4)" />
      
      {/* Molecular dots around */}
      <circle cx="15" cy="50" r="5" fill="rgba(255,255,255,0.3)" />
      <circle cx="85" cy="50" r="5" fill="rgba(255,255,255,0.3)" />
      <circle cx="50" cy="10" r="4" fill="rgba(255,255,255,0.3)" />
      <circle cx="25" cy="20" r="3" fill="rgba(255,255,255,0.2)" />
      <circle cx="75" cy="20" r="3" fill="rgba(255,255,255,0.2)" />
    </svg>
    
    {showText && (
      <div>
        <div style={{
          fontSize: size * 0.4,
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
          fontSize: size * 0.15,
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

// === Molecular Decoration SVG ===

export const MoleculeDecoration = ({ size = 60, opacity = 0.1 }) => (
  <svg width={size} height={size} viewBox="0 0 60 60" style={{ opacity }}>
    <circle cx="30" cy="30" r="8" fill="#3182ce" />
    <circle cx="10" cy="10" r="4" fill="#38a169" />
    <circle cx="50" cy="10" r="4" fill="#38b2ac" />
    <circle cx="10" cy="50" r="4" fill="#38b2ac" />
    <circle cx="50" cy="50" r="4" fill="#38a169" />
    <line x1="30" y1="30" x2="10" y2="10" stroke="#3182ce" strokeWidth="2" />
    <line x1="30" y1="30" x2="50" y2="10" stroke="#38b2ac" strokeWidth="2" />
    <line x1="30" y1="30" x2="10" y2="50" stroke="#38b2ac" strokeWidth="2" />
    <line x1="30" y1="30" x2="50" y2="50" stroke="#38a169" strokeWidth="2" />
  </svg>
);

// === Spinner ===

export const Spinner = ({ size = 24, color = '#3182ce' }) => (
  <svg
    width={size}
    height={size}
    viewBox="0 0 24 24"
    style={{ animation: 'spin 1s linear infinite' }}
  >
    <circle
      cx="12"
      cy="12"
      r="10"
      fill="none"
      stroke={color}
      strokeWidth="3"
      strokeDasharray="32"
      strokeLinecap="round"
      opacity="0.25"
    />
    <circle
      cx="12"
      cy="12"
      r="10"
      fill="none"
      stroke={color}
      strokeWidth="3"
      strokeDasharray="32"
      strokeDashoffset="24"
      strokeLinecap="round"
    />
  </svg>
);

// Export all icons as a collection
export const Icons = {
  Home: HomeIcon,
  Flask: FlaskIcon,
  Microscope: MicroscopeIcon,
  Cogs: CogsIcon,
  ChartBar: ChartBarIcon,
  Users: UsersIcon,
  User: UserIcon,
  Key: KeyIcon,
  Lock: LockIcon,
  Logout: LogoutIcon,
  Login: LoginIcon,
  Plus: PlusIcon,
  Edit: EditIcon,
  Trash: TrashIcon,
  Search: SearchIcon,
  Filter: FilterIcon,
  Download: DownloadIcon,
  Upload: UploadIcon,
  Refresh: RefreshIcon,
  Close: CloseIcon,
  Check: CheckIcon,
  Save: SaveIcon,
  ChevronDown: ChevronDownIcon,
  ChevronUp: ChevronUpIcon,
  ChevronLeft: ChevronLeftIcon,
  ChevronRight: ChevronRightIcon,
  AlertCircle: AlertCircleIcon,
  AlertTriangle: AlertTriangleIcon,
  CheckCircle: CheckCircleIcon,
  Info: InfoIcon,
  Calendar: CalendarIcon,
  Clock: ClockIcon,
  File: FileIcon,
  Folder: FolderIcon,
  Database: DatabaseIcon,
  ArrowUp: ArrowUpIcon,
  ArrowDown: ArrowDownIcon,
  TrendUp: TrendUpIcon,
  TrendDown: TrendDownIcon,
  MoreVertical: MoreVerticalIcon,
  MoreHorizontal: MoreHorizontalIcon,
  Printer: PrinterIcon,
  Copy: CopyIcon,
  ExternalLink: ExternalLinkIcon,
  Eye: EyeIcon,
  EyeOff: EyeOffIcon,
  Bell: BellIcon,
  Settings: SettingsIcon,
  ChelateLogo,
  MoleculeDecoration,
  Spinner
};

export default Icons;
