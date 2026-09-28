import assert from 'node:assert/strict'
import { describe, it } from 'node:test'

import type { Entry } from '../../lib/tauri.ts'
import { periodEmptyMessage, visiblePeriodEntries } from './period.ts'

function unix(date: Date): number {
  return Math.floor(date.getTime() / 1000)
}

function entry(id: string, started: Date, durationSecs = 60): Entry {
  const startedAtUnix = unix(started)
  return {
    id,
    mode: 'manual',
    startedAtUnix,
    endedAtUnix: startedAtUnix + durationSecs,
    durationSecs,
  }
}

describe('visiblePeriodEntries', () => {
  describe('this week', () => {
    // Wednesday 16 Sep 2026. Week is Monday 14 Sep 00:00 → Monday 21 Sep 00:00.
    const now = new Date(2026, 8, 16, 15, 0, 0)

    it('includes an Entry from Monday 00:00 and excludes the Sunday before', () => {
      const mondayStart = entry('monday', new Date(2026, 8, 14, 0, 0, 0))
      const sundayBefore = entry('sunday', new Date(2026, 8, 13, 23, 59, 0))
      const view = visiblePeriodEntries(
        [mondayStart, sundayBefore],
        'this-week',
        now,
        0,
      )

      assert.deepEqual(
        view.entries.map((item) => item.id),
        ['monday'],
      )
      assert.equal(view.canShowMore, false)
    })
  })

  describe('this month', () => {
    const now = new Date(2026, 8, 16, 15, 0, 0)

    it('includes an Entry from the 1st 00:00 and excludes the previous month', () => {
      const first = entry('first', new Date(2026, 8, 1, 0, 0, 0))
      const previous = entry('august', new Date(2026, 7, 31, 23, 59, 0))
      const view = visiblePeriodEntries([first, previous], 'this-month', now, 0)

      assert.deepEqual(
        view.entries.map((item) => item.id),
        ['first'],
      )
    })
  })

  describe('this year', () => {
    const now = new Date(2026, 8, 16, 15, 0, 0)

    it('first shows the last three calendar months, clipped away from June', () => {
      const july = entry('july', new Date(2026, 6, 1, 0, 0, 0))
      const june = entry('june', new Date(2026, 5, 30, 23, 59, 0))
      const december = entry('december', new Date(2026, 11, 1, 12, 0, 0))
      const view = visiblePeriodEntries(
        [december, july, june],
        'this-year',
        now,
        0,
      )

      assert.deepEqual(
        view.entries.map((item) => item.id),
        ['july'],
      )
    })

    it('clips the first window to 1 Jan in early year', () => {
      const now = new Date(2026, 1, 10, 12, 0, 0)
      const january = entry('january', new Date(2026, 0, 1, 0, 0, 0))
      const lastDecember = entry('last-dec', new Date(2025, 11, 31, 23, 59, 0))
      const view = visiblePeriodEntries(
        [january, lastDecember],
        'this-year',
        now,
        0,
      )

      assert.deepEqual(
        view.entries.map((item) => item.id),
        ['january'],
      )
      assert.equal(view.canShowMore, false)
    })

    it('Show more extends the window start back three calendar months', () => {
      const april = entry('april', new Date(2026, 3, 1, 0, 0, 0))
      const march = entry('march', new Date(2026, 2, 31, 23, 59, 0))
      const view = visiblePeriodEntries([april, march], 'this-year', now, 1)

      assert.deepEqual(
        view.entries.map((item) => item.id),
        ['april'],
      )
    })

    it('offers Show more only when an older this-year Entry exists', () => {
      const july = entry('july', new Date(2026, 6, 15, 12, 0, 0))
      const january = entry('january', new Date(2026, 0, 10, 12, 0, 0))
      const lastYear = entry('last-year', new Date(2025, 11, 1, 12, 0, 0))

      const withOlder = visiblePeriodEntries(
        [july, january, lastYear],
        'this-year',
        now,
        0,
      )
      assert.deepEqual(
        withOlder.entries.map((item) => item.id),
        ['july'],
      )
      assert.equal(withOlder.canShowMore, true)

      const withoutOlder = visiblePeriodEntries([july], 'this-year', now, 0)
      assert.equal(withoutOlder.canShowMore, false)

      const fullyRevealed = visiblePeriodEntries(
        [july, january],
        'this-year',
        now,
        2,
      )
      assert.deepEqual(
        fullyRevealed.entries.map((item) => item.id),
        ['july', 'january'],
      )
      assert.equal(fullyRevealed.canShowMore, false)
    })

    it('never lists an Entry from an earlier year', () => {
      const lastYear = entry('last-year', new Date(2025, 8, 16, 12, 0, 0))
      const view = visiblePeriodEntries([lastYear], 'this-year', now, 20)

      assert.deepEqual(view.entries, [])
      assert.equal(view.canShowMore, false)
    })
  })
})

describe('periodEmptyMessage', () => {
  it('names This week when the slice is empty and Show more is not available', () => {
    assert.equal(
      periodEmptyMessage('this-week', 0, false),
      'No entries this week',
    )
  })

  it('names This month and This year the same way', () => {
    assert.equal(
      periodEmptyMessage('this-month', 0, false),
      'No entries this month',
    )
    assert.equal(
      periodEmptyMessage('this-year', 0, false),
      'No entries this year',
    )
  })

  it('is silent when rows exist or Show more can still reveal this year', () => {
    assert.equal(periodEmptyMessage('this-week', 1, false), null)
    assert.equal(periodEmptyMessage('this-year', 0, true), null)
  })
})
