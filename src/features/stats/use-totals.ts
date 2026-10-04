import { useEffect } from 'react'

import { useEntryRecordedData } from '@/features/stats/use-entry-recorded-data'
import { useRefetchAtLocalMidnight } from '@/features/stats/use-refetch-at-local-midnight'
import { api, onSettingsChanged, type Totals } from '@/lib/tauri'

/** Fetches Dashboard totals and keeps them current when an Entry is
 * recorded elsewhere in the app (e.g. a tray-menu pause). Refetches after
 * Settings persist so This Week follows Week start. */
export function useTotals() {
  const { data: totals, refetch } = useEntryRecordedData<Totals>(api.getTotals)
  useRefetchAtLocalMidnight(refetch)

  useEffect(() => {
    const unlisten = onSettingsChanged(() => {
      refetch()
    })
    return () => {
      void unlisten.then((fn) => fn())
    }
  }, [refetch])

  return totals
}
