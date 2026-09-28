import type { Entry } from '@/lib/tauri'

export type Period = 'this-week' | 'this-month' | 'this-year'

export const DEFAULT_PERIOD: Period = 'this-week'

export const PERIOD_OPTIONS = [
  { value: 'this-week', label: 'This Week' },
  { value: 'this-month', label: 'This Month' },
  { value: 'this-year', label: 'This Year' },
] as const

export function isPeriod(value: string): value is Period {
  return PERIOD_OPTIONS.some((option) => option.value === value)
}

export type PeriodView = {
  entries: Entry[]
  canShowMore: boolean
}

function startOfLocalDay(date: Date): Date {
  return new Date(date.getFullYear(), date.getMonth(), date.getDate())
}

function startOfWeekMonday(now: Date): Date {
  const today = startOfLocalDay(now)
  const daysSinceMonday = (today.getDay() + 6) % 7
  return new Date(
    today.getFullYear(),
    today.getMonth(),
    today.getDate() - daysSinceMonday,
  )
}

function unixSecs(date: Date): number {
  return Math.floor(date.getTime() / 1000)
}

function periodBounds(
  period: Period,
  now: Date,
  yearRevealSteps: number,
): { start: Date; end: Date } {
  if (period === 'this-week') {
    const start = startOfWeekMonday(now)
    return {
      start,
      end: new Date(start.getFullYear(), start.getMonth(), start.getDate() + 7),
    }
  }

  const end = new Date(now.getFullYear(), now.getMonth() + 1, 1)
  if (period === 'this-month') {
    return { start: new Date(now.getFullYear(), now.getMonth(), 1), end }
  }

  const yearStart = new Date(now.getFullYear(), 0, 1)
  const unclipped = new Date(
    now.getFullYear(),
    now.getMonth() - (2 + 3 * yearRevealSteps),
    1,
  )
  const start = unclipped < yearStart ? yearStart : unclipped
  return { start, end }
}

export function visiblePeriodEntries(
  entries: Entry[],
  period: Period,
  now: Date,
  yearRevealSteps: number,
): PeriodView {
  const { start, end } = periodBounds(period, now, yearRevealSteps)
  const startUnix = unixSecs(start)
  const endUnix = unixSecs(end)
  const visible = entries.filter(
    (entry) =>
      entry.startedAtUnix >= startUnix && entry.startedAtUnix < endUnix,
  )

  if (period !== 'this-year') {
    return { entries: visible, canShowMore: false }
  }

  const yearStartUnix = unixSecs(new Date(now.getFullYear(), 0, 1))
  const canShowMore = entries.some(
    (entry) =>
      entry.startedAtUnix >= yearStartUnix && entry.startedAtUnix < startUnix,
  )

  return { entries: visible, canShowMore }
}

export function periodEmptyMessage(
  period: Period,
  visibleCount: number,
  canShowMore: boolean,
): string | null {
  if (visibleCount > 0 || canShowMore) return null
  if (period === 'this-week') return 'No entries this week'
  if (period === 'this-month') return 'No entries this month'
  return 'No entries this year'
}
