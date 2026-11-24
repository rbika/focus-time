import {
  selectLiveIntervalElapsedSecs,
  withActiveInterval,
} from '@/features/dashboard/live-totals'
import { useTotals } from '@/features/dashboard/use-totals'
import { useTimerStore } from '@/store/timer-store'

/** Persisted Dashboard totals plus the active interval while the engine is
 * running. */
export function useLiveTotals() {
  const totals = useTotals()
  const intervalElapsedSecs = useTimerStore((s) =>
    selectLiveIntervalElapsedSecs(s.snapshot),
  )
  return withActiveInterval(totals, intervalElapsedSecs)
}
