import { useEffect, useState } from 'react'

import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow'

import { Button } from '@/components/ui/button'
import { WindowTitleBar } from '@/components/window-title-bar'
import { api } from '@/lib/tauri'
import appIcon from '../../../src-tauri/icons/128x128.png'

export function UpToDateView() {
  const [appName, setAppName] = useState('Focus Time')
  const [version, setVersion] = useState('')

  useEffect(() => {
    void api.getAppName().then(setAppName)
    void api.getAppVersion().then(setVersion)
  }, [])

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === 'Escape') {
        event.preventDefault()
        void getCurrentWebviewWindow().hide()
      }
    }

    window.addEventListener('keydown', onKeyDown)
    return () => window.removeEventListener('keydown', onKeyDown)
  }, [])

  return (
    <div className="flex h-full flex-col overflow-hidden rounded-[18px] bg-[canvas]">
      <WindowTitleBar title="" />
      <main className="-mt-3 flex flex-1 flex-col gap-4 px-6 pb-5">
        <img src={appIcon} alt="" className="h-16 w-16 shrink-0" aria-hidden />
        <h1 className="text-[15px] font-semibold text-neutral-900 dark:text-neutral-50">
          You're up to date!
        </h1>
        <p className="mt-2 text-[13px] leading-5 text-neutral-700 dark:text-neutral-300">
          {appName} {version} is currently the newest version available.
        </p>
        <div className="-mx-2 mt-auto flex justify-end">
          <Button
            className="h-7 w-full rounded-full bg-[#007aff] px-4 py-2 text-sm hover:bg-[#006ee6] dark:bg-[#0a84ff] dark:text-white"
            onClick={() => void getCurrentWebviewWindow().hide()}
          >
            OK
          </Button>
        </div>
      </main>
    </div>
  )
}
