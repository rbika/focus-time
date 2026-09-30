import { useEffect, useState } from 'react'

import {
  CROSSFADE_MS,
  crossfadeFrames,
  crossfadeHolding,
  crossfadeOrderedFrames,
  type CrossfadeFrame,
} from '@/features/crossfade/crossfade'
import { usePrefersReducedMotion } from '@/features/crossfade/use-prefers-reduced-motion'

function useHeldPanel<T extends string>(active: T, live: boolean) {
  const reducedMotion = usePrefersReducedMotion()
  const [tracked, setTracked] = useState<T | null>(null)
  const [holding, setHolding] = useState<T | null>(null)

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

  return { holding, animate: !reducedMotion }
}

export function useOrderedCrossfade<T extends string>(
  order: readonly T[],
  active: T,
  nudgePx: number,
  live = true,
): {
  frames: Record<T, CrossfadeFrame>
  animate: boolean
  mounted: Record<T, boolean>
} {
  const { holding, animate } = useHeldPanel(active, live)
  const mounted = {} as Record<T, boolean>
  for (const key of order) {
    mounted[key] = active === key || holding === key
  }

  return {
    frames: crossfadeOrderedFrames(order, active, nudgePx),
    animate,
    mounted,
  }
}

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
  const { holding, animate } = useHeldPanel(active, live)

  return {
    frames: crossfadeFrames(left, right, active, nudgePx),
    animate,
    mounted: {
      [left]: active === left || holding === left,
      [right]: active === right || holding === right,
    } as Record<L | R, boolean>,
  }
}
