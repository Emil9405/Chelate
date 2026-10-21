// components/Dashboard.js - CHELATE Style with Real Activity & Charts
import React, { useState, useEffect, useMemo } from 'react';
import { api } from '../services/api';
import StatsCard from './StatsCard';
import LoadingOverlay from './LoadingOverlay';
import useIsMobile from './hooks/useIsMobile';
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

// ==================== MINI BAR CHART (pure SVG, no deps) ====================

const MiniBarChart = ({ data, dataKey, labelKey, color = '#3182ce', height = 200 }) => {
  if (!data || data.length === 0) {
    return (
      <div style={{
        height,
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        color: '#a0aec0',
        fontSize: '0.875rem'
      }}>
        No data available yet
      </div>
    );
  }

  const maxValue = Math.max(...data.map(d => d[dataKey]), 1);
  const barWidth = Math.max(12, Math.min(40, (600 / data.length) - 8));
  const chartWidth = data.length * (barWidth + 6) + 40;
  const chartHeight = height - 30;

  return (
    <div style={{ overflowX: 'auto', overflowY: 'hidden' }}>
      <svg
        width={Math.max(chartWidth, 300)}
        height={height}
        viewBox={`0 0 ${Math.max(chartWidth, 300)} ${height}`}
        style={{ display: 'block' }}
      >
        {/* Grid lines */}
        {[0.25, 0.5, 0.75, 1].map((frac, i) => {
          const y = chartHeight - (chartHeight * frac) + 5;
          return (
            <g key={i}>
              <line
                x1="35" y1={y}
                x2={chartWidth - 5} y2={y}
                stroke="#e2e8f0"
                strokeWidth="1"
                strokeDasharray="4,4"
              />
              <text
                x="30" y={y + 4}
                textAnchor="end"
                fill="#a0aec0"
                fontSize="10"
              >
                {Math.round(maxValue * frac)}
              </text>
            </g>
          );
        })}

        {/* Bars */}
        {data.map((item, i) => {
          const barHeight = (item[dataKey] / maxValue) * (chartHeight - 10);
          const x = 40 + i * (barWidth + 6);
          const y = chartHeight - barHeight + 5;

          return (
            <g key={i}>
              <rect
                x={x}
                y={y}
                width={barWidth}
                height={Math.max(barHeight, 2)}
                rx="4"
                fill={color}
                opacity="0.85"
              >
                <title>{`${item[labelKey]}: ${item[dataKey]}`}</title>
              </rect>
              <text
                x={x + barWidth / 2}
                y={height - 4}
                textAnchor="middle"
                fill="#a0aec0"
                fontSize="9"
                transform={data.length > 14 ? `rotate(-45, ${x + barWidth / 2}, ${height - 4})` : ''}
              >
                {item[labelKey]?.slice(-5) || ''}
              </text>
            </g>
          );
        })}
      </svg>
    </div>
  );
};

// ==================== HORIZONTAL BAR CHART ====================

const HorizontalBarChart = ({ data, valueKey, labelKey, color = '#ed8936' }) => {
  if (!data || data.length === 0) {
    return (
      <div style={{
        padding: '20px',
        textAlign: 'center',
        color: '#a0aec0',
        fontSize: '0.875rem'
      }}>
        No upcoming expirations
      </div>
    );
  }

  const maxValue = Math.max(...data.map(d => d[valueKey]), 1);

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: '12px' }}>
      {data.map((item, i) => (
        <div key={i}>
          <div style={{
            display: 'flex',
            justifyContent: 'space-between',
            marginBottom: '4px',
            fontSize: '0.8rem'
          }}>
            <span style={{ color: '#4a5568', fontWeight: '500' }}>{item[labelKey]}</span>
            <span style={{ color: '#1a365d', fontWeight: '700' }}>{item[valueKey]}</span>
          </div>
          <div style={{
            height: '8px',
            background: '#edf2f7',
            borderRadius: '4px',
            overflow: 'hidden'
          }}>
            <div style={{
              height: '100%',
              width: `${(item[valueKey] / maxValue) * 100}%`,
              background: i === 0
                ? 'linear-gradient(90deg, #e53e3e, #ed8936)'
                : `linear-gradient(90deg, ${color}, ${color}dd)`,
              borderRadius: '4px',
              transition: 'width 0.6s ease'
            }} />
          </div>
        </div>
      ))}
    </div>
  );
};

// ==================== TIME AGO HELPER ====================

const timeAgo = (dateString) => {
  if (!dateString) return '';
  try {
    const date = new Date(dateString);
    const now = new Date();
    const diffMs = now - date;
    const diffMin = Math.floor(diffMs / 60000);
    const diffHrs = Math.floor(diffMs / 3600000);
    const diffDays = Math.floor(diffMs / 86400000);

    if (diffMin < 1) return 'just now';
    if (diffMin < 60) return `${diffMin}m ago`;
    if (diffHrs < 24) return `${diffHrs}h ago`;
    if (diffDays < 7) return `${diffDays}d ago`;
    return date.toLocaleDateString();
  } catch {
    return dateString;
  }
};

// ==================== ACTIVITY ICON/COLOR MAPPING ====================

const getActivityMeta = (action, entityType) => {
  const map = {
    create: { Icon: PlusIcon, bg: 'rgba(56, 161, 105, 0.1)', fg: '#38a169' },
    update: { Icon: RefreshIcon, bg: 'rgba(49, 130, 206, 0.1)', fg: '#3182ce' },
    delete: { Icon: AlertTriangleIcon, bg: 'rgba(229, 62, 62, 0.1)', fg: '#e53e3e' },
    use_reagent: { Icon: FlaskIcon, bg: 'rgba(56, 178, 172, 0.1)', fg: '#38b2ac' },
    login: { Icon: CheckCircleIcon, bg: 'rgba(56, 161, 105, 0.1)', fg: '#38a169' },
    logout: { Icon: ClockIcon, bg: 'rgba(237, 137, 54, 0.1)', fg: '#ed8936' },
    jwt_rotation: { Icon: CogsIcon, bg: 'rgba(237, 137, 54, 0.1)', fg: '#ed8936' },
  };

  if (map[action]) return map[action];

  for (const [key, val] of Object.entries(map)) {
    if (action?.toLowerCase().includes(key)) return val;
  }

  const entityMap = {
    reagent: map.create,
    batch: map.use_reagent,
    equipment: { Icon: CogsIcon, bg: 'rgba(49, 130, 206, 0.1)', fg: '#3182ce' },
    experiment: { Icon: FlaskIcon, bg: 'rgba(56, 178, 172, 0.1)', fg: '#38b2ac' },
    user: { Icon: CheckCircleIcon, bg: 'rgba(56, 161, 105, 0.1)', fg: '#38a169' },
  };

  return entityMap[entityType] || map.update;
};

// ==================== BEAKER ICON (for experiments card) ====================

const BeakerIcon = ({ size = 24, color = 'currentColor' }) => (
  <svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke={color} strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
    <path d="M4.5 3h15M6 3v16a2 2 0 0 0 2 2h8a2 2 0 0 0 2-2V3" />
    <path d="M6 14h12" />
  </svg>
);

// ==================== WRENCH ICON (for equipment alerts card) ====================

const WrenchIcon = ({ size = 24, color = 'currentColor' }) => (
  <svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke={color} strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
    <path d="M14.7 6.3a1 1 0 0 0 0 1.4l1.6 1.6a1 1 0 0 0 1.4 0l3.77-3.77a6 6 0 0 1-7.94 7.94l-6.91 6.91a2.12 2.12 0 0 1-3-3l6.91-6.91a6 6 0 0 1 7.94-7.94l-3.76 3.76z" />
  </svg>
);

// ==================== MAIN DASHBOARD COMPONENT ====================

const Dashboard = ({ user, showToast, onNavigate }) => {
  const isMobile = useIsMobile();
  // <=1200px: 2 stat cards per row, side column moves under the charts
  const isNarrow = useIsMobile('(max-width: 1200px)');
  const [stats, setStats] = useState(null);
  const [loading, setLoading] = useState(true);
  const [recentActivity, setRecentActivity] = useState([]);
  const [trends, setTrends] = useState({ usage_by_day: [], expiring_by_week: [] });

  useEffect(() => {
    loadDashboardData();
  }, [user]);

  const loadDashboardData = async () => {
    try {
      setLoading(true);

      const [statsData, reagentsData, batchesData, lowStockData, expiringData, activityData, trendsData] = await Promise.all([
        api.getDashboardStats().catch(() => null),
        api.getReagents({ page: 1, per_page: 1 }).catch(() => null),
        api.getAllBatches({ page: 1, per_page: 1 }).catch(() => null),
        api.getLowStockItems(10).catch(() => []),
        api.getExpiringItems(30).catch(() => []),
        api.getRecentActivity().catch(() => null),
        api.getDashboardTrends().catch(() => null),
      ]);

      // Equipment & experiments: use stats if backend is updated, otherwise fetch separately
      let equipmentAlerts = statsData?.equipment_alerts;
      let totalEquipment = statsData?.total_equipment;
      let activeExperiments = statsData?.active_experiments;

      // Fallback: if backend doesn't return new fields, fetch separately
      if (equipmentAlerts == null || totalEquipment == null) {
        try {
          const eqData = await api.getEquipment({ page: 1, per_page: 1000 });
          const items = eqData?.data || (Array.isArray(eqData) ? eqData : []);
          totalEquipment = totalEquipment ?? items.filter(e => e.status !== 'retired').length;
          equipmentAlerts = equipmentAlerts ?? items.filter(e =>
            ['maintenance', 'damaged', 'calibration'].includes(e.status)
          ).length;
        } catch {
          totalEquipment = totalEquipment ?? 0;
          equipmentAlerts = equipmentAlerts ?? 0;
        }
      }

      if (activeExperiments == null) {
        try {
          const expStats = await api.getExperimentsStats();
          activeExperiments = (expStats?.in_progress ?? 0) + (expStats?.planned ?? expStats?.scheduled ?? 0);
        } catch {
          activeExperiments = 0;
        }
      }

      // Normalize array data from separate endpoints (more reliable queries)
      const expiringArr = Array.isArray(expiringData) ? expiringData : (expiringData?.data || []);
      const lowStockArr = Array.isArray(lowStockData) ? lowStockData : (lowStockData?.data || []);

      setStats({
        total_reagents: statsData?.total_reagents ?? reagentsData?.total ?? 0,
        total_batches: statsData?.total_batches ?? batchesData?.total ?? 0,
        low_stock: Math.max(statsData?.low_stock ?? 0, lowStockArr.length),
        expiring_soon: Math.max(statsData?.expiring_soon ?? 0, expiringArr.length),
        total_equipment: totalEquipment ?? 0,
        equipment_alerts: equipmentAlerts ?? 0,
        active_experiments: activeExperiments ?? 0,
      });

      // Real activity from audit logs
      if (activityData) {
        const items = Array.isArray(activityData) ? activityData : (activityData?.data || []);
        setRecentActivity(items.slice(0, 10));
      }

      // Trends for charts
      if (trendsData) {
        const tData = trendsData?.data || trendsData;
        setTrends({
          usage_by_day: tData?.usage_by_day || [],
          expiring_by_week: tData?.expiring_by_week || [],
        });
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

  const handleNavigate = (page) => {
    if (onNavigate && typeof onNavigate === 'function') {
      onNavigate(page);
    }
  };

  const canCreateReagents = () => ['Admin', 'Researcher'].includes(user?.role);
  const canManageEquipment = () => ['Admin', 'Researcher'].includes(user?.role);
  const isAdmin = () => user?.role === 'Admin';

  const quickActions = useMemo(() => {
    const actions = [];
    if (canCreateReagents()) {
      actions.push({ id: 'add-reagent', label: 'Add Reagent', Icon: PlusIcon, color: 'primary', action: () => handleNavigate('reagents') });
    }
    actions.push({ id: 'view-reports', label: 'View Reports', Icon: ChartBarIcon, color: 'secondary', action: () => handleNavigate('reports') });
    if (canManageEquipment()) {
      actions.push({ id: 'manage-equipment', label: 'Equipment', Icon: CogsIcon, color: 'secondary', action: () => handleNavigate('equipment') });
    }
    if (isAdmin()) {
      actions.push({ id: 'manage-users', label: 'Manage Users', Icon: UsersIcon, color: 'secondary', action: () => handleNavigate('users') });
    }
    return actions;
  }, [user]);

  // 4 actionable stat cards
  const statsConfig = [
    {
      key: 'low_stock',
      label: 'Low Stock',
      Icon: AlertTriangleIcon,
      color: 'warning',
      onClick: () => handleNavigate('reports'),
      subtitle: stats ? (stats.low_stock > 0 ? 'Needs reordering' : 'All stocked') : null,
    },
    {
      key: 'expiring_soon',
      label: 'Expiring Soon',
      Icon: ClockIcon,
      color: 'danger',
      onClick: () => handleNavigate('reports'),
      subtitle: stats ? (stats.expiring_soon > 0 ? 'Within 30 days' : 'None upcoming') : null,
    },
    {
      key: 'equipment_alerts',
      label: 'Equipment Alerts',
      Icon: WrenchIcon,
      color: 'info',
      onClick: () => handleNavigate('equipment'),
      subtitle: stats ? (stats.equipment_alerts > 0 ? 'Maintenance / damaged' : 'All operational') : null,
    },
    {
      key: 'active_experiments',
      label: 'Active Experiments',
      Icon: BeakerIcon,
      color: 'teal',
      onClick: () => handleNavigate('experiments'),
      subtitle: stats ? (stats.active_experiments > 0 ? 'In progress / scheduled' : 'None active') : null,
    },
  ];

  // Greeting based on time of day
  const getGreeting = () => {
    const hour = new Date().getHours();
    if (hour < 12) return 'Good morning';
    if (hour < 18) return 'Good afternoon';
    return 'Good evening';
  };

  // Summary line items
  const summaryItems = stats ? [
    { value: stats.total_reagents, label: 'reagents' },
    { value: stats.total_batches, label: 'batches' },
    { value: stats.total_equipment, label: 'equipment' },
  ] : [];

  return (
    // Same page frame as the other sections: clear the fixed header, side padding
    <div className="dashboard" style={isMobile ? { padding: '12px', paddingTop: '68px' } : { padding: '20px', paddingTop: '90px' }}>
      {loading && <LoadingOverlay />}

      {/* Page Header */}
      <div style={{
        display: 'flex',
        justifyContent: 'space-between',
        alignItems: 'center',
        marginBottom: '8px',
        gap: '12px',
        flexWrap: 'wrap'
      }}>
        <div style={{ minWidth: 0 }}>
          <h1 style={{
            fontSize: isMobile ? '1.35rem' : '1.75rem',
            fontWeight: '800',
            color: '#1a365d',
            marginBottom: '4px'
          }}>
            {getGreeting()}, <span style={{ color: '#3182ce' }}>{user?.username}</span>
          </h1>
          <p style={{ color: '#718096', fontSize: '0.875rem', margin: 0 }}>
            {new Date().toLocaleDateString('en-US', { weekday: 'long', year: 'numeric', month: 'long', day: 'numeric' })}
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

      {/* Summary Line — totals as context, not cards */}
      <div style={{
        display: 'flex',
        alignItems: 'center',
        gap: '6px',
        marginBottom: '20px',
        fontSize: '0.85rem',
        color: '#718096',
        flexWrap: 'wrap',
      }}>
        {summaryItems.map((item, i) => (
          <React.Fragment key={item.label}>
            {i > 0 && <span style={{ color: '#cbd5e0', margin: '0 2px' }}>·</span>}
            <span>
              <span style={{ fontWeight: '700', color: '#1a365d' }}>{item.value}</span>{' '}
              {item.label}
            </span>
          </React.Fragment>
        ))}
      </div>

      {/* Stats Grid — 4 actionable cards */}
      <div style={{
        display: 'grid',
        gridTemplateColumns: isNarrow ? 'repeat(2, minmax(0, 1fr))' : 'repeat(4, 1fr)',
        gap: isMobile ? '10px' : '20px',
        marginBottom: isMobile ? '16px' : '24px'
      }}>
        {statsConfig.map(stat => (
          <StatsCard
            key={stat.key}
            value={stats ? stats[stat.key] : '—'}
            title={stat.label}
            icon={<stat.Icon size={24} />}
            variant={stat.color}
            onClick={stat.onClick}
            subtitle={stat.subtitle}
          />
        ))}
      </div>

      {/* Main Grid */}
      <div style={{
        display: 'grid',
        gridTemplateColumns: isNarrow ? 'minmax(0, 1fr)' : '1fr 350px',
        gap: isMobile ? '16px' : '24px'
      }}>
        {/* Left Column — Charts */}
        <div style={{ display: 'flex', flexDirection: 'column', gap: '24px' }}>

          {/* Usage Trends Chart */}
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
              justifyContent: 'space-between',
              marginBottom: '20px'
            }}>
              <div style={{ display: 'flex', alignItems: 'center', gap: '10px' }}>
                <TrendUpIcon size={20} color="#3182ce" />
                <h3 style={{ fontSize: '1rem', fontWeight: '700', color: '#1a365d', margin: 0 }}>
                  Reagent Usage (Last 30 Days)
                </h3>
              </div>
              <span style={{
                fontSize: '0.75rem',
                color: '#a0aec0',
                fontWeight: '500'
              }}>
                {trends.usage_by_day.length > 0
                  ? `${trends.usage_by_day.reduce((s, d) => s + d.usage_count, 0)} total uses`
                  : ''}
              </span>
            </div>
            <MiniBarChart
              data={trends.usage_by_day}
              dataKey="usage_count"
              labelKey="date"
              color="#3182ce"
              height={200}
            />
          </div>

          {/* Expiration Timeline */}
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
              <CalendarIcon size={20} color="#ed8936" />
              <h3 style={{ fontSize: '1rem', fontWeight: '700', color: '#1a365d', margin: 0 }}>
                Upcoming Expirations (4 Weeks)
              </h3>
            </div>
            <HorizontalBarChart
              data={trends.expiring_by_week}
              valueKey="count"
              labelKey="week_label"
              color="#ed8936"
            />
          </div>
        </div>

        {/* Right Column — Quick Actions & Activity */}
        <div style={{ display: 'flex', flexDirection: 'column', gap: '24px' }}>

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
              <h3 style={{ fontSize: '1rem', fontWeight: '700', color: '#1a365d', margin: 0 }}>
                Quick Actions
              </h3>
            </div>
            <div style={{ display: 'flex', flexWrap: 'wrap', gap: '8px' }}>
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

          {/* Recent Activity — REAL DATA from audit_logs */}
          <div style={{
            background: 'white',
            borderRadius: '16px',
            padding: '20px',
            border: '1px solid #e2e8f0',
            boxShadow: '0 1px 3px rgba(26, 54, 93, 0.08)',
            flex: 1,
            minHeight: 0,
            display: 'flex',
            flexDirection: 'column'
          }}>
            <div style={{
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'space-between',
              marginBottom: '16px'
            }}>
              <div style={{ display: 'flex', alignItems: 'center', gap: '10px' }}>
                <RefreshIcon size={20} color="#38a169" />
                <h3 style={{ fontSize: '1rem', fontWeight: '700', color: '#1a365d', margin: 0 }}>
                  Recent Activity
                </h3>
              </div>
              <button
                onClick={() => handleNavigate('reports')}
                style={{
                  background: 'none',
                  border: 'none',
                  color: '#3182ce',
                  fontSize: '0.8rem',
                  fontWeight: '600',
                  cursor: 'pointer'
                }}
              >
                View All
              </button>
            </div>

            <div style={{
              display: 'flex',
              flexDirection: 'column',
              gap: '8px',
              overflowY: 'auto',
              flex: 1,
              maxHeight: '400px'
            }}>
              {recentActivity.length === 0 && !loading && (
                <div style={{
                  textAlign: 'center',
                  padding: '24px',
                  color: '#a0aec0',
                  fontSize: '0.85rem'
                }}>
                  No recent activity
                </div>
              )}

              {recentActivity.map((activity) => {
                const meta = getActivityMeta(activity.action, activity.entity_type);
                const ActivityIcon = meta.Icon;
                const desc = activity.description || `${activity.action} ${activity.entity_type}`;
                const username = activity.username;

                return (
                  <div
                    key={activity.id}
                    style={{
                      display: 'flex',
                      alignItems: 'flex-start',
                      gap: '12px',
                      padding: '10px 12px',
                      background: '#f8fafc',
                      borderRadius: '10px',
                      transition: 'all 0.2s ease'
                    }}
                  >
                    <div style={{
                      width: '34px',
                      height: '34px',
                      borderRadius: '10px',
                      background: meta.bg,
                      display: 'flex',
                      alignItems: 'center',
                      justifyContent: 'center',
                      flexShrink: 0
                    }}>
                      <ActivityIcon size={16} color={meta.fg} />
                    </div>
                    <div style={{ flex: 1, minWidth: 0 }}>
                      <p style={{
                        fontSize: '0.8rem',
                        color: '#1a365d',
                        margin: '0 0 2px 0',
                        fontWeight: '500',
                        lineHeight: '1.4',
                        overflow: 'hidden',
                        textOverflow: 'ellipsis',
                        whiteSpace: 'nowrap'
                      }}>
                        {desc}
                      </p>
                      <div style={{
                        display: 'flex',
                        alignItems: 'center',
                        gap: '8px'
                      }}>
                        {username && (
                          <span style={{
                            fontSize: '0.7rem',
                            color: '#3182ce',
                            fontWeight: '600'
                          }}>
                            {username}
                          </span>
                        )}
                        <span style={{ fontSize: '0.7rem', color: '#a0aec0' }}>
                          {timeAgo(activity.created_at)}
                        </span>
                      </div>
                    </div>
                  </div>
                );
              })}
            </div>
          </div>
        </div>
      </div>

      {/* Responsive Styles */}
    </div>
  );
};

export default Dashboard;
