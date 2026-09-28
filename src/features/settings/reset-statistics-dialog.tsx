import { Button } from '@/components/ui/button'
import type { DialogFrame } from '@/features/dialog/dialog-overlay'
import { DialogSurface } from '@/features/dialog/dialog-surface'

type Props = {
  frame: DialogFrame
  onCancel: () => void
  onConfirm: () => void
}

export function ResetStatisticsDialog({ frame, onCancel, onConfirm }: Props) {
  return (
    <DialogSurface
      frame={frame}
      labelledBy="reset-statistics-title"
      describedBy="reset-statistics-desc"
      onDismiss={onCancel}
    >
      <h2
        id="reset-statistics-title"
        className="text-[15px] font-semibold text-neutral-900 dark:text-neutral-50"
      >
        Reset statistics?
      </h2>
      <p
        id="reset-statistics-desc"
        className="mt-1 text-[13px] text-neutral-500 dark:text-neutral-400"
      >
        This deletes every entry. It can’t be undone.
      </p>
      <div className="mt-4 flex items-center justify-between gap-2">
        <Button type="button" variant="secondary" onClick={onCancel}>
          Cancel
        </Button>
        <Button
          type="button"
          className="bg-red-600 text-white hover:bg-red-700 dark:bg-red-600 dark:text-white dark:hover:bg-red-500"
          onClick={onConfirm}
        >
          Reset
        </Button>
      </div>
    </DialogSurface>
  )
}
