import { useEffect, useState } from 'react'

import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow'

import { EntriesWindow } from '@/features/entries/entries-window'
import { SettingsView } from '@/features/settings/settings-view'
import { TimerWindow } from '@/features/timer/timer-window'
import { UpToDateView } from '@/features/updates/up-to-date-view'
import { UpdateAvailableView } from '@/features/updates/update-available-view'
import { UpdateProgressView } from '@/features/updates/update-progress-view'
import { subscribeToBackend } from '@/store/timer-store'

export default function App() {
  const [label, setLabel] = useState<string | null>(null)

  useEffect(() => {
    const windowLabel = getCurrentWebviewWindow().label
    setLabel(windowLabel)
    void subscribeToBackend()
  }, [])

  if (!label) {
    return null
  }

  if (label === 'settings') {
    return (
      <div className="h-full bg-[canvas]">
        <SettingsView />
      </div>
    )
  }

  if (label === 'entries') {
    return (
      <div className="h-full bg-[canvas]">
        <EntriesWindow />
      </div>
    )
  }

  if (label === 'up-to-date') {
    return <UpToDateView />
  }

  if (label === 'update-available') {
    return (
      <div className="h-full bg-[canvas]">
        <UpdateAvailableView />
      </div>
    )
  }

  if (label === 'update-progress') {
    return (
      <div className="h-full bg-[canvas]">
        <UpdateProgressView />
      </div>
    )
  }

  return (
    <div className="h-full bg-[canvas]">
      <TimerWindow />
    </div>
  )
}
