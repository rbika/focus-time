import { useShallow } from 'zustand/react/shallow'

import {
  selectActiveInterval,
  withActiveEntry,
} from '@/features/entries/live-entries'
import { useEntries } from '@/features/entries/use-entries'
import { useTimerStore } from '@/store/timer-store'

/** Persisted entries plus a synthetic running entry while the engine is
 * running. The active interval is shallow-compared so a tick that does not
 * change mode or elapsed seconds keeps the previous object. */
export function useLiveEntries() {
  const entries = useEntries()
  const active = useTimerStore(
    useShallow((state) => selectActiveInterval(state.snapshot)),
  )
  return withActiveEntry(entries, active)
}
