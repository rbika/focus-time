import type { Entry, TimerMode, TimerSnapshot } from '@/lib/tauri'

/** Sentinel id for the UI-only running entry. Never persisted. */
export const ACTIVE_ENTRY_ID = '__active__'

export type ActiveInterval = {
  mode: TimerMode
  intervalElapsedSecs: number
}

type LiveSnapshot = Pick<
  TimerSnapshot,
  'status' | 'mode' | 'intervalElapsedSecs' | 'isBreak'
>

/** The in-flight interval, or `null` unless the engine is `running`.
 *
 * `null` is a stable reference. Zustand compares selector results with
 * `Object.is`, so ticks that only change `remainingSecs` or `formatted`
 * while the engine is idle, paused, or completed do not re-render Entries.
 * The running object is shallow-compared in `useLiveEntries`. */
export function selectActiveInterval(
  snapshot: LiveSnapshot | null,
): ActiveInterval | null {
  if (snapshot?.status !== 'running') return null
  if (snapshot.isBreak) return null
  return {
    mode: snapshot.mode,
    intervalElapsedSecs: snapshot.intervalElapsedSecs,
  }
}

/** Prepends a synthetic running entry. A null interval or an unloaded list
 * returns the same reference so an idle snapshot does not churn the list. */
export function withActiveEntry(
  entries: Entry[] | null,
  active: ActiveInterval | null,
  nowUnix: number = Math.floor(Date.now() / 1000),
): Entry[] | null {
  if (active == null || entries == null) return entries
  return [
    {
      id: ACTIVE_ENTRY_ID,
      mode: active.mode,
      startedAtUnix: nowUnix - active.intervalElapsedSecs,
      endedAtUnix: nowUnix,
      durationSecs: active.intervalElapsedSecs,
    },
    ...entries,
  ]
}
