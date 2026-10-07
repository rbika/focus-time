import { useCallback, useEffect, useRef, useState } from 'react'

import {
  BellIcon,
  CircleIcon,
  Hourglass,
  Pause,
  Play,
  Timer,
  X,
} from 'lucide-react'

import { Button } from '@/components/ui/button'
import { CROSSFADE_NUDGE_PX } from '@/features/crossfade/crossfade'
import { CrossfadeSlot } from '@/features/crossfade/crossfade-slot'
import {
  useCrossfade,
  useOrderedCrossfade,
} from '@/features/crossfade/use-crossfade'
import { useDialogOverlay } from '@/features/dialog/use-dialog-overlay'
import { DurationInput } from '@/features/timer/duration-input'
import { MODES, ModeSwitch } from '@/features/timer/mode-switch'
import { phaseLabel } from '@/features/timer/pomodoro'
import { RunningIntervalDialog } from '@/features/timer/running-interval-dialog'
import { SessionDots } from '@/features/timer/session-dots'
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
  const skip = useTimerStore((s) => s.actions.skip)
  const setDuration = useTimerStore((s) => s.actions.setDuration)
  const setMode = useTimerStore((s) => s.actions.setMode)
  const modeFade = useOrderedCrossfade(
    MODES,
    snapshot?.mode ?? 'timer',
    CROSSFADE_NUDGE_PX,
    snapshot != null,
  )
  const runFade = useCrossfade(
    'idle',
    'running',
    snapshot?.status === 'running' ||
      snapshot?.status === 'paused' ||
      snapshot?.waiting
      ? 'running'
      : 'idle',
    0,
    snapshot != null,
  )

  const [mask, setMask] = useState('00:00:00')
  const {
    frame: dialogFrame,
    show: showDialog,
    dismiss: dismissDialog,
    snap: snapDialog,
  } = useDialogOverlay()
  const editingRef = useRef(false)
  const debounceRef = useRef<ReturnType<typeof setTimeout> | null>(null)
  const genRef = useRef(0)
  const intendedSecsRef = useRef<number | null>(null)
  const queueRef = useRef(Promise.resolve())
  // Last Pomodoro readout, so the outgoing panel doesn't jump to the new mode.
  const pomodoroIdleLabelRef = useRef('')

  const runExclusive = useCallback((task: () => Promise<void>) => {
    const run = queueRef.current.then(task, task)
    queueRef.current = run.then(
      () => undefined,
      () => undefined,
    )
    return run
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
    if (current?.mode === 'stopwatch' || current?.mode === 'pomodoro') {
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
    if (current?.isBreak) {
      void discard()
      return
    }
    if (current?.status === 'running' && current.intervalElapsedSecs > 10) {
      showDialog()
      return
    }
    void reset()
  }, [showDialog, reset, discard])

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
        if (dialogFrame.mounted || !snapshot) return
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
        if (dialogFrame.mounted) {
          event.preventDefault()
          return
        }
        const current = useTimerStore.getState().snapshot
        const isInFlight =
          current?.status === 'running' || current?.status === 'paused'
        if (!isInFlight && !current?.waiting) return
        event.preventDefault()
        handleCancel()
        return
      }
      if (event.key === 'Escape') {
        event.preventDefault()
        if (dialogFrame.mounted) {
          dismissDialog()
          return
        }
        void api.hideTimerWindow()
        return
      }
    }
    window.addEventListener('keydown', onKeyDown)
    return () => window.removeEventListener('keydown', onKeyDown)
  }, [
    snapshot,
    togglePause,
    handleStart,
    handleCancel,
    dialogFrame.mounted,
    dismissDialog,
  ])

  useEffect(() => {
    if (!active) snapDialog()
  }, [active, snapDialog])

  useEffect(() => {
    if (snapshot?.status !== 'running') dismissDialog()
  }, [snapshot?.status, dismissDialog])

  useEffect(() => {
    let unlisten: (() => void) | undefined
    void onMainWindowHidden(() => snapDialog()).then((fn) => {
      unlisten = fn
    })
    return () => {
      unlisten?.()
    }
  }, [snapDialog])

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
  const isPomodoro = snapshot.mode === 'pomodoro'
  if (isPomodoro) pomodoroIdleLabelRef.current = snapshot.formatted
  const isCountdown = !isStopwatch
  const onBreak = snapshot.isBreak
  const canStart =
    isStopwatch ||
    (isPomodoro ? snapshot.durationSecs > 0 : maskToSecs(mask) > 0)
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
              aria-label={
                isPomodoro
                  ? 'Pomodoro mode'
                  : isStopwatch
                    ? 'Stopwatch mode'
                    : 'Timer mode'
              }
            >
              {isStopwatch ? (
                <>
                  <Timer className="h-4 w-4" aria-hidden />
                  Stopwatch
                </>
              ) : isPomodoro ? (
                <div className="flex items-center gap-1.5">
                  <CircleIcon className="h-3.5 w-3.5" aria-hidden />
                  {phaseLabel(snapshot.phase)}:{' '}
                  {secsToCompact(snapshot.durationSecs)}
                </div>
              ) : (
                <div className="flex w-full items-center justify-center gap-4 px-5">
                  <div className="flex items-center gap-1.5">
                    <Hourglass className="h-3.5 w-3.5" aria-hidden />
                    Timer: {secsToCompact(snapshot.durationSecs)}
                  </div>
                </div>
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
                  'mb-2 flex w-full flex-col items-center gap-3 text-xs text-neutral-500 dark:text-neutral-400',
                  isPaused && 'opacity-60',
                )}
                aria-live="polite"
              >
                {isCountdown ? (
                  <div className="mt-2 flex w-44 flex-col items-center gap-1.5">
                    <TimerProgress
                      remainingSecs={snapshot.remainingSecs}
                      durationSecs={snapshot.durationSecs}
                      running={isRunning}
                    />
                    <div className="flex w-full items-center justify-center gap-2">
                      <span
                        key={isRunning ? 'ends-at' : 'ends-at-none'}
                        className="flex w-20 items-center justify-center gap-1 text-left"
                      >
                        <BellIcon className="mt-px h-3 w-3" aria-hidden />{' '}
                        {endsAt}
                      </span>
                      {isPomodoro ? (
                        <SessionDots
                          completed={snapshot.completedSessions}
                          total={snapshot.sessionsUntilLongBreak}
                        />
                      ) : null}
                    </div>
                  </div>
                ) : null}
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
              {onBreak ? (
                <Button
                  variant="secondary"
                  onClick={() => void skip()}
                  aria-label="Skip break"
                  className="gap-1.5"
                >
                  Skip
                </Button>
              ) : null}
              <Button
                onClick={() => void togglePause()}
                aria-label={isRunning ? 'Pause' : isPaused ? 'Resume' : 'Start'}
                className="w-28 gap-1.5"
              >
                {isRunning ? (
                  <Pause className="h-3.5 w-3.5" aria-hidden />
                ) : (
                  <Play className="h-3.5 w-3.5" aria-hidden />
                )}
                {isRunning ? 'Pause' : isPaused ? 'Resume' : 'Start'}
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

            <div className="grid w-full place-items-center overflow-hidden py-2">
              <CrossfadeSlot
                frame={modeFade.frames.timer}
                animate={modeFade.animate}
                mounted={modeFade.mounted.timer}
                className="col-start-1 row-start-1"
              >
                <div className="flex flex-col items-center gap-1.5">
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
                      <div className="flex min-w-0 justify-center gap-1.5">
                        {configuredPresets.map(({ index, secs }) => (
                          <button
                            key={index}
                            type="button"
                            onClick={() => applyPreset(secs)}
                            aria-label={`Use preset ${secsToCompact(secs)}`}
                            className="h-6 min-w-0 rounded px-2 text-[13px] text-neutral-600 transition-colors hover:bg-neutral-100 dark:text-neutral-300 dark:hover:bg-neutral-700/70"
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
                <div className="mb-4 text-center text-4xl font-light tracking-tight text-neutral-900 tabular-nums opacity-60 dark:text-neutral-50">
                  00:00:00
                </div>
              </CrossfadeSlot>
              <CrossfadeSlot
                frame={modeFade.frames.pomodoro}
                animate={modeFade.animate}
                mounted={modeFade.mounted.pomodoro}
                className="col-start-1 row-start-1"
              >
                <div className="flex flex-col items-center gap-1.5">
                  <p className="mb-4 text-center text-4xl font-light tracking-tight text-neutral-900 tabular-nums opacity-60 dark:text-neutral-50">
                    {pomodoroIdleLabelRef.current}
                  </p>
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
      <RunningIntervalDialog
        mode={snapshot.mode}
        frame={dialogFrame}
        onClose={() => dismissDialog()}
        onDiscard={() => {
          dismissDialog()
          void discard()
        }}
        onSave={() => {
          dismissDialog()
          void reset()
        }}
      />
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
