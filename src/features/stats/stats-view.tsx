import { useCallback, useEffect, useRef, useState } from 'react'

import { SquarePenIcon } from 'lucide-react'

import { Select } from '@/components/ui/select'
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
import {
  DEFAULT_PERIOD,
  isPeriod,
  PERIOD_OPTIONS,
  type Period,
} from '@/features/stats/period'
import { useEditorTransition } from '@/features/stats/use-editor-transition'
import { api, onEntriesChanged, type Entry } from '@/lib/tauri'

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
  const [period, setPeriod] = useState<Period>(DEFAULT_PERIOD)
  const [yearRevealSteps, setYearRevealSteps] = useState(0)
  const [editing, setEditing] = useState<Entry | null>(null)
  const [creating, setCreating] = useState(false)
  const { frames, animate, destination, show, hide, snapToList } =
    useEditorTransition()
  const tabs = useCrossfade('dashboard', 'entries', tab, CROSSFADE_NUDGE_PX)
  const wasActiveRef = useRef(active)

  const resetPeriod = useCallback(() => {
    setPeriod(DEFAULT_PERIOD)
    setYearRevealSteps(0)
  }, [])

  const goToTab = useCallback(
    (next: StatsTab) => {
      if (next === 'entries') resetPeriod()
      setTab(next)
    },
    [resetPeriod],
  )

  const dismissEditor = useCallback(() => {
    setEditing(null)
    setCreating(false)
  }, [])

  useEffect(() => {
    if (active) return
    snapToList()
    dismissEditor()
  }, [active, dismissEditor, snapToList])

  useEffect(() => {
    const wasActive = wasActiveRef.current
    wasActiveRef.current = active
    if (active && !wasActive && tab === 'entries') resetPeriod()
  }, [active, tab, resetPeriod])

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
    if (!active) return
    const onKeyDown = (event: KeyboardEvent) => {
      if (!event.metaKey || event.ctrlKey || event.altKey || event.shiftKey) {
        return
      }
      if (event.key !== '2') return
      event.preventDefault()
      if (destination === 'editor') return
      goToTab(tab === 'dashboard' ? 'entries' : 'dashboard')
    }
    window.addEventListener('keydown', onKeyDown)
    return () => window.removeEventListener('keydown', onKeyDown)
  }, [active, destination, goToTab, tab])

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
              if (value === 'dashboard' || value === 'entries') goToTab(value)
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
                <EntriesTab
                  period={period}
                  yearRevealSteps={yearRevealSteps}
                  onShowMore={() => setYearRevealSteps((steps) => steps + 1)}
                  onOpenEntry={openEditor}
                />
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
