import assert from 'node:assert/strict'
import { describe, it } from 'node:test'

import type { Entry } from '../../lib/tauri.ts'
import { groupEntriesByDay } from './group-entries-by-day.ts'
import {
  ACTIVE_ENTRY_ID,
  selectActiveInterval,
  withActiveEntry,
} from './live-entries.ts'

const now = new Date(2026, 8, 21, 15, 0, 0)
const nowUnix = Math.floor(now.getTime() / 1000)

const persisted: Entry = {
  id: 'persisted',
  mode: 'manual',
  startedAtUnix: nowUnix - 3600,
  endedAtUnix: nowUnix - 3000,
  durationSecs: 600,
}

describe('selectActiveInterval', () => {
  it('returns mode and elapsed seconds while the engine is running', () => {
    assert.deepEqual(
      selectActiveInterval({
        status: 'running',
        mode: 'stopwatch',
        intervalElapsedSecs: 12,
        isBreak: false,
      }),
      { mode: 'stopwatch', intervalElapsedSecs: 12 },
    )
  })

  it('stays null across ticks when the engine is not running', () => {
    for (const status of ['idle', 'paused', 'completed'] as const) {
      assert.equal(
        selectActiveInterval({
          status,
          mode: 'timer',
          intervalElapsedSecs: 12,
          isBreak: false,
        }),
        null,
      )
      assert.equal(
        selectActiveInterval({
          status,
          mode: 'stopwatch',
          intervalElapsedSecs: 13,
          isBreak: false,
        }),
        null,
      )
    }
  })

  it('returns null when there is no snapshot', () => {
    assert.equal(selectActiveInterval(null), null)
  })

  it('stays null while a Break is running', () => {
    assert.equal(
      selectActiveInterval({
        status: 'running',
        mode: 'pomodoro',
        intervalElapsedSecs: 40,
        isBreak: true,
      }),
      null,
    )
  })
})

describe('withActiveEntry', () => {
  it('returns the same entries when there is no active interval', () => {
    const entries = [persisted]
    assert.equal(withActiveEntry(entries, null, nowUnix), entries)
  })

  it('returns null entries unchanged', () => {
    assert.equal(
      withActiveEntry(
        null,
        { mode: 'timer', intervalElapsedSecs: 15 },
        nowUnix,
      ),
      null,
    )
  })

  it('prepends a synthetic running entry', () => {
    const entries = withActiveEntry(
      [persisted],
      { mode: 'timer', intervalElapsedSecs: 40 },
      nowUnix,
    )

    assert.deepEqual(entries, [
      {
        id: ACTIVE_ENTRY_ID,
        mode: 'timer',
        startedAtUnix: nowUnix - 40,
        endedAtUnix: nowUnix,
        durationSecs: 40,
      },
      persisted,
    ])
  })

  it('pins running above a later-start Entry on the same Calendar day', () => {
    const later: Entry = {
      id: 'later',
      mode: 'manual',
      startedAtUnix: nowUnix - 10 * 60,
      endedAtUnix: nowUnix - 5 * 60,
      durationSecs: 5 * 60,
    }
    const entries = withActiveEntry(
      [later],
      { mode: 'timer', intervalElapsedSecs: 40 * 60 },
      nowUnix,
    )

    assert.deepEqual(
      entries?.map((entry) => entry.id),
      [ACTIVE_ENTRY_ID, 'later'],
    )
  })

  it('orders calendar-day sections newest first', () => {
    const yesterday: Entry = {
      id: 'yesterday',
      mode: 'timer',
      startedAtUnix: nowUnix - 86_400,
      endedAtUnix: nowUnix - 86_400 + 600,
      durationSecs: 600,
    }
    const sections = groupEntriesByDay([yesterday, persisted], now)
    assert.deepEqual(
      sections.map((section) => section.label),
      ['Today', 'Yesterday'],
    )
  })

  it('groups the running entry into the day it started and adds its elapsed time', () => {
    const entries = withActiveEntry(
      [persisted],
      { mode: 'stopwatch', intervalElapsedSecs: 90 },
      nowUnix,
    )
    assert.ok(entries)
    const sections = groupEntriesByDay(entries, now)

    assert.equal(sections.length, 1)
    assert.equal(sections[0]?.label, 'Today')
    assert.deepEqual(
      sections[0]?.entries.map((entry) => entry.id),
      [ACTIVE_ENTRY_ID, 'persisted'],
    )
    assert.equal(sections[0]?.entries[0]?.mode, 'stopwatch')
    assert.equal(sections[0]?.totalSecs, 690)
  })

  it('pins a running entry to the top of its Calendar day', () => {
    const justAfterMidnight = new Date(2026, 8, 21, 0, 10, 0)
    const justAfterMidnightUnix = Math.floor(justAfterMidnight.getTime() / 1000)
    const todayEntry: Entry = {
      id: 'today',
      mode: 'timer',
      startedAtUnix: justAfterMidnightUnix - 5 * 60,
      endedAtUnix: justAfterMidnightUnix - 60,
      durationSecs: 4 * 60,
    }
    const yesterdayEntry: Entry = {
      id: 'yesterday',
      mode: 'manual',
      startedAtUnix: justAfterMidnightUnix - 26 * 60,
      endedAtUnix: justAfterMidnightUnix - 25 * 60,
      durationSecs: 60,
    }
    const entries = withActiveEntry(
      [todayEntry, yesterdayEntry],
      { mode: 'timer', intervalElapsedSecs: 20 * 60 },
      justAfterMidnightUnix,
    )

    assert.deepEqual(
      entries?.map((entry) => entry.id),
      ['today', ACTIVE_ENTRY_ID, 'yesterday'],
    )
  })

  it('keeps Today above a running entry that started yesterday', () => {
    const justAfterMidnight = new Date(2026, 8, 21, 0, 10, 0)
    const justAfterMidnightUnix = Math.floor(justAfterMidnight.getTime() / 1000)
    const todayEntry: Entry = {
      id: 'today',
      mode: 'timer',
      startedAtUnix: justAfterMidnightUnix - 5 * 60,
      endedAtUnix: justAfterMidnightUnix - 60,
      durationSecs: 4 * 60,
    }
    const elapsed = 20 * 60
    const entries = withActiveEntry(
      [todayEntry],
      { mode: 'timer', intervalElapsedSecs: elapsed },
      justAfterMidnightUnix,
    )
    assert.ok(entries)
    const sections = groupEntriesByDay(entries, justAfterMidnight)

    assert.deepEqual(
      sections.map((section) => section.label),
      ['Today', 'Yesterday'],
    )
    assert.deepEqual(
      sections[0]?.entries.map((entry) => entry.id),
      ['today'],
    )
    assert.deepEqual(
      sections[1]?.entries.map((entry) => entry.id),
      [ACTIVE_ENTRY_ID],
    )
  })

  it('places a running entry that started before midnight in yesterday', () => {
    const justAfterMidnight = new Date(2026, 8, 21, 0, 10, 0)
    const justAfterMidnightUnix = Math.floor(justAfterMidnight.getTime() / 1000)
    const elapsed = 20 * 60
    const entries = withActiveEntry(
      [],
      { mode: 'timer', intervalElapsedSecs: elapsed },
      justAfterMidnightUnix,
    )
    assert.ok(entries)
    const sections = groupEntriesByDay(entries, justAfterMidnight)

    assert.equal(sections.length, 1)
    assert.equal(sections[0]?.label, 'Yesterday')
    assert.equal(sections[0]?.entries[0]?.id, ACTIVE_ENTRY_ID)
    assert.equal(sections[0]?.totalSecs, elapsed)
  })
})
