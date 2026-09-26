import { useCallback, useEffect, useRef, useState } from 'react'

import {
  CROSSFADE_MS,
  CROSSFADE_NUDGE_PX,
  crossfadeFrames,
  type CrossfadeFrame,
} from '@/features/crossfade/crossfade'
import { usePrefersReducedMotion } from '@/features/crossfade/use-prefers-reduced-motion'

export type EditorDestination = 'list' | 'editor'

export function useEditorTransition(): {
  frames: Record<EditorDestination, CrossfadeFrame>
  animate: boolean
  destination: EditorDestination
  show: () => void
  hide: (onHidden: () => void) => void
  snapToList: () => void
} {
  const reducedMotion = usePrefersReducedMotion()
  const [destination, setDestination] = useState<EditorDestination>('list')
  const [suppress, setSuppress] = useState(false)
  const timers = useRef<number[]>([])
  const destinationRef = useRef(destination)
  const onHiddenRef = useRef<() => void>(() => {})
  destinationRef.current = destination

  const clearTimers = useCallback(() => {
    for (const id of timers.current) window.clearTimeout(id)
    timers.current = []
  }, [])

  const releaseHidden = useCallback(() => {
    const finish = onHiddenRef.current
    onHiddenRef.current = () => {}
    finish()
  }, [])

  useEffect(() => clearTimers, [clearTimers])

  useEffect(() => {
    if (!reducedMotion) return
    clearTimers()
    if (destinationRef.current === 'list') releaseHidden()
  }, [clearTimers, reducedMotion, releaseHidden])

  const start = useCallback(
    (next: EditorDestination) => {
      clearTimers()
      setSuppress(false)
      setDestination(next)
      if (reducedMotion) {
        if (next === 'list') releaseHidden()
        return
      }
      timers.current = [
        window.setTimeout(() => {
          if (next === 'list') releaseHidden()
        }, CROSSFADE_MS),
      ]
    },
    [clearTimers, reducedMotion, releaseHidden],
  )

  const show = useCallback(() => {
    onHiddenRef.current = () => {}
    start('editor')
  }, [start])

  const hide = useCallback(
    (onHidden: () => void) => {
      onHiddenRef.current = onHidden
      start('list')
    },
    [start],
  )

  const snapToList = useCallback(() => {
    clearTimers()
    onHiddenRef.current = () => {}
    setSuppress(true)
    setDestination('list')
  }, [clearTimers])

  return {
    frames: crossfadeFrames('list', 'editor', destination, CROSSFADE_NUDGE_PX),
    animate: !reducedMotion && !suppress,
    destination,
    show,
    hide,
    snapToList,
  }
}
