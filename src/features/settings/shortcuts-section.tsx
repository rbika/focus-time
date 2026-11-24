import {
  SettingsGroup,
  SettingsGroupContent,
  SettingsGroupItem,
  SettingsGroupItemControl,
  SettingsGroupItemDescription,
  SettingsGroupItemLabel,
  SettingsGroupItemText,
  SettingsGroupTitle,
} from '@/components/settings-group'

const SHORTCUTS = [
  { label: 'Start, or pause / resume', keys: '⌘S' },
  { label: 'Cancel the timer', keys: '⌘X' },
  { label: 'Hide the window', keys: 'Escape' },
  {
    label: 'Switch Timer / Stopwatch / Pomodoro',
    description: 'While idle or completed, and not Waiting.',
    keys: '⌘1',
  },
  { label: 'Open Settings', keys: '⌘,' },
] as const

export function ShortcutsSection() {
  return (
    <SettingsGroup>
      <SettingsGroupTitle>Keyboard Shortcuts</SettingsGroupTitle>
      <SettingsGroupContent>
        {SHORTCUTS.map((shortcut) => (
          <SettingsGroupItem key={shortcut.keys}>
            {'description' in shortcut ? (
              <SettingsGroupItemText>
                <SettingsGroupItemLabel>
                  {shortcut.label}
                </SettingsGroupItemLabel>
                <SettingsGroupItemDescription>
                  {shortcut.description}
                </SettingsGroupItemDescription>
              </SettingsGroupItemText>
            ) : (
              <SettingsGroupItemLabel>{shortcut.label}</SettingsGroupItemLabel>
            )}
            <SettingsGroupItemControl>
              <span className="text-[13px] text-neutral-800 dark:text-neutral-100">
                {shortcut.keys}
              </span>
            </SettingsGroupItemControl>
          </SettingsGroupItem>
        ))}
      </SettingsGroupContent>
    </SettingsGroup>
  )
}
