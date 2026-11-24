/** Local `YYYY-MM-DDTHH:mm:ss` for a `datetime-local` input. */
export function toDatetimeLocalValue(unixSecs: number): string {
  const date = new Date(unixSecs * 1000)
  const pad = (n: number) => String(n).padStart(2, '0')
  return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())}T${pad(date.getHours())}:${pad(date.getMinutes())}:${pad(date.getSeconds())}`
}

/** Unix seconds from a `datetime-local` value, or `null` if empty/invalid. */
export function fromDatetimeLocalValue(value: string): number | null {
  if (!value) return null
  const parsed = new Date(value)
  if (Number.isNaN(parsed.getTime())) return null
  return Math.floor(parsed.getTime() / 1000)
}
