import { useEntryRecordedData } from '@/features/stats/use-entry-recorded-data'
import { api, type Entry } from '@/lib/tauri'

/** Fetches all Entries (newest-first) and keeps them current when an Entry
 * is recorded elsewhere in the app (e.g. a tray-menu pause). A previous
 * Calendar day's list is dropped until the current day loads. */
export function useEntries() {
  const { data } = useEntryRecordedData<Entry[]>(api.getEntries)
  return data
}
