import type { CSSProperties } from 'react'

/** Confirmation overlay motion. The panel fades and scales from 95% to
 * 100% over 150ms ease-out. The backdrop fades only. Dismiss reverses
 * that, then the overlay unmounts. Snap drops it immediately. Reduced
 * motion cuts to the end state. While closing, it stays up and swallows
 * clicks. */
export const DIALOG_MS = 150
export const DIALOG_HIDDEN_SCALE = 0.95

export type DialogPresence = 'closed' | 'opening' | 'open' | 'closing'

export type DialogEvent = 'show' | 'presented' | 'dismiss' | 'snap'

export type DialogFrame = {
  mounted: boolean
  shown: boolean
  animate: boolean
  acceptsActions: boolean
}

export function stepDialog(
  presence: DialogPresence,
  event: DialogEvent,
  reducedMotion: boolean,
): DialogPresence {
  if (event === 'snap') return 'closed'
  if (event === 'show') {
    if (presence !== 'closed') return presence
    return reducedMotion ? 'open' : 'opening'
  }
  if (event === 'presented') {
    return presence === 'opening' ? 'open' : presence
  }
  if (presence === 'closed' || presence === 'closing') return presence
  return reducedMotion ? 'closed' : 'closing'
}

export function dialogFrame(
  presence: DialogPresence,
  reducedMotion: boolean,
): DialogFrame {
  if (presence === 'closed') {
    return {
      mounted: false,
      shown: false,
      animate: false,
      acceptsActions: false,
    }
  }
  return {
    mounted: true,
    shown: presence === 'open',
    animate: !reducedMotion,
    acceptsActions: presence !== 'closing',
  }
}

export function dialogBackdropStyle(frame: DialogFrame): CSSProperties {
  return {
    opacity: frame.shown ? 1 : 0,
    transition: frame.animate ? `opacity ${DIALOG_MS}ms ease-out` : 'none',
  }
}

export function dialogPanelStyle(frame: DialogFrame): CSSProperties {
  return {
    opacity: frame.shown ? 1 : 0,
    transform: frame.shown ? 'scale(1)' : `scale(${DIALOG_HIDDEN_SCALE})`,
    transition: frame.animate
      ? `opacity ${DIALOG_MS}ms ease-out, transform ${DIALOG_MS}ms ease-out`
      : 'none',
    pointerEvents: frame.acceptsActions ? 'auto' : 'none',
  }
}
