import { useCallback, useEffect, useRef, useState } from 'react'

import { Hourglass, Pause, Play, Timer, X } from 'lucide-react'

import { Button } from '@/components/ui/button'
import { CROSSFADE_NUDGE_PX } from '@/features/crossfade/crossfade'
import { CrossfadeSlot } from '@/features/crossfade/crossfade-slot'
import { useCrossfade } from '@/features/crossfade/use-crossfade'
import { DurationInput } from '@/features/timer/duration-input'
import { ModeSwitch } from '@/features/timer/mode-switch'
import {
  DIALOG_FADE_MS,
  RunningIntervalDialog,
} from '@/features/timer/running-interval-dialog'
import { TimerProgress } from '@/features/timer/timer-progress'
import { api, onMainWindowHidden } from '@/lib/tauri'
import type { TimerMode } from '@/lib/tauri'
import { useTimerStore } from '@/store/timer-store'
import { cn } from '@/utils/cn'
import { maskToSecs, secsToCompact, secsToMask } from '@/utils/time'

export function TimerView({ active }: { active: boolean }) {
  const snapshot = useTimerStore((s) => s.snapshot)
  const settings = useTimerStore((s) => s.settings)
  const ready = useTimerStore((s) => s.ready)
  const togglePause = useTimerStore((s) => s.actions.togglePause)
  const reset = useTimerStore((s) => s.actions.reset)
  const discard = useTimerStore((s) => s.actions.discard)
  const setDuration = useTimerStore((s) => s.actions.setDuration)
  const setMode = useTimerStore((s) => s.actions.setMode)
  const modeFade = useCrossfade(
    'timer',
    'stopwatch',
    snapshot?.mode === 'stopwatch' ? 'stopwatch' : 'timer',
    CROSSFADE_NUDGE_PX,
    snapshot != null,
  )
  const runFade = useCrossfade(
    'idle',
    'running',
    snapshot?.status === 'running' || snapshot?.status === 'paused'
      ? 'running'
      : 'idle',
    0,
    snapshot != null,
  )

  const [mask, setMask] = useState('00:00:00')
  const [dialog, setDialog] = useState<'closed' | 'open' | 'closing'>('closed')
  const dialogRef = useRef(dialog)
  dialogRef.current = dialog
  const closeTimerRef = useRef<number | null>(null)
  const editingRef = useRef(false)
  const debounceRef = useRef<ReturnType<typeof setTimeout> | null>(null)
  const genRef = useRef(0)
  const intendedSecsRef = useRef<number | null>(null)
  const queueRef = useRef(Promise.resolve())

  const runExclusive = useCallback((task: () => Promise<void>) => {
    const run = queueRef.current.then(task, task)
    queueRef.current = run.then(
      () => undefined,
      () => undefined,
    )
    return run
  }, [])

  const closeDialog = useCallback((immediate: boolean) => {
    if (dialogRef.current === 'closed') return
    if (closeTimerRef.current != null) {
      window.clearTimeout(closeTimerRef.current)
      closeTimerRef.current = null
    }
    if (immediate) {
      setDialog('closed')
      return
    }
    setDialog('closing')
    closeTimerRef.current = window.setTimeout(() => {
      setDialog('closed')
      closeTimerRef.current = null
    }, DIALOG_FADE_MS)
  }, [])

  const syncDuration = useCallback(
    async (secs: number, gen: number) => {
      if (gen !== genRef.current) return
      const current = useTimerStore.getState().snapshot
      if (!current) return
      if (current.status !== 'idle' && current.status !== 'completed') return
      if (current.durationSecs === secs) return
      try {
        await setDuration(secs)
      } catch {
        if (gen === genRef.current) intendedSecsRef.current = null
      }
    },
    [setDuration],
  )

  const commitDuration = useCallback(
    (nextMask: string) => {
      if (debounceRef.current) {
        clearTimeout(debounceRef.current)
        debounceRef.current = null
      }
      const gen = ++genRef.current
      const secs = maskToSecs(nextMask)
      intendedSecsRef.current = secs
      return runExclusive(() => syncDuration(secs, gen))
    },
    [runExclusive, syncDuration],
  )

  const handleMaskChange = useCallback(
    (next: string) => {
      setMask(next)
      if (debounceRef.current) clearTimeout(debounceRef.current)
      const gen = ++genRef.current
      const secs = maskToSecs(next)
      intendedSecsRef.current = secs
      debounceRef.current = setTimeout(() => {
        debounceRef.current = null
        void runExclusive(() => syncDuration(secs, gen))
      }, 200)
    },
    [runExclusive, syncDuration],
  )

  const applyPreset = useCallback(
    (secs: number) => {
      editingRef.current = false
      const next = secsToMask(secs)
      setMask(next)
      void commitDuration(next)
    },
    [commitDuration],
  )

  const handleStart = useCallback(async () => {
    if (debounceRef.current) {
      clearTimeout(debounceRef.current)
      debounceRef.current = null
    }
    const current = useTimerStore.getState().snapshot
    if (current?.mode === 'stopwatch') {
      await togglePause()
      return
    }
    const gen = ++genRef.current
    const secs = maskToSecs(mask)
    intendedSecsRef.current = secs
    await runExclusive(async () => {
      await syncDuration(secs, gen)
      if (secs > 0) await togglePause()
    })
  }, [mask, runExclusive, syncDuration, togglePause])

  const handleModeChange = useCallback(
    (mode: TimerMode) => {
      void setMode(mode)
    },
    [setMode],
  )

  useEffect(() => {
    return () => {
      if (debounceRef.current) clearTimeout(debounceRef.current)
    }
  }, [])

  // Engine is the source of truth when we're not typing. Ignore snapshots
  // that don't match the latest local intent (stale in-flight setDuration).
  useEffect(() => {
    const current = useTimerStore.getState().snapshot
    if (!current) return
    if (editingRef.current) return
    if (current.status !== 'idle' && current.status !== 'completed') return
    if (
      intendedSecsRef.current !== null &&
      current.durationSecs !== intendedSecsRef.current
    ) {
      return
    }
    intendedSecsRef.current = null
    setMask((mask) => {
      const next = secsToMask(current.durationSecs)
      return mask === next ? mask : next
    })
  }, [snapshot?.status, snapshot?.durationSecs])

  const handleCancel = useCallback(() => {
    const current = useTimerStore.getState().snapshot
    if (current?.status === 'running' && current.intervalElapsedSecs > 10) {
      if (closeTimerRef.current != null) {
        window.clearTimeout(closeTimerRef.current)
        closeTimerRef.current = null
      }
      setDialog('open')
      return
    }
    void reset()
  }, [reset])

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.metaKey && event.key === ',') {
        event.preventDefault()
        void api.openSettings()
        return
      }
      if (event.metaKey && event.key.toLowerCase() === 'q') {
        event.preventDefault()
        void api.quitApp()
        return
      }
      if (event.metaKey && event.key.toLowerCase() === 's') {
        event.preventDefault()
        if (dialog !== 'closed' || !snapshot) return
        const isActive =
          snapshot.status === 'running' || snapshot.status === 'paused'
        if (isActive) {
          void togglePause()
        } else {
          void handleStart()
        }
        return
      }
      if (event.metaKey && event.key.toLowerCase() === 'x') {
        if (isTextInput(event.target)) return
        if (dialog !== 'closed') {
          event.preventDefault()
          return
        }
        const current = useTimerStore.getState().snapshot
        const isActive =
          current?.status === 'running' || current?.status === 'paused'
        if (!isActive) return
        event.preventDefault()
        handleCancel()
        return
      }
      if (event.key === 'Escape') {
        event.preventDefault()
        if (dialog !== 'closed') {
          closeDialog(false)
          return
        }
        void api.hideTimerWindow()
        return
      }
    }
    window.addEventListener('keydown', onKeyDown)
    return () => window.removeEventListener('keydown', onKeyDown)
  }, [snapshot, togglePause, handleStart, handleCancel, dialog, closeDialog])

  useEffect(() => {
    if (!active) closeDialog(false)
  }, [active, closeDialog])

  useEffect(() => {
    if (snapshot?.status !== 'running') closeDialog(false)
  }, [snapshot?.status, closeDialog])

  useEffect(() => {
    let unlisten: (() => void) | undefined
    void onMainWindowHidden(() => closeDialog(true)).then((fn) => {
      unlisten = fn
    })
    return () => {
      unlisten?.()
    }
  }, [closeDialog])

  useEffect(() => {
    return () => {
      if (closeTimerRef.current != null) {
        window.clearTimeout(closeTimerRef.current)
      }
    }
  }, [])

  if (!ready || !snapshot) {
    return (
      <div className="flex h-full items-center justify-center text-sm text-neutral-500">
        Loading…
      </div>
    )
  }

  const isRunning = snapshot.status === 'running'
  const isPaused = snapshot.status === 'paused'
  const isStopwatch = snapshot.mode === 'stopwatch'
  const canStart = isStopwatch || maskToSecs(mask) > 0
  const endsAt = isRunning
    ? new Date(Date.now() + snapshot.remainingSecs * 1000).toLocaleTimeString(
        undefined,
        { hour: 'numeric', minute: '2-digit' },
      )
    : '--:--'
  const configuredPresets = (settings?.presets ?? []).flatMap((secs, index) =>
    secs != null && secs > 0 ? [{ index, secs }] : [],
  )

  return (
    <div className="flex h-full flex-col">
      <main className="relative min-h-0 flex-1">
        <CrossfadeSlot
          frame={runFade.frames.running}
          animate={runFade.animate}
          mounted={runFade.mounted.running}
          className="absolute inset-0"
        >
          <div className="flex h-full w-full flex-col items-center justify-between gap-3 px-4 pt-1 pb-4">
            <div
              className={cn(
                'flex h-7 w-full shrink-0 items-center justify-center gap-1.5 text-sm font-medium text-neutral-900 transition-opacity duration-200 dark:text-neutral-50',
                isPaused && 'opacity-60',
              )}
              aria-label={isStopwatch ? 'Stopwatch mode' : 'Timer mode'}
            >
              {isStopwatch ? (
                <>
                  <Timer className="h-3.5 w-3.5" aria-hidden />
                  Stopwatch
                </>
              ) : (
                <>
                  <Hourglass className="h-3.5 w-3.5" aria-hidden />
                  Timer
                </>
              )}
            </div>

            <div className="flex w-full flex-col items-center gap-1">
              <time
                dateTime={`PT${isStopwatch ? snapshot.elapsedSecs : snapshot.remainingSecs}S`}
                aria-live="polite"
                aria-atomic="true"
                className={cn(
                  'text-4xl font-light tracking-tight text-neutral-900 tabular-nums transition-opacity duration-200 dark:text-neutral-50',
                  isPaused && 'opacity-60',
                )}
              >
                {snapshot.formatted}
              </time>

              <div
                className={cn(
                  'mb-4 flex w-full flex-col items-center gap-3 text-xs text-neutral-500 transition-opacity duration-200 dark:text-neutral-400',
                  isPaused && 'opacity-60',
                )}
                aria-live="polite"
              >
                {isStopwatch ? (
                  <span>No end time</span>
                ) : (
                  <>
                    <div className="mt-1 flex w-[80%] items-center gap-1.5">
                      <span className="w-20 text-right">{endsAt}</span>
                      <TimerProgress
                        remainingSecs={snapshot.remainingSecs}
                        durationSecs={snapshot.durationSecs}
                        running={isRunning}
                      />
                      <span className="w-20 text-left">
                        {secsToCompact(snapshot.durationSecs)}
                      </span>
                    </div>
                  </>
                )}
              </div>
            </div>

            <div className="flex w-full items-center justify-center gap-2">
              <Button
                variant="secondary"
                onClick={handleCancel}
                aria-label="Cancel timer"
                className="gap-1.5"
              >
                <X className="h-3.5 w-3.5" aria-hidden />
              </Button>
              <Button
                onClick={() => void togglePause()}
                aria-label={isRunning ? 'Pause' : 'Resume'}
                className="w-28 gap-1.5"
              >
                {isRunning ? (
                  <Pause className="h-3.5 w-3.5" aria-hidden />
                ) : (
                  <Play className="h-3.5 w-3.5" aria-hidden />
                )}
                {isRunning ? 'Pause' : 'Resume'}
              </Button>
            </div>
          </div>
        </CrossfadeSlot>
        <CrossfadeSlot
          frame={runFade.frames.idle}
          animate={runFade.animate}
          mounted={runFade.mounted.idle}
          className="absolute inset-0"
        >
          <div className="flex h-full w-full min-w-0 flex-col items-center justify-between gap-3 px-4 pt-1 pb-4">
            <ModeSwitch mode={snapshot.mode} onChange={handleModeChange} />

            <div className="grid w-full place-items-center overflow-hidden">
              <CrossfadeSlot
                frame={modeFade.frames.timer}
                animate={modeFade.animate}
                mounted={modeFade.mounted.timer}
                className="col-start-1 row-start-1"
              >
                <div className="flex flex-col gap-1.5">
                  <DurationInput
                    value={mask}
                    onChange={handleMaskChange}
                    onFocus={() => {
                      editingRef.current = true
                    }}
                    onBlur={() => {
                      editingRef.current = false
                      const normalized = secsToMask(maskToSecs(mask))
                      setMask(normalized)
                      void commitDuration(normalized)
                    }}
                    onCommit={() => void handleStart()}
                  />

                  {configuredPresets.length > 0 ? (
                    <div className="flex w-full justify-center gap-1.5">
                      <div className="flex min-w-0 gap-1.5">
                        {configuredPresets.map(({ index, secs }) => (
                          <button
                            key={index}
                            type="button"
                            onClick={() => applyPreset(secs)}
                            aria-label={`Use preset ${secsToCompact(secs)}`}
                            className="h-5 min-w-0 rounded px-2 text-xs text-neutral-600 transition-colors hover:bg-neutral-100 dark:text-neutral-300 dark:hover:bg-neutral-700/70"
                          >
                            {secsToCompact(secs)}
                          </button>
                        ))}
                      </div>
                    </div>
                  ) : null}
                </div>
              </CrossfadeSlot>
              <CrossfadeSlot
                frame={modeFade.frames.stopwatch}
                animate={modeFade.animate}
                mounted={modeFade.mounted.stopwatch}
                className="col-start-1 row-start-1"
              >
                <div className="mb-[35px] text-center text-4xl font-light tracking-tight text-neutral-900 tabular-nums opacity-60 dark:text-neutral-50">
                  00:00:00
                </div>
              </CrossfadeSlot>
            </div>

            <div className="flex w-full items-center justify-center gap-2">
              <Button
                variant="secondary"
                disabled
                aria-label="Cancel timer"
                className="gap-1.5"
              >
                <X className="h-3.5 w-3.5" aria-hidden />
              </Button>
              <Button
                onClick={() => void handleStart()}
                disabled={!canStart}
                aria-label="Start"
                className="w-28 gap-1.5"
              >
                <Play className="h-3.5 w-3.5" aria-hidden />
                Start
              </Button>
            </div>
          </div>
        </CrossfadeSlot>
      </main>
      {dialog !== 'closed' && snapshot ? (
        <RunningIntervalDialog
          mode={snapshot.mode}
          leaving={dialog === 'closing'}
          onClose={() => closeDialog(false)}
          onDiscard={() => void discard()}
          onSave={() => void reset()}
        />
      ) : null}
    </div>
  )
}

function isTextInput(target: EventTarget | null): boolean {
  return (
    target instanceof HTMLInputElement ||
    target instanceof HTMLTextAreaElement ||
    (target instanceof HTMLElement && target.isContentEditable)
  )
}
