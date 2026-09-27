type Props = {
  remainingSecs: number
  durationSecs: number
  running: boolean
}

/**
 * The engine reports whole seconds (`60` stays `60` until it has fully
 * elapsed), so anything driven off the tick alone trails real time by a
 * second. Instead each snapshot re-anchors a CSS animation that drains the
 * bar over the seconds it still has, so it moves the instant the timer
 * starts, keeps running between ticks, and reaches empty at the deadline.
 */
export function TimerProgress({ remainingSecs, durationSecs, running }: Props) {
  const percent =
    durationSecs > 0
      ? Math.min(100, Math.max(0, (remainingSecs / durationSecs) * 100))
      : 0
  const draining = running && remainingSecs > 0 && durationSecs > 0

  return (
    <div
      role="progressbar"
      aria-label="Time remaining"
      aria-valuemin={0}
      aria-valuemax={100}
      aria-valuenow={Math.round(percent)}
      className="h-1 w-full overflow-hidden rounded-full bg-neutral-300 dark:bg-neutral-700"
    >
      <div
        // Remounting restarts the animation from the fresh snapshot.
        key={draining ? `${durationSecs}:${remainingSecs}` : 'idle'}
        className="h-full origin-left rounded-full bg-neutral-900 dark:bg-neutral-50"
        style={{
          width: `${percent}%`,
          animation: draining
            ? `timer-drain ${remainingSecs}s linear forwards`
            : undefined,
        }}
      />
    </div>
  )
}
