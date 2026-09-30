import type { CSSProperties } from 'react'

/** Shared crossfade. Panels move together for 150ms ease-out.
 * The panel you are leaving fades out while the other fades in. Binary
 * `crossfadeFrames` names a left and a right panel. `crossfadeOrderedFrames`
 * does the same for a tab order: toward a later panel both travel left,
 * toward an earlier panel both travel right, and a skip still nudges once.
 * Pass 0 to fade in place. A panel does not take clicks while its opacity
 * target is 0. Reduced motion cuts to the destination.
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
  return crossfadeOrderedFrames([left, right], active, nudgePx)
}

/** Ordered panels. The active panel sits at 0. Earlier ones sit to the
 * left, later ones to the right, so a skip still nudges 16px the same way
 * an adjacent switch does. Pass 0 to fade in place. */
export function crossfadeOrderedFrames<T extends string>(
  order: readonly T[],
  active: T,
  nudgePx: number,
): Record<T, CrossfadeFrame> {
  const activeIndex = order.indexOf(active)
  const frames = {} as Record<T, CrossfadeFrame>
  for (const [index, key] of order.entries()) {
    const show = key === active
    frames[key] = {
      x: nudgePx === 0 || show ? 0 : index < activeIndex ? -nudgePx : nudgePx,
      opacity: show ? 1 : 0,
      interactive: show,
    }
  }
  return frames
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
