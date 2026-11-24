import type { TimerSnapshot, Totals } from '@/lib/tauri'

type LiveSnapshot = Pick<
  TimerSnapshot,
  'status' | 'intervalElapsedSecs' | 'isBreak'
>

function isNonBreakRunning(
  snapshot: LiveSnapshot | null,
): snapshot is LiveSnapshot {
  return snapshot?.status === 'running' && !snapshot.isBreak
}

/** Elapsed seconds to overlay on persisted totals.
 *
 * Returns a stable `0` unless the engine is `running`. Zustand compares this
 * with `Object.is`, so ticks that only change `remainingSecs` or `formatted`
 * do not re-render the totals. */
export function selectLiveIntervalElapsedSecs(
  snapshot: LiveSnapshot | null,
): number {
  if (!isNonBreakRunning(snapshot)) return 0
  return snapshot.intervalElapsedSecs
}

/** Whether Today shows the live running indicator. Stable across ticks
 * while the engine stays running, so Dashboard does not re-render each
 * second. */
export function isTodayLive(snapshot: LiveSnapshot | null): boolean {
  return isNonBreakRunning(snapshot)
}

/** Adds the active interval to each Dashboard total. A zero overlay returns
 * the same object so an idle snapshot does not churn the totals. Missing
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
