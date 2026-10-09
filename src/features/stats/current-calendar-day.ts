export function localCalendarDayKey(date: Date): string {
  return `${date.getFullYear()}-${date.getMonth()}-${date.getDate()}`
}

/** Cached Stats data is only shown when it was loaded on `now`'s Calendar day. */
export function dataForCurrentCalendarDay<T>(
  data: T | null,
  loadedCalendarDay: string | null,
  now: Date,
): T | null {
  if (data == null || loadedCalendarDay == null) return null
  if (loadedCalendarDay !== localCalendarDayKey(now)) return null
  return data
}
