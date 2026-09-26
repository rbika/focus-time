import { useCallback, useEffect, useState } from 'react'

import { SquarePenIcon } from 'lucide-react'

import { Tabs, TabsList, TabsTrigger } from '@/components/ui/tabs'
import {
  CROSSFADE_NUDGE_PX,
  crossfadeStyle,
} from '@/features/crossfade/crossfade'
import { CrossfadeSlot } from '@/features/crossfade/crossfade-slot'
import { useCrossfade } from '@/features/crossfade/use-crossfade'
import { DashboardTab } from '@/features/stats/dashboard-tab'
import { EntriesTab } from '@/features/stats/entries-tab'
import { EntryEditor } from '@/features/stats/entry-editor'
import { useEditorTransition } from '@/features/stats/use-editor-transition'
import type { Entry } from '@/lib/tauri'

const MANUAL_DRAFT_DURATION_SECS = 60
const PANEL_CLASS = 'absolute inset-0 flex min-h-0 flex-col px-4 pt-1 pb-4'

type StatsTab = 'dashboard' | 'entries'

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

export function StatsView({ active }: { active: boolean }) {
  const [tab, setTab] = useState<StatsTab>('dashboard')
  const [editing, setEditing] = useState<Entry | null>(null)
  const [creating, setCreating] = useState(false)
  const { frames, animate, destination, show, hide, snapToList } =
    useEditorTransition()
  const tabs = useCrossfade('dashboard', 'entries', tab, CROSSFADE_NUDGE_PX)

  const dismissEditor = useCallback(() => {
    setEditing(null)
    setCreating(false)
  }, [])

  useEffect(() => {
    if (active) return
    snapToList()
    dismissEditor()
  }, [active, dismissEditor, snapToList])

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

  useEffect(() => {
    if (!active) return
    const onKeyDown = (event: KeyboardEvent) => {
      if (!event.metaKey || event.ctrlKey || event.altKey || event.shiftKey) {
        return
      }
      if (event.key !== '2') return
      event.preventDefault()
      if (destination === 'editor') return
      setTab((current) => (current === 'dashboard' ? 'entries' : 'dashboard'))
    }
    window.addEventListener('keydown', onKeyDown)
    return () => window.removeEventListener('keydown', onKeyDown)
  }, [active, destination])

  const editorActive = destination === 'editor' || frames.editor.interactive

  return (
    <div className="relative flex h-full min-h-0 flex-col overflow-hidden">
      <div
        className={PANEL_CLASS}
        style={crossfadeStyle(frames.list, animate)}
        inert={!frames.list.interactive ? true : undefined}
      >
        <main className="flex min-h-0 flex-1 flex-col">
          <Tabs
            value={tab}
            onValueChange={(value) => {
              if (value === 'dashboard' || value === 'entries') setTab(value)
            }}
            className="min-h-0 flex-1 flex-col gap-3"
          >
            <TabsList className="mx-auto shrink-0">
              <TabsTrigger value="dashboard" className="w-24 text-xs">
                Dashboard
              </TabsTrigger>
              <TabsTrigger value="entries" className="w-24 text-xs">
                Entries
              </TabsTrigger>
            </TabsList>
            <div className="relative min-h-0 flex-1 overflow-hidden">
              <CrossfadeSlot
                frame={tabs.frames.dashboard}
                animate={tabs.animate}
                mounted={tabs.mounted.dashboard}
                className="absolute inset-0"
              >
                <DashboardTab />
              </CrossfadeSlot>
              <CrossfadeSlot
                frame={tabs.frames.entries}
                animate={tabs.animate}
                mounted={tabs.mounted.entries}
                className="absolute inset-0 flex min-h-0 flex-col gap-1"
              >
                <button
                  type="button"
                  onClick={openCreate}
                  className="flex shrink-0 items-center gap-1 self-start rounded-sm text-[13px] text-neutral-400 transition-colors hover:text-neutral-600 focus-visible:outline focus-visible:outline-offset-2 focus-visible:outline-blue-500 dark:text-neutral-500 dark:hover:text-neutral-300"
                >
                  <SquarePenIcon className="h-3.5 w-3.5 shrink-0" /> Add entry
                </button>
                <EntriesTab onOpenEntry={openEditor} />
              </CrossfadeSlot>
            </div>
          </Tabs>
        </main>
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
  )
}
