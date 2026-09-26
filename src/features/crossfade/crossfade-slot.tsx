import type { ReactNode } from 'react'

import {
  crossfadeStyle,
  type CrossfadeFrame,
} from '@/features/crossfade/crossfade'

export function CrossfadeSlot({
  frame,
  animate,
  mounted,
  className,
  children,
}: {
  frame: CrossfadeFrame
  animate: boolean
  mounted: boolean
  className?: string
  children: ReactNode
}) {
  return (
    <div
      className={className}
      style={crossfadeStyle(frame, animate)}
      inert={!frame.interactive ? true : undefined}
    >
      {mounted ? children : null}
    </div>
  )
}
