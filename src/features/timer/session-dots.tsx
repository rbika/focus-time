type Props = {
  completed: number
  total: number
}

export function SessionDots({ completed, total }: Props) {
  const count = total
  return (
    <ol
      className="flex items-center justify-center gap-1.5"
      aria-label={`${completed} of ${count} Sessions until the long break`}
    >
      {Array.from({ length: count }, (_, index) => {
        const filled = index < completed
        return (
          <li
            key={index}
            className={
              filled
                ? 'size-1.5 rounded-full bg-neutral-900 dark:bg-neutral-50'
                : 'size-1.5 rounded-full bg-neutral-300 dark:bg-neutral-600'
            }
            aria-hidden
          />
        )
      })}
    </ol>
  )
}
