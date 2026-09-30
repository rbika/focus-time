import { useCallback, useEffect, useState } from 'react'

import { WindowTitleBar } from '@/components/window-title-bar'
import { crossfadeFrames, crossfadeStyle } from '@/features/crossfade/crossfade'
import { usePrefersReducedMotion } from '@/features/crossfade/use-prefers-reduced-motion'
import { StatsView } from '@/features/stats/stats-view'
import { TimerView } from '@/features/timer/timer-view'
import { api } from '@/lib/tauri'
import type { MainView } from '@/lib/tauri'
import { useTimerStore } from '@/store/timer-store'

export function MainWindow() {
  const reducedMotion = usePrefersReducedMotion()
  const [view, setView] = useState<MainView>('timer')
  const frames = crossfadeFrames('timer', 'stats', view, 0)
  const animate = !reducedMotion

  const switchTo = useCallback(
    (next: MainView) => {
      if (next === view) return
      setView(next)
      void api.resizeMainWindow(next)
    },
    [view],
  )

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (!event.metaKey || event.ctrlKey || event.altKey || event.shiftKey) {
        return
      }
      if (event.key === '1') {
        event.preventDefault()
        if (view !== 'timer') {
          switchTo('timer')
          return
        }
        const snapshot = useTimerStore.getState().snapshot
        if (
          snapshot == null ||
          snapshot.waiting ||
          (snapshot.status !== 'idle' && snapshot.status !== 'completed')
        ) {
          return
        }
        const nextMode =
          snapshot.mode === 'timer'
            ? 'stopwatch'
            : snapshot.mode === 'stopwatch'
              ? 'pomodoro'
              : 'timer'
        void useTimerStore.getState().actions.setMode(nextMode)
        return
      }
      if (event.key === '2') {
        event.preventDefault()
        switchTo('stats')
      }
    }
    window.addEventListener('keydown', onKeyDown)
    return () => window.removeEventListener('keydown', onKeyDown)
  }, [switchTo, view])

  return (
    <div className="flex h-full flex-col overflow-hidden">
      <WindowTitleBar
        title=""
        onOpenSettings={() => void api.openSettings()}
        navigation={
          view === 'timer'
            ? { direction: 'stats', onClick: () => switchTo('stats') }
            : { direction: 'back', onClick: () => switchTo('timer') }
        }
      />
      <div className="relative min-h-0 flex-1">
        <div
          className="absolute inset-0"
          style={crossfadeStyle(frames.timer, animate)}
          inert={!frames.timer.interactive ? true : undefined}
        >
          <TimerView active={view === 'timer'} />
        </div>
        <div
          className="absolute inset-0"
          style={crossfadeStyle(frames.stats, animate)}
          inert={!frames.stats.interactive ? true : undefined}
        >
          <StatsView active={view === 'stats'} />
        </div>
      </div>
    </div>
  )
}
