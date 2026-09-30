import { Button } from '@/components/ui/button'
import type { DialogFrame } from '@/features/dialog/dialog-overlay'
import { DialogSurface } from '@/features/dialog/dialog-surface'
import type { TimerMode } from '@/lib/tauri'

type Props = {
  mode: TimerMode
  frame: DialogFrame
  onClose: () => void
  onDiscard: () => void
  onSave: () => void
}

export function RunningIntervalDialog({
  mode,
  frame,
  onClose,
  onDiscard,
  onSave,
}: Props) {
  const title =
    mode === 'stopwatch'
      ? 'Stopwatch is running'
      : mode === 'pomodoro'
        ? 'Session is running'
        : 'Timer is running'

  return (
    <DialogSurface
      frame={frame}
      labelledBy="running-interval-title"
      describedBy="running-interval-desc"
      onDismiss={onClose}
    >
      <h2
        id="running-interval-title"
        className="text-[15px] font-semibold text-neutral-900 dark:text-neutral-50"
      >
        {title}
      </h2>
      <p
        id="running-interval-desc"
        className="mt-1 text-[13px] text-neutral-500 dark:text-neutral-400"
      >
        Save this Entry, or Discard it?
      </p>
      <div className="mt-4 flex items-center justify-between gap-2">
        <Button type="button" variant="secondary" autoFocus onClick={onClose}>
          Close
        </Button>
        <div className="flex items-center gap-2">
          <Button type="button" variant="destructive" onClick={onDiscard}>
            Discard
          </Button>
          <Button type="button" onClick={onSave}>
            Save
          </Button>
        </div>
      </div>
    </DialogSurface>
  )
}
