// Updated components/Table.js with custom actions support (e.g., reset)
import React from 'react';
import Button from './Button';

const Table = ({ 
  data, 
  columns, 
  actions, 
  onAction, 
  emptyMessage, 
  loading,
  pagination = true,  
  currentPage = 1,
  totalItems = 0,
  perPage = 10,
  onPageChange  
}) => {
  const totalPages = Math.ceil(totalItems / perPage);
  const startIndex = (currentPage - 1) * perPage;
  const endIndex = startIndex + perPage;
  const paginatedData = data ? data.slice(startIndex, endIndex) : [];

  const handlePageChange = (page) => {
    if (onPageChange) onPageChange(page);
  };

  const actionButtons = {
    view: { label: 'View', variant: 'secondary' },
    edit: { label: 'Edit', variant: 'primary' },
    delete: { label: 'Delete', variant: 'danger' },
    reset: { label: 'Reset Password', variant: 'warning' }  
  };

  return (
    <div style={{ overflowX: 'auto', position: 'relative' }}>
      {loading && (
        <div style={{
          position: 'absolute',
          top: 0,
          left: 0,
          right: 0,
          bottom: 0,
          background: 'rgba(255, 255, 255, 0.8)',
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
          zIndex: 10
        }}>
          <div style={{
            width: '16px',
            height: '16px',
            border: '2px solid #e2e8f0',
            borderRadius: '50%',
            borderTopColor: '#667eea',
            animation: 'spin 1s linear infinite'
          }}></div>
        </div>
      )}
      <table style={{ width: '100%', borderCollapse: 'collapse' }}>
        <thead>
          <tr>
            {columns.map(col => (
              <th key={col.key} style={{
                textAlign: 'left',
                padding: '12px 16px',
                borderBottom: '1px solid #e2e8f0',
                backgroundColor: '#f8fafc',
                fontWeight: '600',
                color: '#4a5568',
                fontSize: '0.875rem',
                textTransform: 'uppercase',
                letterSpacing: '0.05em'
              }}>
                {col.label}
              </th>
            ))}
            {actions && (
              <th style={{
                textAlign: 'left',
                padding: '12px 16px',
                borderBottom: '1px solid #e2e8f0',
                backgroundColor: '#f8fafc',
                fontWeight: '600',
                color: '#4a5568',
                fontSize: '0.875rem',
                textTransform: 'uppercase',
                letterSpacing: '0.05em'
              }}>
                Actions
              </th>
            )}
          </tr>
        </thead>
        <tbody>
          {paginatedData.length > 0 ? (
            paginatedData.map(item => (
              <tr key={item.id} style={{ backgroundColor: 'white' }}>
                {columns.map(col => (
                  <td key={col.key} style={{
                    textAlign: 'left',
                    padding: '12px 16px',
                    borderBottom: '1px solid #e2e8f0'
                  }}>
                    {col.render ? col.render(item) : item[col.key]}
                  </td>
                ))}
                {actions && (
                  <td style={{
                    textAlign: 'left',
                    padding: '12px 16px',
                    borderBottom: '1px solid #e2e8f0',
                    whiteSpace: 'nowrap'
                  }}>
                    {Object.keys(actions).map((key) => actions[key] && (
                      <Button 
                        key={key}
                        variant={actionButtons[key]?.variant || 'secondary'} 
                        size="sm" 
                        onClick={() => onAction(key, item)}
                      >
                        {actionButtons[key]?.label || key.charAt(0).toUpperCase() + key.slice(1)}
                      </Button>
                    ))}
                  </td>
                )}
              </tr>
            ))
          ) : (
            <tr>
              <td colSpan={columns.length + (actions ? 1 : 0)} style={{
                textAlign: 'center',
                padding: '3rem 2rem',
                color: '#718096'
              }}>
                {emptyMessage || 'No data found'}
              </td>
            </tr>
          )}
        </tbody>
      </table>

      {pagination && totalPages > 1 && (
        <div style={{
          display: 'flex',
          justifyContent: 'space-between',
          alignItems: 'center',
          padding: '1rem 0',
          borderTop: '1px solid #e2e8f0'
        }}>
          <div>
            <Button 
              variant="secondary" 
              size="sm" 
              disabled={currentPage === 1}
              onClick={() => handlePageChange(currentPage - 1)}
            >
              Previous
            </Button>
          </div>
          <div style={{ color: '#718096' }}>
            Page {currentPage} of {totalPages} (total items: {totalItems})
          </div>
          <div>
            <Button 
              variant="secondary" 
              size="sm" 
              disabled={currentPage === totalPages}
              onClick={() => handlePageChange(currentPage + 1)}
            >
              Next
            </Button>
          </div>
        </div>
      )}
    </div>
  );
};

export default Table;