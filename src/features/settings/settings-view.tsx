import { useEffect, useState } from 'react'

import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow'
import { openUrl } from '@tauri-apps/plugin-opener'

import {
  SettingsGroup,
  SettingsGroupContent,
  SettingsGroupItem,
  SettingsGroupItemControl,
  SettingsGroupItemLabel,
  SettingsGroupTitle,
} from '@/components/settings-group'
import { Button } from '@/components/ui/button'
import { Select } from '@/components/ui/select'
import { Switch } from '@/components/ui/switch'
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs'
import { WindowTitleBar } from '@/components/window-title-bar'
import { PresetDurationInput } from '@/features/settings/preset-duration-input'
import { ShortcutsSection } from '@/features/settings/shortcuts-section'
import { StatisticsSection } from '@/features/settings/statistics-section'
import {
  api,
  NO_COMPLETION_SOUND,
  onUpdateStatus,
  type Presets,
  type Settings,
  type UpdateStatus,
} from '@/lib/tauri'
import { useTimerStore } from '@/store/timer-store'
import appIcon from '../../../src-tauri/icons/128x128@2x.png'

const REPO_URL = 'https://github.com/rbika/focus-time'

const PRESET_LABELS = ['Preset 1', 'Preset 2', 'Preset 3'] as const

const SETTINGS_TABS = [
  ['general', 'General'],
  ['timer', 'Timer'],
  ['pomodoro', 'Pomodoro'],
  ['notifications', 'Notifications'],
  ['shortcuts', 'Shortcuts'],
  ['about', 'About'],
] as const

type SettingsTab = (typeof SETTINGS_TABS)[number][0]

const DEFAULT_SETTINGS_TAB = SETTINGS_TABS[0][0]

const tabPanelClassName =
  'flex min-h-0 flex-col gap-5 overflow-y-auto overscroll-none pb-8 px-5'

export function SettingsView() {
  const [tab, setTab] = useState<SettingsTab>(DEFAULT_SETTINGS_TAB)
  const settings = useTimerStore((s) => s.settings)
  const saveSettings = useTimerStore((s) => s.actions.saveSettings)
  const [sounds, setSounds] = useState<string[]>([])
  const [appName, setAppName] = useState('Focus Time')
  const [appVersion, setAppVersion] = useState('')
  const [updateStatus, setUpdateStatus] = useState<UpdateStatus>({
    kind: 'idle',
  })

  useEffect(() => {
    void api.getCompletionSounds().then(setSounds)
    void api.getAppName().then(setAppName)
    void api.getAppVersion().then(setAppVersion)
    void api.getUpdateStatus().then(setUpdateStatus)
  }, [])

  useEffect(() => {
    let unlisten: (() => void) | undefined
    void onUpdateStatus(setUpdateStatus).then((fn) => {
      unlisten = fn
    })
    return () => {
      unlisten?.()
    }
  }, [])

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.metaKey && event.key.toLowerCase() === 'q') {
        event.preventDefault()
        void api.quitApp()
      }
      if (event.metaKey && event.key === ',') {
        event.preventDefault()
      }
    }
    window.addEventListener('keydown', onKeyDown)
    return () => window.removeEventListener('keydown', onKeyDown)
  }, [])

  useEffect(() => {
    let unlisten: (() => void) | undefined
    void getCurrentWebviewWindow()
      .onCloseRequested(() => {
        setTab(DEFAULT_SETTINGS_TAB)
      })
      .then((fn) => {
        unlisten = fn
      })
    return () => {
      unlisten?.()
    }
  }, [])

  if (!settings) {
    return (
      <div className="flex h-full items-center justify-center text-sm text-neutral-500">
        Loading…
      </div>
    )
  }

  const update = <K extends keyof Settings>(key: K, value: Settings[K]) => {
    const current = useTimerStore.getState().settings
    if (!current) return
    void saveSettings({ ...current, [key]: value })
  }

  const updatePreset = (index: number, secs: number | null) => {
    const current = useTimerStore.getState().settings
    if (!current) return
    if (current.presets[index] === secs) return
    const presets = [...current.presets] as Presets
    presets[index] = secs
    void saveSettings({ ...current, presets })
  }

  const checking =
    updateStatus.kind === 'checking' || updateStatus.kind === 'downloading'

  return (
    <div className="flex h-full flex-col overflow-hidden">
      <WindowTitleBar title="Settings" />
      <Tabs
        value={tab}
        onValueChange={(value) => {
          if (isSettingsTab(value)) setTab(value)
        }}
        className="min-h-0 flex-1 gap-5 pt-4"
      >
        <TabsList className="mx-auto shrink-0 gap-2">
          {SETTINGS_TABS.map(([value, label]) => (
            <TabsTrigger
              key={value}
              value={value}
              className="flex-none text-xs"
            >
              {label}
            </TabsTrigger>
          ))}
        </TabsList>

        <TabsContent value="general" className={tabPanelClassName}>
          <SettingsGroup>
            <SettingsGroupTitle>Behavior</SettingsGroupTitle>
            <SettingsGroupContent>
              <SettingsGroupItem>
                <SettingsGroupItemLabel htmlFor="start-at-login">
                  Start at login
                </SettingsGroupItemLabel>
                <SettingsGroupItemControl>
                  <Switch
                    id="start-at-login"
                    checked={settings.startAtLogin}
                    onCheckedChange={(value) => update('startAtLogin', value)}
                    aria-label="Start at login"
                  />
                </SettingsGroupItemControl>
              </SettingsGroupItem>
              <SettingsGroupItem>
                <SettingsGroupItemLabel htmlFor="hide-on-start">
                  Hide window when timer starts
                </SettingsGroupItemLabel>
                <SettingsGroupItemControl>
                  <Switch
                    id="hide-on-start"
                    checked={settings.hideWindowOnStart}
                    onCheckedChange={(value) =>
                      update('hideWindowOnStart', value)
                    }
                    aria-label="Hide window when timer starts"
                  />
                </SettingsGroupItemControl>
              </SettingsGroupItem>
              <SettingsGroupItem>
                <SettingsGroupItemLabel htmlFor="pause-on-sleep">
                  Pause when Mac sleeps
                </SettingsGroupItemLabel>
                <SettingsGroupItemControl>
                  <Switch
                    id="pause-on-sleep"
                    checked={settings.pauseOnSleep}
                    onCheckedChange={(value) => update('pauseOnSleep', value)}
                    aria-label="Pause when Mac sleeps"
                  />
                </SettingsGroupItemControl>
              </SettingsGroupItem>
            </SettingsGroupContent>
          </SettingsGroup>

          <SettingsGroup>
            <SettingsGroupTitle>Appearance</SettingsGroupTitle>
            <SettingsGroupContent>
              <SettingsGroupItem>
                <SettingsGroupItemLabel htmlFor="icon-only">
                  Icon only in menu bar
                </SettingsGroupItemLabel>
                <SettingsGroupItemControl>
                  <Switch
                    id="icon-only"
                    checked={settings.iconOnly}
                    onCheckedChange={(value) => update('iconOnly', value)}
                    aria-label="Icon only in menu bar"
                  />
                </SettingsGroupItemControl>
              </SettingsGroupItem>
            </SettingsGroupContent>
          </SettingsGroup>

          <StatisticsSection
            weekStart={settings.weekStart}
            onWeekStartChange={(value) => update('weekStart', value)}
          />

          <SettingsGroup>
            <SettingsGroupTitle>Updates</SettingsGroupTitle>
            <SettingsGroupContent>
              <SettingsGroupItem>
                <SettingsGroupItemLabel htmlFor="auto-check-updates">
                  Automatically check for updates
                </SettingsGroupItemLabel>
                <SettingsGroupItemControl>
                  <Switch
                    id="auto-check-updates"
                    checked={settings.autoCheckForUpdates}
                    onCheckedChange={(value) =>
                      update('autoCheckForUpdates', value)
                    }
                    aria-label="Automatically check for updates"
                  />
                </SettingsGroupItemControl>
              </SettingsGroupItem>
              <SettingsGroupItem>
                <SettingsGroupItemLabel htmlFor="check-now">
                  Check for updates
                </SettingsGroupItemLabel>
                <SettingsGroupItemControl className="gap-2">
                  <Button
                    id="check-now"
                    variant="secondary"
                    disabled={checking}
                    onClick={() => {
                      void api
                        .checkForUpdates()
                        .then(setUpdateStatus)
                        .catch(() => {
                          // Status events already cover failures.
                        })
                    }}
                  >
                    Check now
                  </Button>
                </SettingsGroupItemControl>
              </SettingsGroupItem>
            </SettingsGroupContent>
          </SettingsGroup>
        </TabsContent>

        <TabsContent value="timer" keepMounted className={tabPanelClassName}>
          <SettingsGroup>
            <SettingsGroupTitle>Presets</SettingsGroupTitle>
            <SettingsGroupContent>
              {PRESET_LABELS.map((label, index) => (
                <SettingsGroupItem key={label}>
                  <SettingsGroupItemLabel htmlFor={`preset-${index}`}>
                    {label}
                  </SettingsGroupItemLabel>
                  <SettingsGroupItemControl>
                    <PresetDurationInput
                      id={`preset-${index}`}
                      label={label}
                      valueSecs={settings.presets[index]}
                      onCommit={(secs) => updatePreset(index, secs)}
                    />
                  </SettingsGroupItemControl>
                </SettingsGroupItem>
              ))}
            </SettingsGroupContent>
          </SettingsGroup>
        </TabsContent>

        <TabsContent value="pomodoro" keepMounted className={tabPanelClassName}>
          <SettingsGroup>
            <SettingsGroupTitle>Durations</SettingsGroupTitle>
            <SettingsGroupContent>
              <SettingsGroupItem>
                <SettingsGroupItemLabel htmlFor="session-length">
                  Session
                </SettingsGroupItemLabel>
                <SettingsGroupItemControl>
                  <PresetDurationInput
                    id="session-length"
                    label="Session"
                    valueSecs={settings.sessionLengthSecs}
                    onCommit={(secs) => update('sessionLengthSecs', secs ?? 0)}
                  />
                </SettingsGroupItemControl>
              </SettingsGroupItem>
              <SettingsGroupItem>
                <SettingsGroupItemLabel htmlFor="short-break-length">
                  Short break
                </SettingsGroupItemLabel>
                <SettingsGroupItemControl>
                  <PresetDurationInput
                    id="short-break-length"
                    label="Short break"
                    valueSecs={settings.shortBreakLengthSecs}
                    onCommit={(secs) =>
                      update('shortBreakLengthSecs', secs ?? 0)
                    }
                  />
                </SettingsGroupItemControl>
              </SettingsGroupItem>
              <SettingsGroupItem>
                <SettingsGroupItemLabel htmlFor="long-break-length">
                  Long break
                </SettingsGroupItemLabel>
                <SettingsGroupItemControl>
                  <PresetDurationInput
                    id="long-break-length"
                    label="Long break"
                    valueSecs={settings.longBreakLengthSecs}
                    onCommit={(secs) =>
                      update('longBreakLengthSecs', secs ?? 0)
                    }
                  />
                </SettingsGroupItemControl>
              </SettingsGroupItem>
              <SettingsGroupItem>
                <SettingsGroupItemLabel htmlFor="sessions-until-long-break">
                  Long break after
                </SettingsGroupItemLabel>
                <SettingsGroupItemControl>
                  <Select
                    id="sessions-until-long-break"
                    value={String(settings.sessionsUntilLongBreak)}
                    onChange={(event) =>
                      update(
                        'sessionsUntilLongBreak',
                        Number(event.target.value),
                      )
                    }
                    aria-label="Long break after"
                  >
                    {Array.from({ length: 9 }, (_, index) => index + 2).map(
                      (count) => (
                        <option key={count} value={count}>
                          {count} Sessions
                        </option>
                      ),
                    )}
                  </Select>
                </SettingsGroupItemControl>
              </SettingsGroupItem>
            </SettingsGroupContent>
          </SettingsGroup>

          <SettingsGroup>
            <SettingsGroupTitle>Auto-start</SettingsGroupTitle>
            <SettingsGroupContent>
              <SettingsGroupItem>
                <SettingsGroupItemLabel htmlFor="auto-start-sessions">
                  Start Sessions automatically
                </SettingsGroupItemLabel>
                <SettingsGroupItemControl>
                  <Switch
                    id="auto-start-sessions"
                    checked={settings.autoStartSessions}
                    onCheckedChange={(value) =>
                      update('autoStartSessions', value)
                    }
                    aria-label="Start Sessions automatically"
                  />
                </SettingsGroupItemControl>
              </SettingsGroupItem>
              <SettingsGroupItem>
                <SettingsGroupItemLabel htmlFor="auto-start-breaks">
                  Start Breaks automatically
                </SettingsGroupItemLabel>
                <SettingsGroupItemControl>
                  <Switch
                    id="auto-start-breaks"
                    checked={settings.autoStartBreaks}
                    onCheckedChange={(value) =>
                      update('autoStartBreaks', value)
                    }
                    aria-label="Start Breaks automatically"
                  />
                </SettingsGroupItemControl>
              </SettingsGroupItem>
            </SettingsGroupContent>
          </SettingsGroup>
        </TabsContent>

        <TabsContent value="notifications" className={tabPanelClassName}>
          <SettingsGroup>
            <SettingsGroupTitle>Notifications</SettingsGroupTitle>
            <SettingsGroupContent>
              <SettingsGroupItem>
                <SettingsGroupItemLabel htmlFor="notifications-enabled">
                  Allow notifications
                </SettingsGroupItemLabel>
                <SettingsGroupItemControl>
                  <Switch
                    id="notifications-enabled"
                    checked={settings.notificationsEnabled}
                    onCheckedChange={(value) =>
                      update('notificationsEnabled', value)
                    }
                    aria-label="Allow notifications"
                  />
                </SettingsGroupItemControl>
              </SettingsGroupItem>
              <SettingsGroupItem>
                <SettingsGroupItemLabel htmlFor="completion-sound">
                  Completion sound
                </SettingsGroupItemLabel>
                <SettingsGroupItemControl>
                  <Select
                    id="completion-sound"
                    value={settings.completionSound}
                    onChange={(e) => {
                      const name = e.target.value
                      update('completionSound', name)
                      if (name !== NO_COMPLETION_SOUND) {
                        void api.previewSound(name)
                      }
                    }}
                  >
                    <option value={NO_COMPLETION_SOUND}>None</option>
                    {sounds.map((name) => (
                      <option key={name} value={name}>
                        {name}
                      </option>
                    ))}
                  </Select>
                </SettingsGroupItemControl>
              </SettingsGroupItem>
            </SettingsGroupContent>
          </SettingsGroup>
        </TabsContent>

        <TabsContent value="shortcuts" className={tabPanelClassName}>
          <ShortcutsSection />
        </TabsContent>

        <TabsContent value="about" className="flex justify-center">
          <section className="mt-5 flex flex-col items-center text-center">
            <img src={appIcon} alt="" className="mb-3 size-16" aria-hidden />
            <p className="text-lg font-semibold text-neutral-900 dark:text-neutral-50">
              {appName}
            </p>
            <div className="mt-1 flex gap-2">
              <p className="text-xs text-neutral-400 dark:text-neutral-500">
                {appVersion ? `Version ${appVersion}` : '\u00a0'}
              </p>
            </div>

            <p className="mt-6 text-xs text-neutral-400 dark:text-neutral-500">
              Focus Time is free and open-source.
            </p>
            <p>
              <a
                href={REPO_URL}
                className="text-xs text-[#007aff] hover:underline dark:text-[#0a84ff]"
                onClick={(event) => {
                  event.preventDefault()
                  void openUrl(REPO_URL)
                }}
              >
                View on Github
              </a>
            </p>
          </section>
        </TabsContent>
      </Tabs>
    </div>
  )
}

function isSettingsTab(value: unknown): value is SettingsTab {
  return SETTINGS_TABS.some(([tab]) => tab === value)
}
