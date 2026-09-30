import type { PomodoroPhase } from '@/lib/tauri'

export function isBreakPhase(phase: PomodoroPhase | null | undefined): boolean {
  return phase === 'shortBreak' || phase === 'longBreak'
}

export function phaseLabel(phase: PomodoroPhase | null | undefined): string {
  if (phase === 'shortBreak') return 'Short break'
  if (phase === 'longBreak') return 'Long break'
  return 'Session'
}
