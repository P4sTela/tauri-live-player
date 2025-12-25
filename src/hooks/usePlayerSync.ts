import { useEffect, useRef } from 'react';
import { usePlayerStore } from '../stores/playerStore';

/**
 * プレイヤー状態を定期的に同期するフック
 */
export function usePlayerSync(intervalMs: number = 100) {
  const { syncState, status } = usePlayerStore();
  const intervalRef = useRef<number | null>(null);

  useEffect(() => {
    // 再生中のみ高頻度で同期
    if (status === 'playing') {
      intervalRef.current = window.setInterval(() => {
        syncState();
      }, intervalMs);
    } else {
      // 再生中でなければ低頻度
      intervalRef.current = window.setInterval(() => {
        syncState();
      }, 500);
    }

    return () => {
      if (intervalRef.current) {
        clearInterval(intervalRef.current);
      }
    };
  }, [syncState, status, intervalMs]);
}
