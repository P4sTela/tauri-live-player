import { useEffect } from 'react';
import { usePlayerStore } from '../stores/playerStore';
import { useProjectStore } from '../stores/projectStore';

export function useKeyboard() {
  const { play, pause, stop, seek, next, prev, status, currentTime } = usePlayerStore();
  const { saveProject } = useProjectStore();

  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      // 入力フィールドでは無効
      if (e.target instanceof HTMLInputElement || e.target instanceof HTMLTextAreaElement) {
        return;
      }

      switch (e.code) {
        case 'Space':
          e.preventDefault();
          if (status === 'playing') {
            pause();
          } else {
            play();
          }
          break;

        case 'Escape':
          stop();
          break;

        case 'ArrowLeft':
          seek(Math.max(0, currentTime - 5));
          break;

        case 'ArrowRight':
          seek(currentTime + 5);
          break;

        case 'ArrowUp':
          e.preventDefault();
          prev();
          break;

        case 'ArrowDown':
          e.preventDefault();
          next();
          break;

        case 'PageUp':
          prev();
          break;

        case 'PageDown':
          next();
          break;

        case 'Home':
          seek(0);
          break;

        case 'KeyS':
          if (e.metaKey || e.ctrlKey) {
            e.preventDefault();
            saveProject();
          }
          break;
      }
    };

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [play, pause, stop, seek, next, prev, status, currentTime, saveProject]);
}
