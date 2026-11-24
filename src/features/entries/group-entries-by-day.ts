import type { Entry } from '@/lib/tauri'

export type DaySection = {
  key: string
  label: string
  totalSecs: number
  entries: Entry[]
}

function localDayKey(date: Date): string {
  return `${date.getFullYear()}-${date.getMonth()}-${date.getDate()}`
}

function startOfLocalDay(date: Date): Date {
  return new Date(date.getFullYear(), date.getMonth(), date.getDate())
}

/** `Tue Sep 15` this year, `Tue Sep 15, 2025` otherwise. */
function formatOlderDayLabel(date: Date, includeYear: boolean): string {
  const parts = new Intl.DateTimeFormat(undefined, {
    weekday: 'short',
    month: 'short',
    day: 'numeric',
    ...(includeYear ? { year: 'numeric' as const } : {}),
  }).formatToParts(date)

  const weekday = parts.find((part) => part.type === 'weekday')?.value
  const month = parts.find((part) => part.type === 'month')?.value
  const day = parts.find((part) => part.type === 'day')?.value
  const year = parts.find((part) => part.type === 'year')?.value

  const head = [weekday, month, day].filter(Boolean).join(' ')
  return includeYear && year ? `${head}, ${year}` : head
}

export function formatDayLabel(date: Date, now: Date): string {
  const thatDay = startOfLocalDay(date).getTime()
  const today = startOfLocalDay(now).getTime()
  const diffDays = Math.round((today - thatDay) / 86_400_000)

  if (diffDays === 0) return 'Today'
  if (diffDays === 1) return 'Yesterday'
  return formatOlderDayLabel(date, date.getFullYear() !== now.getFullYear())
}

function sectionDayTime(section: DaySection): number {
  const first = section.entries[0]
  if (first == null) return 0
  return startOfLocalDay(new Date(first.startedAtUnix * 1000)).getTime()
}

/** Groups Entries into newest-first calendar-day sections using each
 * Entry's local start date. */
export function groupEntriesByDay(
  entries: Entry[],
  now: Date = new Date(),
): DaySection[] {
  const groups = new Map<string, Entry[]>()

  for (const entry of entries) {
    const started = new Date(entry.startedAtUnix * 1000)
    const key = localDayKey(started)
    const existing = groups.get(key)
    if (existing) {
      existing.push(entry)
    } else {
      groups.set(key, [entry])
    }
  }

  return [...groups.entries()]
    .map(([key, dayEntries]) => {
      const first = dayEntries[0]
      const started = new Date((first?.startedAtUnix ?? 0) * 1000)
      return {
        key,
        label: formatDayLabel(started, now),
        totalSecs: dayEntries.reduce(
          (sum, entry) => sum + entry.durationSecs,
          0,
        ),
        entries: dayEntries,
      }
    })
    .sort((a, b) => sectionDayTime(b) - sectionDayTime(a))
}
