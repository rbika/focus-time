import { BoltIcon, List } from 'lucide-react'

import { Button } from '@/components/ui/button'

export function WindowTitleBar({
  title,
  onOpenEntries,
  onOpenSettings,
}: {
  title: string
  onOpenEntries?: () => void
  onOpenSettings?: () => void
}) {
  return (
    <div
      data-tauri-drag-region
      className="relative flex h-8 shrink-0 items-center justify-center"
    >
      <span
        data-tauri-drag-region
        className="text-[13px] font-semibold text-neutral-900 select-none dark:text-neutral-50"
      >
        {title}
      </span>
      {onOpenEntries && (
        <Button
          type="button"
          variant="ghost"
          pressShift={false}
          onClick={onOpenEntries}
          aria-label="Open entries"
          title="Entries"
          className="absolute right-7.5 h-7 w-7 rounded-xl p-0 opacity-60 transition-opacity hover:bg-transparent hover:opacity-100 dark:hover:bg-transparent"
        >
          <List className="h-3.5 w-3.5" aria-hidden />
        </Button>
      )}
      {onOpenSettings && (
        <Button
          type="button"
          variant="ghost"
          pressShift={false}
          onClick={onOpenSettings}
          aria-label="Open settings"
          title="Settings"
          className="absolute right-0.5 h-7 w-7 rounded-xl p-0 opacity-60 transition-opacity hover:bg-transparent hover:opacity-100 dark:hover:bg-transparent"
        >
          <BoltIcon className="h-3.5 w-3.5" aria-hidden />
        </Button>
      )}
    </div>
  )
}
