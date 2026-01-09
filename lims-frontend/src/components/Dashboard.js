// components/Dashboard.js - CHELATE Style with SVG Icons
import React, { useState, useEffect } from 'react';
import { api } from '../services/api';
import StatsCard from './StatsCard';
import LoadingOverlay from './LoadingOverlay';
import Button from './Button';
import {
  FlaskIcon,
  DatabaseIcon,
  AlertTriangleIcon,
  ClockIcon,
  RefreshIcon,
  PlusIcon,
  ChartBarIcon,
  CogsIcon,
  UsersIcon,
  CheckCircleIcon,
  TrendUpIcon,
  CalendarIcon
} from './Icons';
import './Dashboard.css';

const Dashboard = ({ user, showToast, onNavigate }) => {
  const [stats, setStats] = useState({
    total_reagents: 0,
    total_batches: 0,
    low_stock: 0,
    expiring_soon: 0
  });
  const [loading, setLoading] = useState(true);
  const [recentActivity, setRecentActivity] = useState([]);
  const [quickActions, setQuickActions] = useState([]);

  useEffect(() => {
    loadDashboardData();
    loadRecentActivity();
    setupQuickActions();
  }, [user]);

  const loadDashboardData = async () => {
    try {
      setLoading(true);
      
      const [statsData, reagentsData, batchesData, lowStockData, expiringData] = await Promise.all([
        api.getDashboardStats().catch(() => null),
        api.getReagents({ page: 1, per_page: 1 }).catch(() => null),
        api.getAllBatches({ page: 1, per_page: 1 }).catch(() => null),
        api.getLowStockItems(10).catch(() => []),
        api.getExpiringItems(30).catch(() => [])
      ]);

      const newStats = {
        total_reagents: statsData?.total_reagents || reagentsData?.total || 0,
        total_batches: statsData?.total_batches || batchesData?.total || 0,
        low_stock: statsData?.low_stock || lowStockData?.length || 0,
        expiring_soon: statsData?.expiring_soon || expiringData?.length || 0
      };

      setStats(newStats);
      if (showToast) {
        showToast('Dashboard data loaded successfully', 'success');
      }
    } catch (error) {
      console.error('Failed to load dashboard data:', error);
      if (showToast) {
        showToast('Failed to load dashboard data', 'error');
      }
    } finally {
      setLoading(false);
    }
  };

  const loadRecentActivity = async () => {
    const activities = [
      { 
        id: 1, 
        type: 'reagent_added', 
        message: 'New reagent "Sodium Chloride" added', 
        time: '2 hours ago', 
        Icon: CheckCircleIcon, 
        color: 'success' 
      },
      { 
        id: 2, 
        type: 'batch_used', 
        message: 'Used 50g from batch #B-2024-001', 
        time: '3 hours ago', 
        Icon: FlaskIcon, 
        color: 'info' 
      },
      { 
        id: 3, 
        type: 'low_stock', 
        message: 'Ethanol running low (< 100mL)', 
        time: '5 hours ago', 
        Icon: AlertTriangleIcon, 
        color: 'warning' 
      },
      { 
        id: 4, 
        type: 'batch_expired', 
        message: 'Batch #B-2023-145 expired', 
        time: '1 day ago', 
        Icon: CalendarIcon, 
        color: 'danger' 
      }
    ];
    setRecentActivity(activities);
  };

  const setupQuickActions = () => {
    const actions = [];
    
    if (canCreateReagents()) {
      actions.push({
        id: 'add-reagent',
        label: 'Add Reagent',
        Icon: PlusIcon,
        color: 'primary',
        action: () => handleNavigate('reagents')
      });
    }
    
    actions.push({
      id: 'view-reports',
      label: 'View Reports',
      Icon: ChartBarIcon,
      color: 'secondary',
      action: () => handleNavigate('reports')
    });
    
    if (canManageEquipment()) {
      actions.push({
        id: 'manage-equipment',
        label: 'Equipment',
        Icon: CogsIcon,
        color: 'secondary',
        action: () => handleNavigate('equipment')
      });
    }
    
    if (isAdmin()) {
      actions.push({
        id: 'manage-users',
        label: 'Manage Users',
        Icon: UsersIcon,
        color: 'secondary',
        action: () => handleNavigate('users')
      });
    }
    
    setQuickActions(actions);
  };

  const handleNavigate = (page) => {
    if (onNavigate && typeof onNavigate === 'function') {
      onNavigate(page);
    } else {
      console.warn('onNavigate function not provided to Dashboard');
    }
  };

  const canCreateReagents = () => ['Admin', 'Researcher'].includes(user?.role);
  const canManageEquipment = () => ['Admin', 'Researcher'].includes(user?.role);
  const isAdmin = () => user?.role === 'Admin';

  const statsConfig = [
    {
      key: 'total_reagents',
      label: 'Total Reagents',
      Icon: FlaskIcon,
      color: 'primary',
      onClick: () => handleNavigate('reagents'),
      trend: { direction: 'up', value: 12, description: 'vs last month' }
    },
    {
      key: 'total_batches',
      label: 'Total Batches',
      Icon: DatabaseIcon,
      color: 'success',
      onClick: () => handleNavigate('reagents'),
      trend: { direction: 'up', value: 8 }
    },
    {
      key: 'low_stock',
      label: 'Low Stock Items',
      Icon: AlertTriangleIcon,
      color: 'warning',
      onClick: () => handleNavigate('reports'),
      trend: stats.low_stock > 0 ? { direction: 'up', value: 5 } : null
    },
    {
      key: 'expiring_soon',
      label: 'Expiring Soon',
      Icon: ClockIcon,
      color: 'danger',
      onClick: () => handleNavigate('reports'),
      trend: stats.expiring_soon > 0 ? { direction: 'down', value: 3 } : null
    }
  ];

  // Color configs for activity items
  const activityColors = {
    success: { bg: 'rgba(56, 161, 105, 0.1)', color: '#38a169' },
    info: { bg: 'rgba(49, 130, 206, 0.1)', color: '#3182ce' },
    warning: { bg: 'rgba(237, 137, 54, 0.1)', color: '#ed8936' },
    danger: { bg: 'rgba(229, 62, 62, 0.1)', color: '#e53e3e' }
  };

  return (
    <div className="dashboard">
      {loading && <LoadingOverlay />}
      
      {/* Page Header */}
      <div style={{
        display: 'flex',
        justifyContent: 'space-between',
        alignItems: 'center',
        marginBottom: '24px'
      }}>
        <div>
          <h1 style={{
            fontSize: '1.75rem',
            fontWeight: '800',
            color: '#1a365d',
            marginBottom: '4px'
          }}>
            Dashboard
          </h1>
          <p style={{ color: '#718096', fontSize: '0.875rem' }}>
            Welcome back, <span style={{ color: '#3182ce', fontWeight: '600' }}>{user?.username}</span>!
          </p>
        </div>
        <button
          onClick={loadDashboardData}
          style={{
            display: 'flex',
            alignItems: 'center',
            gap: '8px',
            padding: '10px 20px',
            background: 'linear-gradient(135deg, #3182ce 0%, #38b2ac 100%)',
            color: 'white',
            border: 'none',
            borderRadius: '10px',
            fontWeight: '600',
            fontSize: '0.875rem',
            cursor: 'pointer',
            boxShadow: '0 4px 15px rgba(49, 130, 206, 0.3)',
            transition: 'all 0.2s ease'
          }}
        >
          <RefreshIcon size={18} color="white" />
          Refresh
        </button>
      </div>

      {/* Stats Grid */}
      <div style={{
        display: 'grid',
        gridTemplateColumns: 'repeat(4, 1fr)',
        gap: '20px',
        marginBottom: '24px'
      }}>
        {statsConfig.map(stat => (
          <StatsCard
            key={stat.key}
            value={stats[stat.key]}
            title={stat.label}
            icon={<stat.Icon size={24} />}
            variant={stat.color}
            onClick={stat.onClick}
            trend={stat.trend?.direction}
            trendValue={stat.trend ? `${stat.trend.value}%` : null}
          />
        ))}
      </div>

      {/* Main Grid */}
      <div style={{
        display: 'grid',
        gridTemplateColumns: '1fr 350px',
        gap: '24px'
      }}>
        {/* Left Column - Charts placeholder */}
        <div style={{
          display: 'flex',
          flexDirection: 'column',
          gap: '24px'
        }}>
          {/* Chart Placeholder 1 */}
          <div style={{
            background: 'white',
            borderRadius: '16px',
            padding: '24px',
            border: '1px solid #e2e8f0',
            boxShadow: '0 1px 3px rgba(26, 54, 93, 0.08)'
          }}>
            <div style={{
              display: 'flex',
              alignItems: 'center',
              gap: '10px',
              marginBottom: '20px'
            }}>
              <TrendUpIcon size={20} color="#3182ce" />
              <h3 style={{ 
                fontSize: '1rem', 
                fontWeight: '700', 
                color: '#1a365d',
                margin: 0 
              }}>
                Inventory Trends
              </h3>
            </div>
            <div style={{
              height: '200px',
              background: 'linear-gradient(180deg, rgba(49, 130, 206, 0.05) 0%, rgba(56, 161, 105, 0.05) 100%)',
              borderRadius: '12px',
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
              color: '#718096'
            }}>
              <div style={{ textAlign: 'center' }}>
                <ChartBarIcon size={40} color="#a0aec0" />
                <p style={{ marginTop: '10px', fontSize: '0.875rem' }}>Chart coming soon</p>
              </div>
            </div>
          </div>
        </div>

        {/* Right Column - Quick Actions & Activity */}
        <div style={{
          display: 'flex',
          flexDirection: 'column',
          gap: '24px'
        }}>
          {/* Quick Actions */}
          <div style={{
            background: 'white',
            borderRadius: '16px',
            padding: '20px',
            border: '1px solid #e2e8f0',
            boxShadow: '0 1px 3px rgba(26, 54, 93, 0.08)'
          }}>
            <div style={{
              display: 'flex',
              alignItems: 'center',
              gap: '10px',
              marginBottom: '16px'
            }}>
              <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="#38b2ac" strokeWidth="2">
                <path d="M13 2L3 14h9l-1 8 10-12h-9l1-8z" />
              </svg>
              <h3 style={{ 
                fontSize: '1rem', 
                fontWeight: '700', 
                color: '#1a365d',
                margin: 0 
              }}>
                Quick Actions
              </h3>
            </div>
            <div style={{
              display: 'flex',
              flexWrap: 'wrap',
              gap: '8px'
            }}>
              {quickActions.map(action => {
                const ActionIcon = action.Icon;
                return (
                  <button
                    key={action.id}
                    onClick={action.action}
                    style={{
                      display: 'flex',
                      alignItems: 'center',
                      gap: '6px',
                      padding: '8px 14px',
                      background: action.color === 'primary' 
                        ? 'linear-gradient(135deg, #3182ce 0%, #38b2ac 100%)'
                        : 'white',
                      color: action.color === 'primary' ? 'white' : '#1a365d',
                      border: action.color === 'primary' ? 'none' : '1px solid #e2e8f0',
                      borderRadius: '8px',
                      fontWeight: '600',
                      fontSize: '0.8rem',
                      cursor: 'pointer',
                      transition: 'all 0.2s ease'
                    }}
                  >
                    <ActionIcon size={16} color={action.color === 'primary' ? 'white' : '#3182ce'} />
                    {action.label}
                  </button>
                );
              })}
            </div>
          </div>

          {/* Recent Activity */}
          <div style={{
            background: 'white',
            borderRadius: '16px',
            padding: '20px',
            border: '1px solid #e2e8f0',
            boxShadow: '0 1px 3px rgba(26, 54, 93, 0.08)',
            flex: 1
          }}>
            <div style={{
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'space-between',
              marginBottom: '16px'
            }}>
              <div style={{ display: 'flex', alignItems: 'center', gap: '10px' }}>
                <RefreshIcon size={20} color="#38a169" />
                <h3 style={{ 
                  fontSize: '1rem', 
                  fontWeight: '700', 
                  color: '#1a365d',
                  margin: 0 
                }}>
                  Recent Activity
                </h3>
              </div>
              <button style={{
                background: 'none',
                border: 'none',
                color: '#3182ce',
                fontSize: '0.8rem',
                fontWeight: '600',
                cursor: 'pointer'
              }}>
                View All
              </button>
            </div>
            
            <div style={{ display: 'flex', flexDirection: 'column', gap: '12px' }}>
              {recentActivity.map(activity => {
                const ActivityIcon = activity.Icon;
                const colors = activityColors[activity.color];
                return (
                  <div
                    key={activity.id}
                    style={{
                      display: 'flex',
                      alignItems: 'flex-start',
                      gap: '12px',
                      padding: '12px',
                      background: '#f8fafc',
                      borderRadius: '10px',
                      transition: 'all 0.2s ease'
                    }}
                  >
                    <div style={{
                      width: '36px',
                      height: '36px',
                      borderRadius: '10px',
                      background: colors.bg,
                      display: 'flex',
                      alignItems: 'center',
                      justifyContent: 'center',
                      flexShrink: 0
                    }}>
                      <ActivityIcon size={18} color={colors.color} />
                    </div>
                    <div style={{ flex: 1, minWidth: 0 }}>
                      <p style={{
                        fontSize: '0.85rem',
                        color: '#1a365d',
                        margin: '0 0 4px 0',
                        fontWeight: '500',
                        lineHeight: '1.4'
                      }}>
                        {activity.message}
                      </p>
                      <span style={{
                        fontSize: '0.75rem',
                        color: '#a0aec0'
                      }}>
                        {activity.time}
                      </span>
                    </div>
                  </div>
                );
              })}
            </div>
          </div>
        </div>
      </div>

      {/* Responsive Styles */}
      <style>{`
        @media (max-width: 1200px) {
          .dashboard > div:nth-child(3) {
            grid-template-columns: repeat(2, 1fr) !important;
          }
          .dashboard > div:nth-child(4) {
            grid-template-columns: 1fr !important;
          }
        }
        @media (max-width: 768px) {
          .dashboard > div:nth-child(3) {
            grid-template-columns: 1fr !important;
          }
        }
      `}</style>
    </div>
  );
};

export default Dashboard;
