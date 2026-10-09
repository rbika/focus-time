import type { TimerSnapshot, Totals } from '@/lib/tauri'

type LiveSnapshot = Pick<
  TimerSnapshot,
  'status' | 'intervalElapsedSecs' | 'isBreak'
>

/** Elapsed seconds to overlay on persisted totals.
 *
 * Returns a stable `0` unless the engine is `running`. Zustand compares this
 * with `Object.is`, so ticks that only change `remainingSecs` or `formatted`
 * do not re-render the tiles. */
export function selectLiveIntervalElapsedSecs(
  snapshot: LiveSnapshot | null,
): number {
  if (snapshot?.status !== 'running') return 0
  if (snapshot.isBreak) return 0
  return snapshot.intervalElapsedSecs
}

/** Adds the active interval to each Dashboard total. A zero overlay returns
 * the same object so an idle snapshot does not churn the tiles. Missing
 * persisted totals count as zero so a running interval still shows. */
export function withActiveInterval(
  totals: Totals | null,
  intervalElapsedSecs: number,
): Totals | null {
  if (intervalElapsedSecs === 0) return totals
  const persisted = totals ?? { today: 0, thisWeek: 0, thisMonth: 0 }
  return {
    today: persisted.today + intervalElapsedSecs,
    thisWeek: persisted.thisWeek + intervalElapsedSecs,
    thisMonth: persisted.thisMonth + intervalElapsedSecs,
  }
}
