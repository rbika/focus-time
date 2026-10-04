import { useEffect, useRef, useState, type ReactNode, type Ref } from 'react'

import { LogicalSize } from '@tauri-apps/api/dpi'
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow'

import { Button } from '@/components/ui/button'
import { WindowTitleBar } from '@/components/window-title-bar'
import { formatDownloadProgress } from '@/features/updates/format-bytes'
import {
  api,
  onUpdateProgressPreview,
  onUpdateStatus,
  type UpdateStatus,
} from '@/lib/tauri'
import appIcon from '../../../src-tauri/icons/128x128.png'

const PROGRESS_WINDOW_WIDTH = 400
const PROGRESS_WINDOW_HEIGHT = 148
const BAR_FILL_MS = 220

type ProgressView =
  | { kind: 'hidden' }
  | {
      kind: 'downloading'
      percent: number
      label: string | null
      cancelDisabled: boolean
    }
  | { kind: 'ready'; installRequested: boolean }
  | { kind: 'installFailed'; installRequested: boolean }
  | { kind: 'error' }

function downloadPercent(downloaded: number, total?: number | null): number {
  if (!total || total <= 0) {
    return 0
  }

  return Math.min(100, Math.round((downloaded / total) * 100))
}

function toProgressView({
  status,
  showInstallStep,
  installFailed,
  installRequested,
  progressLabel,
}: {
  status: UpdateStatus
  showInstallStep: boolean
  installFailed: boolean
  installRequested: boolean
  progressLabel: string | null
}): ProgressView {
  if (status.kind === 'downloading') {
    return {
      kind: 'downloading',
      percent: downloadPercent(status.downloaded, status.total),
      label: progressLabel,
      cancelDisabled: false,
    }
  }

  if (status.kind === 'readyToRestart' && !showInstallStep) {
    return {
      kind: 'downloading',
      percent: 100,
      label: progressLabel,
      cancelDisabled: true,
    }
  }

  if (status.kind === 'readyToRestart') {
    if (installFailed) {
      return { kind: 'installFailed', installRequested }
    }

    return { kind: 'ready', installRequested }
  }

  if (status.kind === 'error') {
    return { kind: 'error' }
  }

  return { kind: 'hidden' }
}

function ProgressBar({
  percent,
  label,
  animated = false,
  barRef,
}: {
  percent: number
  label: string
  animated?: boolean
  barRef?: Ref<HTMLDivElement>
}) {
  return (
    <div
      className="my-1.25 h-1.5 overflow-hidden rounded-full bg-neutral-200 dark:bg-neutral-700"
      role="progressbar"
      aria-valuemin={0}
      aria-valuemax={100}
      aria-valuenow={percent}
      aria-label={label}
    >
      <div
        ref={barRef}
        className={
          animated
            ? 'h-full rounded-full bg-[#007aff] transition-[width] duration-150 ease-out dark:bg-[#0a84ff]'
            : 'h-full rounded-full bg-[#007aff] dark:bg-[#0a84ff]'
        }
        style={{ width: `${percent}%` }}
      />
    </div>
  )
}

function InstallActions({
  disabled,
  onLater,
  onInstall,
}: {
  disabled: boolean
  onLater: () => void
  onInstall: () => void
}) {
  return (
    <>
      <Button
        variant="secondary"
        className="h-7 rounded-full px-4 py-2 text-sm"
        disabled={disabled}
        onClick={onLater}
      >
        Later
      </Button>
      <Button
        className="h-7 rounded-full bg-[#007aff] px-4 py-2 text-sm hover:bg-[#006ee6] disabled:opacity-60 dark:bg-[#0a84ff] dark:text-white"
        disabled={disabled}
        onClick={onInstall}
      >
        Install and restart
      </Button>
    </>
  )
}

function ProgressFrame({
  title,
  body,
  caption,
  actions,
}: {
  title: string
  body: ReactNode
  caption: ReactNode
  actions: ReactNode
}) {
  return (
    <main className="mt-3 flex min-h-0 flex-1 items-start gap-4 px-5 pb-4">
      <img
        src={appIcon}
        alt=""
        className="h-14 w-14 shrink-0 rounded-[14px]"
        aria-hidden
      />
      <div className="flex min-w-0 flex-1 flex-col gap-1.5">
        <p className="text-[13px] text-neutral-900 dark:text-neutral-50">
          {title}
        </p>
        {body}
        <div
          className={
            caption
              ? 'mt-1 flex items-start justify-between gap-3'
              : 'mt-1 flex items-start justify-end gap-2'
          }
        >
          {caption}
          {actions}
        </div>
      </div>
    </main>
  )
}

export function UpdateProgressView() {
  const [status, setStatus] = useState<UpdateStatus>({ kind: 'idle' })
  const [showInstallStep, setShowInstallStep] = useState(false)
  const [installRequested, setInstallRequested] = useState(false)
  const [installFailed, setInstallFailed] = useState(false)
  const barRef = useRef<HTMLDivElement>(null)
  const previousKind = useRef(status.kind)
  const lastProgressLabel = useRef<string | null>(null)

  useEffect(() => {
    void api.getUpdateStatus().then(setStatus)
  }, [])

  useEffect(() => {
    let unlisten: (() => void) | undefined
    void onUpdateStatus(setStatus).then((fn) => {
      unlisten = fn
    })
    return () => {
      unlisten?.()
    }
  }, [])

  useEffect(() => {
    let unlisten: (() => void) | undefined
    void onUpdateProgressPreview((preview) => {
      setInstallFailed(preview.installFailed)
      setInstallRequested(false)
    }).then((fn) => {
      unlisten = fn
    })
    return () => {
      unlisten?.()
    }
  }, [])

  useEffect(() => {
    if (status.kind === 'cancelled' || status.kind === 'available') {
      void getCurrentWebviewWindow().hide()
    }
  }, [status.kind])

  useEffect(() => {
    const previous = previousKind.current
    previousKind.current = status.kind

    if (status.kind !== 'readyToRestart') {
      setShowInstallStep(false)
      setInstallFailed(false)
      return
    }

    if (previous !== 'downloading') {
      setShowInstallStep(true)
      return
    }

    let cancelled = false
    const show = () => {
      if (!cancelled) {
        setShowInstallStep(true)
      }
    }
    const bar = barRef.current
    const onEnd = (event: TransitionEvent) => {
      if (event.propertyName === 'width') {
        show()
      }
    }
    bar?.addEventListener('transitionend', onEnd)
    const timeout = window.setTimeout(show, BAR_FILL_MS)
    return () => {
      cancelled = true
      bar?.removeEventListener('transitionend', onEnd)
      window.clearTimeout(timeout)
    }
  }, [status.kind])

  useEffect(() => {
    void getCurrentWebviewWindow().setSize(
      new LogicalSize(PROGRESS_WINDOW_WIDTH, PROGRESS_WINDOW_HEIGHT),
    )
  }, [])

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key !== 'Escape') {
        return
      }

      event.preventDefault()
      if (status.kind === 'downloading') {
        void api.cancelUpdateDownload()
      } else if (status.kind === 'readyToRestart' && showInstallStep) {
        void api.dismissUpdateProgress()
      }
    }

    window.addEventListener('keydown', onKeyDown)
    return () => window.removeEventListener('keydown', onKeyDown)
  }, [showInstallStep, status.kind])

  if (status.kind === 'downloading') {
    lastProgressLabel.current = formatDownloadProgress(
      status.downloaded,
      status.total,
    )
  }

  const view = toProgressView({
    status,
    showInstallStep,
    installFailed,
    installRequested,
    progressLabel: lastProgressLabel.current,
  })

  const onLater = () => {
    void api.dismissUpdateProgress()
  }

  const onInstall = () => {
    setInstallRequested(true)
    setInstallFailed(false)
    void api.installAndRestart().catch(() => {
      setInstallRequested(false)
      setInstallFailed(true)
    })
  }

  let content: ReactNode = null
  switch (view.kind) {
    case 'hidden':
      break
    case 'downloading':
      content = (
        <ProgressFrame
          title="Downloading update…"
          body={
            <ProgressBar
              percent={view.percent}
              label="Downloading update…"
              animated
              barRef={barRef}
            />
          }
          caption={
            <p className="text-[12px] leading-4 text-neutral-600 tabular-nums dark:text-neutral-400">
              {view.label ?? '\u00a0'}
            </p>
          }
          actions={
            <Button
              variant="secondary"
              className="rounded-full px-5"
              disabled={view.cancelDisabled}
              onClick={() => void api.cancelUpdateDownload()}
            >
              Cancel
            </Button>
          }
        />
      )
      break
    case 'ready':
      content = (
        <ProgressFrame
          title="Ready to install"
          body={<ProgressBar percent={100} label="Ready to install" />}
          caption={null}
          actions={
            <InstallActions
              disabled={view.installRequested}
              onLater={onLater}
              onInstall={onInstall}
            />
          }
        />
      )
      break
    case 'installFailed':
      content = (
        <ProgressFrame
          title="Install failed"
          body={
            <p className="text-[12px] leading-4 text-neutral-600 dark:text-neutral-400">
              Please try again later.
            </p>
          }
          caption={null}
          actions={
            <Button
              className="h-7 rounded-full bg-[#007aff] px-4 py-2 text-sm hover:bg-[#006ee6] dark:bg-[#0a84ff] dark:text-white"
              onClick={() => void api.dismissUpdateProgress()}
            >
              OK
            </Button>
          }
        />
      )
      break
    case 'error':
      content = (
        <ProgressFrame
          title="Update failed"
          body={
            <p className="text-[12px] leading-4 text-neutral-600 dark:text-neutral-400">
              Please try again later.
            </p>
          }
          caption={null}
          actions={
            <Button
              className="h-7 rounded-full bg-[#007aff] px-4 py-2 text-sm hover:bg-[#006ee6] dark:bg-[#0a84ff] dark:text-white"
              onClick={() => void api.dismissUpdateProgress()}
            >
              OK
            </Button>
          }
        />
      )
      break
  }

  return (
    <div className="flex h-full flex-col bg-[canvas]">
      <WindowTitleBar title="" />
      {content}
    </div>
  )
}
