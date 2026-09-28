import { useState } from 'react'

import {
  SettingsGroup,
  SettingsGroupContent,
  SettingsGroupItem,
  SettingsGroupItemControl,
  SettingsGroupItemLabel,
  SettingsGroupTitle,
} from '@/components/settings-group'
import { Button } from '@/components/ui/button'
import { useDialogOverlay } from '@/features/dialog/use-dialog-overlay'
import { ResetStatisticsDialog } from '@/features/settings/reset-statistics-dialog'
import { api } from '@/lib/tauri'

export function StatisticsSection() {
  const {
    frame: dialogFrame,
    show: showDialog,
    dismiss: dismissDialog,
  } = useDialogOverlay()
  const [persistError, setPersistError] = useState(false)

  const confirmAndReset = async () => {
    try {
      await api.resetStatistics()
    } catch {
      setPersistError(true)
    }
  }

  return (
    <>
      <SettingsGroup>
        <SettingsGroupTitle>Statistics</SettingsGroupTitle>
        <SettingsGroupContent>
          <SettingsGroupItem>
            <SettingsGroupItemLabel htmlFor="reset-statistics">
              Reset statistics
            </SettingsGroupItemLabel>
            <SettingsGroupItemControl>
              <Button
                id="reset-statistics"
                variant="secondary"
                onClick={() => {
                  setPersistError(false)
                  showDialog()
                }}
              >
                Reset
              </Button>
            </SettingsGroupItemControl>
          </SettingsGroupItem>
        </SettingsGroupContent>
        {persistError ? (
          <p className="px-1 text-xs text-red-600 dark:text-red-400">
            Couldn’t reset statistics
          </p>
        ) : null}
      </SettingsGroup>
      <ResetStatisticsDialog
        frame={dialogFrame}
        onCancel={() => dismissDialog()}
        onConfirm={() => {
          dismissDialog()
          void confirmAndReset()
        }}
      />
    </>
  )
}
