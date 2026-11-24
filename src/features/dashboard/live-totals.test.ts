import assert from 'node:assert/strict'
import { describe, it } from 'node:test'

import {
  isTodayLive,
  selectLiveIntervalElapsedSecs,
  withActiveInterval,
} from './live-totals.ts'

const totals = { today: 10, thisWeek: 20, thisMonth: 30 }

describe('withActiveInterval', () => {
  it('treats missing persisted totals as zero', () => {
    assert.deepEqual(withActiveInterval(null, 15), {
      today: 15,
      thisWeek: 15,
      thisMonth: 15,
    })
  })

  it('returns the same totals when there is nothing to add', () => {
    assert.equal(withActiveInterval(totals, 0), totals)
  })

  it('adds the elapsed seconds to today, this week, and this month', () => {
    assert.deepEqual(withActiveInterval(totals, 5), {
      today: 15,
      thisWeek: 25,
      thisMonth: 35,
    })
  })
})

describe('selectLiveIntervalElapsedSecs', () => {
  it('returns the elapsed seconds while the engine is running', () => {
    assert.equal(
      selectLiveIntervalElapsedSecs({
        status: 'running',
        intervalElapsedSecs: 12,
        isBreak: false,
      }),
      12,
    )
  })

  it('stays at 0 across ticks when the engine is not running', () => {
    for (const status of ['idle', 'paused', 'completed'] as const) {
      assert.equal(
        selectLiveIntervalElapsedSecs({
          status,
          intervalElapsedSecs: 12,
          isBreak: false,
        }),
        0,
      )
      assert.equal(
        selectLiveIntervalElapsedSecs({
          status,
          intervalElapsedSecs: 13,
          isBreak: false,
        }),
        0,
      )
    }
  })

  it('returns 0 when there is no snapshot', () => {
    assert.equal(selectLiveIntervalElapsedSecs(null), 0)
  })

  it('returns 0 while a Break is running', () => {
    assert.equal(
      selectLiveIntervalElapsedSecs({
        status: 'running',
        intervalElapsedSecs: 40,
        isBreak: true,
      }),
      0,
    )
  })
})

describe('isTodayLive', () => {
  it('is live from the first second of a non-break Interval', () => {
    assert.equal(
      isTodayLive({
        status: 'running',
        intervalElapsedSecs: 0,
        isBreak: false,
      }),
      true,
    )
    assert.equal(
      isTodayLive({
        status: 'running',
        intervalElapsedSecs: 12,
        isBreak: false,
      }),
      true,
    )
  })

  it('is not live while idle, paused, completed, or on a Break', () => {
    for (const status of ['idle', 'paused', 'completed'] as const) {
      assert.equal(
        isTodayLive({
          status,
          intervalElapsedSecs: 12,
          isBreak: false,
        }),
        false,
      )
    }
    assert.equal(
      isTodayLive({
        status: 'running',
        intervalElapsedSecs: 40,
        isBreak: true,
      }),
      false,
    )
    assert.equal(isTodayLive(null), false)
  })
})
