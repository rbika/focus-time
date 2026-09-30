import assert from 'node:assert/strict'
import { describe, it } from 'node:test'

import {
  CROSSFADE_MS,
  CROSSFADE_NUDGE_PX,
  crossfadeFrames,
  crossfadeHolding,
  crossfadeOrderedFrames,
} from './crossfade.ts'

describe('crossfadeFrames', () => {
  it('moves both panels in one 150ms step', () => {
    assert.equal(CROSSFADE_MS, 150)
    assert.equal(CROSSFADE_NUDGE_PX, 16)
  })

  it('slides left toward the right-hand panel', () => {
    assert.deepEqual(
      crossfadeFrames('list', 'editor', 'editor', CROSSFADE_NUDGE_PX),
      {
        list: { x: -CROSSFADE_NUDGE_PX, opacity: 0, interactive: false },
        editor: { x: 0, opacity: 1, interactive: true },
      },
    )
  })

  it('slides right toward the left-hand panel', () => {
    assert.deepEqual(
      crossfadeFrames('list', 'editor', 'list', CROSSFADE_NUDGE_PX),
      {
        list: { x: 0, opacity: 1, interactive: true },
        editor: { x: CROSSFADE_NUDGE_PX, opacity: 0, interactive: false },
      },
    )
  })

  it('fades with no nudge when the distance is 0', () => {
    assert.deepEqual(crossfadeFrames('timer', 'stats', 'stats', 0), {
      timer: { x: 0, opacity: 0, interactive: false },
      stats: { x: 0, opacity: 1, interactive: true },
    })
    assert.deepEqual(crossfadeFrames('timer', 'stats', 'timer', 0), {
      timer: { x: 0, opacity: 1, interactive: true },
      stats: { x: 0, opacity: 0, interactive: false },
    })
  })
})

describe('crossfadeHolding', () => {
  it('keeps the panel you left mounted through the fade', () => {
    assert.equal(crossfadeHolding('dashboard', 'entries', false), 'dashboard')
    assert.equal(crossfadeHolding('entries', 'dashboard', false), 'entries')
  })

  it('drops the other panel when nothing changed or motion is reduced', () => {
    assert.equal(crossfadeHolding('dashboard', 'dashboard', false), null)
    assert.equal(crossfadeHolding('timer', 'stopwatch', true), null)
  })
})

describe('crossfadeOrderedFrames', () => {
  const order = ['timer', 'stopwatch', 'pomodoro'] as const

  it('slides left toward a later panel, including a skip', () => {
    assert.deepEqual(
      crossfadeOrderedFrames(order, 'pomodoro', CROSSFADE_NUDGE_PX),
      {
        timer: { x: -CROSSFADE_NUDGE_PX, opacity: 0, interactive: false },
        stopwatch: { x: -CROSSFADE_NUDGE_PX, opacity: 0, interactive: false },
        pomodoro: { x: 0, opacity: 1, interactive: true },
      },
    )
  })

  it('slides right toward an earlier panel, including a skip', () => {
    assert.deepEqual(
      crossfadeOrderedFrames(order, 'timer', CROSSFADE_NUDGE_PX),
      {
        timer: { x: 0, opacity: 1, interactive: true },
        stopwatch: { x: CROSSFADE_NUDGE_PX, opacity: 0, interactive: false },
        pomodoro: { x: CROSSFADE_NUDGE_PX, opacity: 0, interactive: false },
      },
    )
  })

  it('sits neighbors on both sides of a middle panel', () => {
    assert.deepEqual(
      crossfadeOrderedFrames(order, 'stopwatch', CROSSFADE_NUDGE_PX),
      {
        timer: { x: -CROSSFADE_NUDGE_PX, opacity: 0, interactive: false },
        stopwatch: { x: 0, opacity: 1, interactive: true },
        pomodoro: { x: CROSSFADE_NUDGE_PX, opacity: 0, interactive: false },
      },
    )
  })

  it('fades with no nudge when the distance is 0', () => {
    assert.deepEqual(crossfadeOrderedFrames(order, 'pomodoro', 0), {
      timer: { x: 0, opacity: 0, interactive: false },
      stopwatch: { x: 0, opacity: 0, interactive: false },
      pomodoro: { x: 0, opacity: 1, interactive: true },
    })
  })
})
