import { create } from 'zustand'

import {
  api,
  onSettingsChanged,
  onTimerTick,
  type Settings,
  type TimerMode,
  type TimerSnapshot,
} from '@/lib/tauri'

// Types
// -----

interface TimerState {
  snapshot: TimerSnapshot | null
  settings: Settings | null
  ready: boolean
}

interface TimerActions {
  actions: {
    hydrate: () => Promise<void>
    applySnapshot: (snapshot: TimerSnapshot) => void
    applySettings: (settings: Settings) => void
    start: () => Promise<void>
    togglePause: () => Promise<void>
    reset: () => Promise<void>
    discard: () => Promise<void>
    skip: () => Promise<void>
    setDuration: (durationSecs: number) => Promise<void>
    setMode: (mode: TimerMode) => Promise<void>
    saveSettings: (settings: Settings) => Promise<void>
  }
}

interface TimerStore extends TimerState, TimerActions {}

// Store
// -----

const defaultState: TimerState = {
  snapshot: null,
  settings: null,
  ready: false,
}

export const useTimerStore = create<TimerStore>()((set) => {
  let saving = false
  let queued: Settings | null = null

  const persistSettings = async () => {
    if (saving) return
    saving = true
    try {
      while (queued) {
        const next = queued
        queued = null
        const saved = await api.updateSettings(next)
        if (!queued) {
          set({ settings: saved })
        }
      }
    } finally {
      saving = false
    }
  }

  return {
    ...defaultState,
    actions: {
      hydrate: async () => {
        const [snapshot, settings] = await Promise.all([
          api.getSnapshot(),
          api.getSettings(),
        ])
        set({ snapshot, settings, ready: true })
      },

      applySnapshot: (snapshot) => set({ snapshot }),
      applySettings: (settings) => {
        if (saving || queued) return
        set({ settings })
      },

      start: async () => {
        const snapshot = await api.start()
        set({ snapshot })
      },

      togglePause: async () => {
        const snapshot = await api.togglePause()
        set({ snapshot })
      },

      reset: async () => {
        const snapshot = await api.reset()
        set({ snapshot })
      },

      discard: async () => {
        const snapshot = await api.discard()
        set({ snapshot })
      },

      skip: async () => {
        const snapshot = await api.skip()
        set({ snapshot })
      },

      setDuration: async (durationSecs) => {
        const snapshot = await api.setDuration(durationSecs)
        set({ snapshot })
      },

      setMode: async (mode) => {
        const snapshot = await api.setMode(mode)
        set({ snapshot })
      },

      saveSettings: async (settings) => {
        set({ settings })
        queued = settings
        await persistSettings()
      },
    },
  }
})

// Listeners
// ---------

let subscribed = false

export async function subscribeToBackend() {
  if (subscribed) return
  subscribed = true

  await useTimerStore.getState().actions.hydrate()

  await onTimerTick((snapshot) => {
    useTimerStore.getState().actions.applySnapshot(snapshot)
  })

  await onSettingsChanged((settings) => {
    useTimerStore.getState().actions.applySettings(settings)
  })
}
