import { Tabs, TabsList, TabsTrigger } from '@/components/ui/tabs'
import type { TimerMode } from '@/lib/tauri'

type Props = {
  mode: TimerMode
  onChange: (mode: TimerMode) => void
}

export const MODES: readonly TimerMode[] = ['timer', 'stopwatch', 'pomodoro']

export function ModeSwitch({ mode, onChange }: Props) {
  return (
    <Tabs
      value={mode}
      onValueChange={(value) => {
        if (value === mode) return
        if (MODES.includes(value as TimerMode)) onChange(value as TimerMode)
      }}
      className="w-full shrink-0 gap-0"
    >
      <TabsList className="mx-auto shrink-0">
        <TabsTrigger value="timer" className="w-[4.75rem] text-xs">
          Timer
        </TabsTrigger>
        <TabsTrigger value="stopwatch" className="w-[4.75rem] text-xs">
          Stopwatch
        </TabsTrigger>
        <TabsTrigger value="pomodoro" className="w-[4.75rem] text-xs">
          Pomodoro
        </TabsTrigger>
      </TabsList>
    </Tabs>
  )
}
