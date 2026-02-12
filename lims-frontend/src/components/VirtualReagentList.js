// components/VirtualReagentList.js
// Виртуализированный список с infinite scroll
import React, { useRef, useCallback, useEffect } from 'react';
import Loading from './Loading';

const VirtualReagentList = ({
  items,
  renderItem,
  itemHeight = 70,
  containerHeight = 600,
  loadMore,
  hasMore,
  loadingMore,
  overscan = 5,  // Количество элементов за пределами viewport
}) => {
  const containerRef = useRef(null);
  const [scrollTop, setScrollTop] = React.useState(0);

  // Вычисляем видимые элементы
  const totalHeight = items.length * itemHeight;
  const startIndex = Math.max(0, Math.floor(scrollTop / itemHeight) - overscan);
  const endIndex = Math.min(
    items.length,
    Math.ceil((scrollTop + containerHeight) / itemHeight) + overscan
  );
  const visibleItems = items.slice(startIndex, endIndex);
  const offsetY = startIndex * itemHeight;

  // Обработка скролла
  const handleScroll = useCallback((e) => {
    const { scrollTop, scrollHeight, clientHeight } = e.target;
    setScrollTop(scrollTop);
    
    // Загружаем ещё когда до конца осталось 3 экрана
    const threshold = clientHeight * 3;
    if (scrollHeight - scrollTop - clientHeight < threshold) {
      if (hasMore && !loadingMore) {
        loadMore();
      }
    }
  }, [hasMore, loadingMore, loadMore]);

  // Intersection Observer для более надёжного триггера
  const sentinelRef = useRef(null);
  
  useEffect(() => {
    if (!sentinelRef.current) return;
    
    const observer = new IntersectionObserver(
      (entries) => {
        if (entries[0].isIntersecting && hasMore && !loadingMore) {
          loadMore();
        }
      },
      { root: containerRef.current, rootMargin: '200px' }
    );
    
    observer.observe(sentinelRef.current);
    return () => observer.disconnect();
  }, [hasMore, loadingMore, loadMore]);

  return (
    <div
      ref={containerRef}
      onScroll={handleScroll}
      style={{
        height: containerHeight,
        overflow: 'auto',
        position: 'relative',
      }}
    >
      {/* Spacer для общей высоты */}
      <div style={{ height: totalHeight, position: 'relative' }}>
        {/* Видимые элементы */}
        <div
          style={{
            position: 'absolute',
            top: offsetY,
            left: 0,
            right: 0,
          }}
        >
          {visibleItems.map((item, index) => (
            <div
              key={item.id}
              style={{ height: itemHeight }}
            >
              {renderItem(item, startIndex + index)}
            </div>
          ))}
        </div>
        
        {/* Sentinel для Intersection Observer */}
        {hasMore && (
          <div
            ref={sentinelRef}
            style={{
              position: 'absolute',
              bottom: 0,
              height: 1,
              width: '100%',
            }}
          />
        )}
      </div>
      
      {/* Loading indicator */}
      {loadingMore && (
        <div style={{
          position: 'sticky',
          bottom: 0,
          padding: '12px',
          textAlign: 'center',
          background: 'linear-gradient(transparent, white 30%)',
        }}>
          <Loading size="small" message="Loading more..." />
        </div>
      )}
    </div>
  );
};

export default VirtualReagentList;