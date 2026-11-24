import { useCallback, useEffect, useRef, useState } from 'react'

import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow'
import { SquarePenIcon } from 'lucide-react'

import { Select } from '@/components/ui/select'
import { WindowTitleBar } from '@/components/window-title-bar'
import { crossfadeStyle } from '@/features/crossfade/crossfade'
import { EntriesList } from '@/features/entries/entries-list'
import { EntryEditor } from '@/features/entries/entry-editor'
import {
  DEFAULT_PERIOD,
  isPeriod,
  PERIOD_OPTIONS,
  type Period,
} from '@/features/entries/period'
import { useEditorTransition } from '@/features/entries/use-editor-transition'
import { api, onEntriesChanged, type Entry } from '@/lib/tauri'

const MANUAL_DRAFT_DURATION_SECS = 60
const PANEL_CLASS = 'absolute inset-0 flex min-h-0 flex-col px-4 pt-1 pb-4'

function draftManualEntry(): Entry {
  const endedAtUnix = Math.floor(Date.now() / 1000)
  const startedAtUnix = endedAtUnix - MANUAL_DRAFT_DURATION_SECS
  return {
    id: '',
    mode: 'manual',
    startedAtUnix,
    endedAtUnix,
    durationSecs: MANUAL_DRAFT_DURATION_SECS,
  }
}

export function EntriesWindow() {
  const [period, setPeriod] = useState<Period>(DEFAULT_PERIOD)
  const [yearRevealSteps, setYearRevealSteps] = useState(0)
  const [editing, setEditing] = useState<Entry | null>(null)
  const [creating, setCreating] = useState(false)
  const { frames, animate, destination, show, hide, snapToList } =
    useEditorTransition()

  const dismissEditor = useCallback(() => {
    setEditing(null)
    setCreating(false)
  }, [])

  const openEditor = (entry: Entry) => {
    setCreating(false)
    setEditing(entry)
    show()
  }

  const openCreate = () => {
    setCreating(true)
    setEditing(draftManualEntry())
    show()
  }

  const closeEditor = useCallback(() => {
    if (destination === 'list') return
    hide(dismissEditor)
  }, [destination, dismissEditor, hide])

  const creatingRef = useRef(creating)
  creatingRef.current = creating
  const editingRef = useRef(editing)
  editingRef.current = editing

  useEffect(() => {
    let unlisten: (() => void) | undefined
    void onEntriesChanged(() => {
      if (creatingRef.current) return
      const current = editingRef.current
      if (!current) return
      void api.getEntries().then((entries) => {
        if (creatingRef.current) return
        if (editingRef.current?.id !== current.id) return
        if (entries.some((entry) => entry.id === current.id)) return
        closeEditor()
      })
    }).then((fn) => {
      unlisten = fn
    })
    return () => {
      unlisten?.()
    }
  }, [closeEditor])

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
      }
    }
    window.addEventListener('keydown', onKeyDown)
    return () => window.removeEventListener('keydown', onKeyDown)
  }, [])

  useEffect(() => {
    let unlisten: (() => void) | undefined
    void getCurrentWebviewWindow()
      .onCloseRequested(() => {
        snapToList()
        dismissEditor()
      })
      .then((fn) => {
        unlisten = fn
      })
    return () => {
      unlisten?.()
    }
  }, [dismissEditor, snapToList])

  const editorActive = destination === 'editor' || frames.editor.interactive

  return (
    <div className="flex h-full flex-col overflow-hidden">
      <WindowTitleBar title="Entries" />
      <div className="relative min-h-0 flex-1">
        <div
          className={PANEL_CLASS}
          style={crossfadeStyle(frames.list, animate)}
          inert={!frames.list.interactive ? true : undefined}
        >
          <div className="flex min-h-0 flex-1 flex-col gap-1">
            <div className="flex shrink-0 items-center justify-between gap-2">
              <button
                type="button"
                onClick={openCreate}
                className="flex shrink-0 items-center gap-1 rounded-sm text-[13px] text-neutral-400 transition-colors hover:text-neutral-600 focus-visible:outline focus-visible:outline-offset-2 focus-visible:outline-blue-500 dark:text-neutral-500 dark:hover:text-neutral-300"
              >
                <SquarePenIcon className="h-3.5 w-3.5 shrink-0" /> Add entry
              </button>
              <Select
                aria-label="Period"
                value={period}
                onChange={(event) => {
                  const value = event.target.value
                  if (!isPeriod(value)) return
                  setPeriod(value)
                  setYearRevealSteps(0)
                }}
                className="h-6 w-auto py-0 text-xs"
              >
                {PERIOD_OPTIONS.map((option) => (
                  <option key={option.value} value={option.value}>
                    {option.label}
                  </option>
                ))}
              </Select>
            </div>
            <EntriesList
              period={period}
              yearRevealSteps={yearRevealSteps}
              onShowMore={() => setYearRevealSteps((steps) => steps + 1)}
              onOpenEntry={openEditor}
            />
          </div>
        </div>
        <div
          className={PANEL_CLASS}
          style={crossfadeStyle(frames.editor, animate)}
          inert={!frames.editor.interactive ? true : undefined}
        >
          {editing ? (
            <EntryEditor
              entry={editing}
              creating={creating}
              active={editorActive}
              onClose={closeEditor}
            />
          ) : null}
        </div>
      </div>
    </div>
  )
}
