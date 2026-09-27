# Focus Timer

macOS menu bar countdown timer.

Requires **macOS 26 (Tahoe)** or newer.

![Screenshot of Focus Timer](./screenshots/focus-timer.png)

## Features

- Menu bar countdown timer and stopwatch.
- Light and dark mode.
- Timer state preserved across app restarts.
- Pause when your Mac sleeps.
- Stats for the day, week, and month.
- Add, edit, and delete focused-time entries.

## Installation instructions

1. Download the latest `.dmg` file from [Github Releases page](https://github.com/rbika/focus-timer/releases).
2. Open and move the app into Applications folder.
3. Run the following command in the terminal to remove quarantine flag:
   ```shell
   xattr -cr /Applications/Focus\ Timer.app
   ```

## Develop

```bash
npm install
cp .env.example .env
npm run tauri dev
```

Debug builds load `.env` / `.env.local` from the project root. `ALWAYS_ON_TOP=true` keeps the timer panel visible while you work; set it to `false` for normal hide-on-blur. Release builds ignore these files and never pin.
