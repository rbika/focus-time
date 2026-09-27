import assert from 'node:assert/strict'
import { describe, it } from 'node:test'

import {
  DIALOG_HIDDEN_SCALE,
  DIALOG_MS,
  dialogBackdropStyle,
  dialogFrame,
  dialogPanelStyle,
  stepDialog,
} from './dialog-overlay.ts'

describe('dialog overlay motion', () => {
  it('fades and scales over 150ms ease-out', () => {
    assert.equal(DIALOG_MS, 150)
    assert.equal(DIALOG_HIDDEN_SCALE, 0.95)
  })

  it('mounts hidden, then presents at full size', () => {
    const opening = stepDialog('closed', 'show', false)
    assert.equal(opening, 'opening')
    assert.deepEqual(dialogFrame(opening, false), {
      mounted: true,
      shown: false,
      animate: true,
      acceptsActions: true,
    })

    const open = stepDialog(opening, 'presented', false)
    assert.equal(open, 'open')
    assert.deepEqual(dialogFrame(open, false), {
      mounted: true,
      shown: true,
      animate: true,
      acceptsActions: true,
    })
  })

  it('reverses on dismiss and keeps swallowing clicks until unmount', () => {
    const closing = stepDialog('open', 'dismiss', false)
    assert.equal(closing, 'closing')
    assert.deepEqual(dialogFrame(closing, false), {
      mounted: true,
      shown: false,
      animate: true,
      acceptsActions: false,
    })
    assert.equal(stepDialog(closing, 'dismiss', false), 'closing')
    assert.equal(stepDialog(closing, 'show', false), 'closing')
  })

  it('drops immediately when snapped', () => {
    assert.equal(stepDialog('open', 'snap', false), 'closed')
    assert.equal(stepDialog('closing', 'snap', false), 'closed')
    assert.deepEqual(dialogFrame('closed', false), {
      mounted: false,
      shown: false,
      animate: false,
      acceptsActions: false,
    })
  })

  it('skips the motion when reduced motion is on', () => {
    assert.equal(stepDialog('closed', 'show', true), 'open')
    assert.deepEqual(dialogFrame('open', true), {
      mounted: true,
      shown: true,
      animate: false,
      acceptsActions: true,
    })
    assert.equal(stepDialog('open', 'dismiss', true), 'closed')
  })

  it('ignores show, dismiss, and presented when they do not apply', () => {
    assert.equal(stepDialog('open', 'show', false), 'open')
    assert.equal(stepDialog('closed', 'dismiss', false), 'closed')
    assert.equal(stepDialog('open', 'presented', false), 'open')
    assert.equal(stepDialog('closed', 'presented', false), 'closed')
  })
})

describe('dialog overlay styles', () => {
  const hidden = dialogFrame('opening', false)
  const shown = dialogFrame('open', false)
  const leaving = dialogFrame('closing', false)

  it('fades the backdrop without scaling it', () => {
    const backdrop = dialogBackdropStyle(hidden)
    assert.equal(backdrop.opacity, 0)
    assert.equal(backdrop.transition, `opacity ${DIALOG_MS}ms ease-out`)
    assert.equal('transform' in backdrop, false)

    const visible = dialogBackdropStyle(shown)
    assert.equal(visible.opacity, 1)
  })

  it('fades and scales the panel from 95% to 100%', () => {
    const panel = dialogPanelStyle(hidden)
    assert.equal(panel.opacity, 0)
    assert.equal(panel.transform, `scale(${DIALOG_HIDDEN_SCALE})`)
    assert.equal(
      panel.transition,
      `opacity ${DIALOG_MS}ms ease-out, transform ${DIALOG_MS}ms ease-out`,
    )

    const visible = dialogPanelStyle(shown)
    assert.equal(visible.opacity, 1)
    assert.equal(visible.transform, 'scale(1)')
    assert.equal(visible.pointerEvents, 'auto')
  })

  it('lets the panel ignore clicks while leaving', () => {
    const panel = dialogPanelStyle(leaving)
    assert.equal(panel.opacity, 0)
    assert.equal(panel.transform, `scale(${DIALOG_HIDDEN_SCALE})`)
    assert.equal(panel.pointerEvents, 'none')
  })

  it('cuts to the end state when motion is reduced', () => {
    const panel = dialogPanelStyle(dialogFrame('open', true))
    assert.equal(panel.opacity, 1)
    assert.equal(panel.transform, 'scale(1)')
    assert.equal(panel.transition, 'none')
    assert.equal(
      dialogBackdropStyle(dialogFrame('open', true)).transition,
      'none',
    )
  })
})
