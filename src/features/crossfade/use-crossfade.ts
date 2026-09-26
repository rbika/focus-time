import { useEffect, useState } from 'react'

import {
  CROSSFADE_MS,
  crossfadeFrames,
  crossfadeHolding,
  type CrossfadeFrame,
} from '@/features/crossfade/crossfade'
import { usePrefersReducedMotion } from '@/features/crossfade/use-prefers-reduced-motion'

export function useCrossfade<L extends string, R extends string>(
  left: L,
  right: R,
  active: L | R,
  nudgePx: number,
  live = true,
): {
  frames: Record<L | R, CrossfadeFrame>
  animate: boolean
  mounted: Record<L | R, boolean>
} {
  const reducedMotion = usePrefersReducedMotion()
  const [tracked, setTracked] = useState<L | R | null>(null)
  const [holding, setHolding] = useState<L | R | null>(null)

  if (live && tracked !== active) {
    const previous = tracked
    setTracked(active)
    setHolding(
      previous == null
        ? null
        : crossfadeHolding(previous, active, reducedMotion),
    )
  }

  useEffect(() => {
    if (holding == null) return
    const id = window.setTimeout(() => setHolding(null), CROSSFADE_MS)
    return () => window.clearTimeout(id)
  }, [holding])

  return {
    frames: crossfadeFrames(left, right, active, nudgePx),
    animate: !reducedMotion,
    mounted: {
      [left]: active === left || holding === left,
      [right]: active === right || holding === right,
    } as Record<L | R, boolean>,
  }
}
