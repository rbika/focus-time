import type { CSSProperties } from 'react'

/** Shared crossfade. Both panels move together for 150ms ease-out.
 * The panel you are leaving fades out while the other fades in. Pass
 * `CROSSFADE_NUDGE_PX` to also slide: toward the right-hand panel both
 * travel left, and toward the left-hand panel both travel right. Pass 0
 * to fade in place. A panel does not take clicks while its opacity target
 * is 0. Reduced motion cuts to the destination.
 */
export const CROSSFADE_MS = 150
export const CROSSFADE_NUDGE_PX = 16

export type CrossfadeFrame = {
  x: number
  opacity: 0 | 1
  interactive: boolean
}

export function crossfadeFrames<L extends string, R extends string>(
  left: L,
  right: R,
  active: L | R,
  nudgePx: number,
): Record<L | R, CrossfadeFrame> {
  const showLeft = active === left
  return {
    [left]: {
      x: showLeft || nudgePx === 0 ? 0 : -nudgePx,
      opacity: showLeft ? 1 : 0,
      interactive: showLeft,
    },
    [right]: {
      x: showLeft ? nudgePx : 0,
      opacity: showLeft ? 0 : 1,
      interactive: !showLeft,
    },
  } as Record<L | R, CrossfadeFrame>
}

/** The panel that just became inactive, kept mounted so it can fade out.
 * Null when there is nothing to hold. */
export function crossfadeHolding<T extends string>(
  previous: T,
  active: T,
  reducedMotion: boolean,
): T | null {
  if (reducedMotion || previous === active) return null
  return previous
}

export function crossfadeStyle(
  frame: CrossfadeFrame,
  animate: boolean,
): CSSProperties {
  return {
    transform: `translateX(${frame.x}px)`,
    opacity: frame.opacity,
    pointerEvents: frame.interactive ? 'auto' : 'none',
    transition: animate
      ? `transform ${CROSSFADE_MS}ms ease-out, opacity ${CROSSFADE_MS}ms ease-out`
      : 'none',
  }
}
