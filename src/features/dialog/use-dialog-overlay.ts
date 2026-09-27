import { useCallback, useEffect, useState } from 'react'

import { usePrefersReducedMotion } from '@/features/crossfade/use-prefers-reduced-motion'
import {
  DIALOG_MS,
  dialogFrame,
  stepDialog,
  type DialogFrame,
  type DialogPresence,
} from '@/features/dialog/dialog-overlay'

export function useDialogOverlay(): {
  frame: DialogFrame
  show: () => void
  dismiss: () => void
  snap: () => void
} {
  const reducedMotion = usePrefersReducedMotion()
  const [presence, setPresence] = useState<DialogPresence>('closed')

  useEffect(() => {
    if (presence !== 'opening') return
    if (reducedMotion) {
      setPresence('open')
      return
    }
    const id = requestAnimationFrame(() => {
      setPresence((current) => stepDialog(current, 'presented', false))
    })
    return () => cancelAnimationFrame(id)
  }, [presence, reducedMotion])

  useEffect(() => {
    if (presence !== 'closing') return
    if (reducedMotion) {
      setPresence('closed')
      return
    }
    const id = window.setTimeout(() => {
      setPresence((current) => (current === 'closing' ? 'closed' : current))
    }, DIALOG_MS)
    return () => window.clearTimeout(id)
  }, [presence, reducedMotion])

  const show = useCallback(() => {
    setPresence((current) => stepDialog(current, 'show', reducedMotion))
  }, [reducedMotion])

  const dismiss = useCallback(() => {
    setPresence((current) => stepDialog(current, 'dismiss', reducedMotion))
  }, [reducedMotion])

  const snap = useCallback(() => {
    setPresence((current) => stepDialog(current, 'snap', reducedMotion))
  }, [reducedMotion])

  return {
    frame: dialogFrame(presence, reducedMotion),
    show,
    dismiss,
    snap,
  }
}
