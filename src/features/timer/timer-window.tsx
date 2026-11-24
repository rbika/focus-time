import { useEffect } from 'react'

import { WindowTitleBar } from '@/components/window-title-bar'
import { Dashboard } from '@/features/dashboard/dashboard'
import { TimerView } from '@/features/timer/timer-view'
import { api } from '@/lib/tauri'
import { useTimerStore } from '@/store/timer-store'

export function TimerWindow() {
  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (!event.metaKey || event.ctrlKey || event.altKey || event.shiftKey) {
        return
      }
      if (event.key !== '1') return
      event.preventDefault()
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
    }
    window.addEventListener('keydown', onKeyDown)
    return () => window.removeEventListener('keydown', onKeyDown)
  }, [])

  return (
    <div className="flex h-full flex-col overflow-hidden">
      <WindowTitleBar
        title=""
        onOpenEntries={() => void api.openEntries()}
        onOpenSettings={() => void api.openSettings()}
      />
      <div className="flex min-h-0 flex-1">
        <div className="min-w-0 flex-1">
          <TimerView />
        </div>
        <div
          className="mr-6 mb-6 w-px shrink-0 bg-neutral-200 dark:bg-neutral-800"
          aria-hidden
        />
        <div className="w-[160px] shrink-0">
          <Dashboard />
        </div>
      </div>
    </div>
  )
}
