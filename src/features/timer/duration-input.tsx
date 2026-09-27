import { useEffect, useRef } from 'react'

import { cn } from '@/utils/cn'
import { digitsToMask } from '@/utils/time'

type Props = {
  value: string
  onChange: (mask: string) => void
  onFocus?: () => void
  onBlur?: () => void
  onCommit?: () => void
  id?: string
  'aria-label'?: string
  className?: string
}

export function DurationInput({
  value,
  onChange,
  onFocus,
  onBlur,
  onCommit,
  id,
  'aria-label': ariaLabel = 'Duration',
  className,
}: Props) {
  const inputRef = useRef<HTMLInputElement>(null)

  const pinCaretToEnd = () => {
    const el = inputRef.current
    if (el) el.setSelectionRange(el.value.length, el.value.length)
  }

  // Controlled value re-renders can reset the caret; pin it back to the end
  // so digits always append (odometer-style) instead of inserting mid-string.
  useEffect(() => {
    if (document.activeElement === inputRef.current) pinCaretToEnd()
  }, [value])

  return (
    <input
      ref={inputRef}
      id={id}
      type="text"
      inputMode="numeric"
      autoComplete="off"
      spellCheck={false}
      aria-label={ariaLabel}
      value={value}
      onChange={(e) => onChange(digitsToMask(e.target.value))}
      onFocus={() => {
        pinCaretToEnd()
        onFocus?.()
      }}
      onBlur={onBlur}
      onClick={pinCaretToEnd}
      onSelect={pinCaretToEnd}
      onKeyDown={(e) => {
        if (['ArrowLeft', 'ArrowRight', 'Home', 'End'].includes(e.key)) {
          e.preventDefault()
        }
        if (e.key === 'Enter') {
          e.preventDefault()
          onCommit?.()
        }
      }}
      className={cn(
        'h-11 w-40 rounded-md border border-neutral-300 bg-white text-center text-3xl text-neutral-900 tabular-nums caret-transparent dark:border-neutral-600 dark:bg-neutral-800 dark:text-neutral-100',
        className,
      )}
    />
  )
}
