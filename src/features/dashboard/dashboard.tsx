import { isTodayLive } from '@/features/dashboard/live-totals'
import { useLiveTotals } from '@/features/dashboard/use-live-totals'
import { useTimerStore } from '@/store/timer-store'
import { secsToSummaryLabel } from '@/utils/time'

function formatTotal(seconds: number | undefined): string {
  if (seconds == null) return ''
  if (seconds === 0) return '–'
  return secsToSummaryLabel(seconds)
}

function TotalRow({
  label,
  seconds,
  live,
}: {
  label: string
  seconds: number | undefined
  live?: boolean
}) {
  return (
    <div className="flex flex-col gap-0.5">
      <span className="flex items-center gap-1.5 text-xs text-neutral-500 dark:text-neutral-400">
        {label}
        {live ? (
          <span
            className="pulse-dot size-1.5 shrink-0 rounded-full bg-current text-neutral-300 dark:text-neutral-600"
            aria-label="Running"
          />
        ) : null}
      </span>
      <span className="text-xl tabular-nums">{formatTotal(seconds)}</span>
    </div>
  )
}

export function Dashboard() {
  const totals = useLiveTotals()
  const todayLive = useTimerStore((s) => isTodayLive(s.snapshot))

  return (
    <div className="flex h-full flex-col px-4 pt-1 pb-4">
      {/* <h2 className="flex h-8 shrink-0 items-center text-sm font-medium text-neutral-900 dark:text-neutral-50">
        Focus Time
      </h2> */}
      <div className="flex flex-col gap-4 pt-3">
        <TotalRow label="Today" seconds={totals?.today} live={todayLive} />
        <TotalRow label="This Week" seconds={totals?.thisWeek} />
        <TotalRow label="This Month" seconds={totals?.thisMonth} />
      </div>
    </div>
  )
}
