import { useEffect, useRef, useState } from 'react'

import { LogicalSize } from '@tauri-apps/api/dpi'
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow'

import { Button } from '@/components/ui/button'
import { WindowTitleBar } from '@/components/window-title-bar'
import { formatDownloadProgress } from '@/features/updates/format-bytes'
import { api, onUpdateStatus, type UpdateStatus } from '@/lib/tauri'
import appIcon from '../../../src-tauri/icons/128x128.png'

const PROGRESS_WINDOW_WIDTH = 400
const PROGRESS_WINDOW_HEIGHT = 148
const READY_WINDOW_HEIGHT = 180
const BAR_FILL_MS = 220

function downloadPercent(downloaded: number, total?: number | null): number {
  if (!total || total <= 0) {
    return 0
  }

  return Math.min(100, Math.round((downloaded / total) * 100))
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
    if (status.kind === 'cancelled' || status.kind === 'available') {
      void getCurrentWebviewWindow().hide()
    }
  }, [status.kind])

  useEffect(() => {
    const previous = previousKind.current
    previousKind.current = status.kind

    if (status.kind !== 'readyToRestart') {
      setShowInstallStep(false)
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
    const height = showInstallStep
      ? READY_WINDOW_HEIGHT
      : PROGRESS_WINDOW_HEIGHT
    void getCurrentWebviewWindow().setSize(
      new LogicalSize(PROGRESS_WINDOW_WIDTH, height),
    )
  }, [showInstallStep])

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

  const isDownloading = status.kind === 'downloading'
  const holdingFullBar = status.kind === 'readyToRestart' && !showInstallStep
  const isReady = showInstallStep
  const isError = status.kind === 'error'

  const title = isReady
    ? 'Ready to install'
    : isError
      ? 'Update failed'
      : 'Downloading update…'

  if (isDownloading && status.kind === 'downloading') {
    lastProgressLabel.current = formatDownloadProgress(
      status.downloaded,
      status.total,
    )
  }

  const progressLabel =
    isDownloading || holdingFullBar ? lastProgressLabel.current : null

  const percent =
    isDownloading && status.kind === 'downloading'
      ? downloadPercent(status.downloaded, status.total)
      : holdingFullBar || isReady
        ? 100
        : 0

  return (
    <div className="flex h-full flex-col bg-[canvas]">
      <WindowTitleBar title="" />
      {isReady ? (
        <main className="mt-3 flex min-h-0 flex-1 flex-col px-5 pb-4">
          <div className="flex items-start gap-4">
            <img
              src={appIcon}
              alt=""
              className="h-14 w-14 shrink-0 rounded-[14px]"
              aria-hidden
            />
            <div className="flex min-w-0 flex-1 flex-col gap-3">
              <p className="text-[13px] text-neutral-900 dark:text-neutral-50">
                {title}
              </p>
              {installFailed ? (
                <p className="text-[12px] leading-4 text-neutral-600 dark:text-neutral-400">
                  Install failed. Try again.
                </p>
              ) : (
                <div
                  className="h-1.5 overflow-hidden rounded-full bg-neutral-200 dark:bg-neutral-700"
                  role="progressbar"
                  aria-valuemin={0}
                  aria-valuemax={100}
                  aria-valuenow={percent}
                  aria-label={title}
                >
                  <div
                    className="h-full rounded-full bg-[#007aff] dark:bg-[#0a84ff]"
                    style={{ width: `${percent}%` }}
                  />
                </div>
              )}
            </div>
          </div>
          <div className="mt-auto flex justify-end gap-2">
            <Button
              variant="secondary"
              className="h-7 rounded-full px-4 py-2 text-sm"
              disabled={installRequested}
              onClick={() => void api.dismissUpdateProgress()}
            >
              Later
            </Button>
            <Button
              className="h-7 rounded-full bg-[#007aff] px-4 py-2 text-sm hover:bg-[#006ee6] disabled:opacity-60 dark:bg-[#0a84ff] dark:text-white"
              disabled={installRequested}
              onClick={() => {
                setInstallRequested(true)
                setInstallFailed(false)
                void api.installAndRestart().catch(() => {
                  setInstallRequested(false)
                  setInstallFailed(true)
                })
              }}
            >
              Install and restart
            </Button>
          </div>
        </main>
      ) : (
        <main className="mt-3 flex min-h-0 flex-1 items-start gap-4 px-5 pb-4">
          <img
            src={appIcon}
            alt=""
            className="h-14 w-14 shrink-0 rounded-[14px]"
            aria-hidden
          />
          <div className="flex min-w-0 flex-1 flex-col gap-3">
            <p className="text-[13px] text-neutral-900 dark:text-neutral-50">
              {title}
            </p>

            {!isError ? (
              <div
                className="h-1.5 overflow-hidden rounded-full bg-neutral-200 dark:bg-neutral-700"
                role="progressbar"
                aria-valuemin={0}
                aria-valuemax={100}
                aria-valuenow={percent}
                aria-label={title}
              >
                <div
                  ref={barRef}
                  className="h-full rounded-full bg-[#007aff] transition-[width] duration-150 ease-out dark:bg-[#0a84ff]"
                  style={{ width: `${percent}%` }}
                />
              </div>
            ) : null}

            <div className="flex items-center justify-between gap-3">
              {isError ? (
                <p className="text-[12px] leading-4 text-neutral-600 dark:text-neutral-400">
                  Please try again later.
                </p>
              ) : (
                <p className="text-[12px] leading-4 text-neutral-600 tabular-nums dark:text-neutral-400">
                  {progressLabel ?? '\u00a0'}
                </p>
              )}

              {isError ? (
                <Button
                  variant="secondary"
                  className="h-7 shrink-0 rounded-full border border-[#007aff] px-4 py-2 text-sm dark:border-[#0a84ff]"
                  onClick={() => void api.dismissUpdateProgress()}
                >
                  OK
                </Button>
              ) : isDownloading ? (
                <Button
                  variant="secondary"
                  className="px-5"
                  onClick={() => void api.cancelUpdateDownload()}
                >
                  Cancel
                </Button>
              ) : null}
            </div>
          </div>
        </main>
      )}
    </div>
  )
}
