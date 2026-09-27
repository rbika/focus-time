import { useEffect, useRef, useState } from 'react'

import { ChevronLeft } from 'lucide-react'

import {
  SettingsGroup,
  SettingsGroupContent,
  SettingsGroupItem,
  SettingsGroupItemControl,
  SettingsGroupItemLabel,
} from '@/components/settings-group'
import { Button } from '@/components/ui/button'
import { useDialogOverlay } from '@/features/dialog/use-dialog-overlay'
import {
  fromDatetimeLocalValue,
  toDatetimeLocalValue,
} from '@/features/stats/datetime-local'
import { DeleteEntryDialog } from '@/features/stats/delete-entry-dialog'
import { api, onMainWindowHidden, type Entry } from '@/lib/tauri'
import { cn } from '@/utils/cn'
import { secsToTotalLabel } from '@/utils/time'

type Props = {
  entry: Entry
  creating: boolean
  active: boolean
  onClose: () => void
}

export function EntryEditor({ entry, creating, active, onClose }: Props) {
  const [started, setStarted] = useState(() =>
    toDatetimeLocalValue(entry.startedAtUnix),
  )
  const [ended, setEnded] = useState(() =>
    toDatetimeLocalValue(entry.endedAtUnix),
  )
  const {
    frame: dialogFrame,
    show: showDialog,
    dismiss: dismissDialog,
    snap: snapDialog,
  } = useDialogOverlay()
  const wasActive = useRef(active)
  const [persistError, setPersistError] = useState<'save' | 'delete' | null>(
    null,
  )
  const [saving, setSaving] = useState(false)

  useEffect(() => {
    if (active && !wasActive.current) snapDialog()
    wasActive.current = active
  }, [active, snapDialog])

  useEffect(() => {
    if (!active) return
    setStarted(toDatetimeLocalValue(entry.startedAtUnix))
    setEnded(toDatetimeLocalValue(entry.endedAtUnix))
    setPersistError(null)
    // Reset only when the editor is opened, not when the stored Entry
    // identity changes behind the local draft.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [active])

  useEffect(() => {
    if (!active) return
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key !== 'Escape') return
      event.preventDefault()
      if (dialogFrame.mounted) {
        dismissDialog()
        return
      }
      onClose()
    }
    window.addEventListener('keydown', onKeyDown)
    return () => window.removeEventListener('keydown', onKeyDown)
  }, [active, dialogFrame.mounted, dismissDialog, onClose])

  useEffect(() => {
    let unlisten: (() => void) | undefined
    void onMainWindowHidden(() => snapDialog()).then((fn) => {
      unlisten = fn
    })
    return () => {
      unlisten?.()
    }
  }, [snapDialog])

  const startedUnix = fromDatetimeLocalValue(started)
  const endedUnix = fromDatetimeLocalValue(ended)
  const rangeValid =
    startedUnix != null && endedUnix != null && endedUnix > startedUnix
  const durationSecs = rangeValid ? endedUnix - startedUnix : 0
  const dirty =
    startedUnix !== entry.startedAtUnix || endedUnix !== entry.endedAtUnix
  const canSave = rangeValid && (creating || dirty) && !saving
  const datetimeLocalClassName =
    'h-7 max-w-[168px] min-w-0 rounded-md border border-neutral-300 bg-white px-1.5 text-right text-[12px] text-neutral-800 tabular-nums outline-none dark:border-neutral-600 dark:bg-neutral-700 dark:text-neutral-100'

  const footerMessage = persistError
    ? persistError === 'save'
      ? 'Couldn’t save'
      : 'Couldn’t delete'
    : rangeValid
      ? null
      : 'Invalid range'

  const onDraftChange = (field: 'started' | 'ended', value: string) => {
    setPersistError(null)
    if (field === 'started') setStarted(value)
    else setEnded(value)
  }

  const save = async () => {
    if (!canSave || startedUnix == null || endedUnix == null) return
    setSaving(true)
    try {
      if (creating) {
        await api.createEntry(startedUnix, endedUnix)
      } else {
        await api.updateEntry(entry.id, startedUnix, endedUnix)
      }
      onClose()
    } catch {
      setPersistError('save')
    } finally {
      setSaving(false)
    }
  }

  const confirmAndDelete = async () => {
    try {
      await api.deleteEntry(entry.id)
      onClose()
    } catch {
      setPersistError('delete')
    }
  }

  return (
    <div
      className="flex h-full min-h-0 flex-col"
      inert={dialogFrame.mounted || !active ? true : undefined}
    >
      <div className="shrink-0">
        <button
          type="button"
          onClick={onClose}
          className="mb-2 flex shrink-0 items-center gap-1 self-start rounded-sm text-[13px] text-neutral-400 transition-colors hover:text-neutral-600 focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-2 dark:text-neutral-500 dark:hover:text-neutral-300"
        >
          <ChevronLeft className="h-3.5 w-3.5 shrink-0" aria-hidden />
          Back
        </button>
      </div>

      <div className="flex min-h-0 flex-1 flex-col gap-3 pt-1">
        <SettingsGroup>
          <SettingsGroupContent>
            <SettingsGroupItem className="flex-wrap">
              <SettingsGroupItemLabel htmlFor="entry-started">
                Started
              </SettingsGroupItemLabel>
              <SettingsGroupItemControl>
                <input
                  id="entry-started"
                  type="datetime-local"
                  step={1}
                  value={started}
                  onChange={(event) =>
                    onDraftChange('started', event.target.value)
                  }
                  className={datetimeLocalClassName}
                />
              </SettingsGroupItemControl>
            </SettingsGroupItem>
            <SettingsGroupItem className="flex-wrap">
              <SettingsGroupItemLabel htmlFor="entry-ended">
                Ended
              </SettingsGroupItemLabel>
              <SettingsGroupItemControl>
                <input
                  id="entry-ended"
                  type="datetime-local"
                  step={1}
                  value={ended}
                  onChange={(event) =>
                    onDraftChange('ended', event.target.value)
                  }
                  className={datetimeLocalClassName}
                />
              </SettingsGroupItemControl>
            </SettingsGroupItem>
            <SettingsGroupItem>
              <SettingsGroupItemLabel>Duration</SettingsGroupItemLabel>
              <SettingsGroupItemControl>
                <span className="text-[13px] font-medium text-neutral-900 tabular-nums dark:text-neutral-50">
                  {secsToTotalLabel(durationSecs)}
                </span>
              </SettingsGroupItemControl>
            </SettingsGroupItem>
          </SettingsGroupContent>
        </SettingsGroup>{' '}
        {footerMessage ? (
          <p className="mb-2 text-center text-xs text-red-600 dark:text-red-400">
            {footerMessage}
          </p>
        ) : null}
      </div>

      <div className="shrink-0 pt-2">
        <div
          className={cn(
            'flex items-center gap-2',
            creating ? 'justify-end' : 'justify-between',
          )}
        >
          {creating ? null : (
            <Button
              type="button"
              variant="destructive"
              onClick={() => showDialog()}
            >
              Delete
            </Button>
          )}
          <div className="flex items-center gap-2">
            <Button type="button" variant="secondary" onClick={onClose}>
              Cancel
            </Button>
            <Button
              type="button"
              disabled={!canSave}
              onClick={() => void save()}
            >
              Save
            </Button>
          </div>
        </div>
      </div>

      <DeleteEntryDialog
        frame={dialogFrame}
        onCancel={() => dismissDialog()}
        onConfirm={() => {
          dismissDialog()
          void confirmAndDelete()
        }}
      />
    </div>
  )
}
