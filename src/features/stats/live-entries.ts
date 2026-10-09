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

function startOfLocalDayTime(unixSecs: number): number {
  const date = new Date(unixSecs * 1000)
  return new Date(date.getFullYear(), date.getMonth(), date.getDate()).getTime()
}

/** Pins a synthetic running entry to the top of its Calendar day. A null
 * interval returns the same reference so an idle snapshot does not churn
 * the list. An unloaded list still shows the running entry. Assumes
 * `entries` is newest-first. */
export function withActiveEntry(
  entries: Entry[] | null,
  active: ActiveInterval | null,
  nowUnix: number = Math.floor(Date.now() / 1000),
): Entry[] | null {
  if (active == null) return entries
  const persisted = entries ?? []
  const running: Entry = {
    id: ACTIVE_ENTRY_ID,
    mode: active.mode,
    startedAtUnix: nowUnix - active.intervalElapsedSecs,
    endedAtUnix: nowUnix,
    durationSecs: active.intervalElapsedSecs,
  }
  const runningDay = startOfLocalDayTime(running.startedAtUnix)
  const insertAt = persisted.findIndex(
    (entry) => startOfLocalDayTime(entry.startedAtUnix) <= runningDay,
  )
  const index = insertAt === -1 ? persisted.length : insertAt
  return [...persisted.slice(0, index), running, ...persisted.slice(index)]
}
