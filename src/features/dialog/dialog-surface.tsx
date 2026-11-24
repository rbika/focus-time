import type { ReactNode } from 'react'

import { createPortal } from 'react-dom'

import {
  dialogBackdropStyle,
  dialogPanelStyle,
  type DialogFrame,
} from '@/features/dialog/dialog-overlay'

type Props = {
  frame: DialogFrame
  labelledBy: string
  describedBy: string
  onDismiss: () => void
  belowTitleBar?: boolean
  children: ReactNode
}

export function DialogSurface({
  frame,
  labelledBy,
  describedBy,
  onDismiss,
  belowTitleBar = false,
  children,
}: Props) {
  if (!frame.mounted) return null

  return createPortal(
    <div
      className={
        belowTitleBar
          ? 'fixed inset-x-0 top-8 bottom-0 z-50'
          : 'fixed inset-0 z-50'
      }
    >
      <div
        className="absolute inset-0 bg-black/40"
        style={dialogBackdropStyle(frame)}
        onClick={() => {
          if (frame.acceptsActions) onDismiss()
        }}
      />
      <div className="pointer-events-none absolute inset-0 flex items-center justify-center px-5">
        <div
          role="alertdialog"
          aria-labelledby={labelledBy}
          aria-describedby={describedBy}
          className="pointer-events-auto w-full max-w-90 rounded-xl bg-[canvas] p-4 shadow-lg"
          style={dialogPanelStyle(frame)}
          onClick={(event) => event.stopPropagation()}
        >
          {children}
        </div>
      </div>
    </div>,
    document.body,
  )
}
