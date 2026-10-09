import { useEffect, useRef, useState } from 'react'

import {
  dataForCurrentCalendarDay,
  localCalendarDayKey,
} from '@/features/stats/current-calendar-day'
import { onEntriesChanged, onEntryRecorded } from '@/lib/tauri'
import { useTimerStore } from '@/store/timer-store'

/** Fetches data via `fetcher` on mount and refetches it whenever Entries
 * change (recorded, edited, or deleted) or the local Calendar day changes.
 * Shared by the Dashboard and Entries tabs. A previous Calendar day's
 * payload is never returned. */
export function useEntryRecordedData<T>(fetcher: () => Promise<T>): {
  data: T | null
  refetch: () => void
} {
  const [data, setData] = useState<T | null>(null)
  const [loadedCalendarDay, setLoadedCalendarDay] = useState<string | null>(
    null,
  )
  const cancelledRef = useRef(false)
  const fetcherRef = useRef(fetcher)
  fetcherRef.current = fetcher
  const loadedCalendarDayRef = useRef<string | null>(null)
  const generationRef = useRef(0)

  const refetch = useRef(() => {
    const generation = ++generationRef.current
    const requestedDay = localCalendarDayKey(new Date())
    void fetcherRef.current().then((next) => {
      if (cancelledRef.current || generation !== generationRef.current) return
      loadedCalendarDayRef.current = requestedDay
      setData(next)
      setLoadedCalendarDay(requestedDay)
    })
  }).current

  useEffect(() => {
    cancelledRef.current = false
    refetch()

    const dropIfPreviousCalendarDay = () => {
      const loaded = loadedCalendarDayRef.current
      if (loaded == null) return
      if (loaded === localCalendarDayKey(new Date())) return
      loadedCalendarDayRef.current = null
      setData(null)
      setLoadedCalendarDay(null)
      refetch()
    }

    const unlistenRecorded = onEntryRecorded(refetch)
    const unlistenChanged = onEntriesChanged(refetch)
    const unsubscribeTick = useTimerStore.subscribe(dropIfPreviousCalendarDay)

    return () => {
      cancelledRef.current = true
      unsubscribeTick()
      void unlistenRecorded.then((fn) => fn())
      void unlistenChanged.then((fn) => fn())
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [])

  return {
    data: dataForCurrentCalendarDay(data, loadedCalendarDay, new Date()),
    refetch,
  }
}
