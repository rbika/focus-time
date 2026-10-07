import Markdown from 'react-markdown'

const textClass = 'text-[13px] leading-5 text-neutral-700 dark:text-neutral-300'

export function ReleaseNotes({ content }: { content?: string | null }) {
  const trimmed = content?.trim()
  if (!trimmed) {
    return <p className={textClass}>No release notes provided.</p>
  }

  return (
    <div
      className={`my-2 h-full max-h-52 overflow-y-auto overscroll-none rounded-md border border-neutral-200 p-3 dark:border-neutral-700 ${textClass} [&_h2+h3]:mt-2 [&_h2:not(:first-child)]:mt-3 [&_h3+ul]:mt-1 [&_p+p]:mt-2 [&_ul+p]:mt-2`}
    >
      <Markdown
        components={{
          p: ({ children }) => <p>{children}</p>,
          ul: ({ children }) => (
            <ul className="list-disc space-y-1 pl-4">{children}</ul>
          ),
          ol: ({ children }) => (
            <ol className="list-decimal space-y-1 pl-4">{children}</ol>
          ),
          h1: ({ children }) => (
            <h1 className="text-[15px] font-semibold text-neutral-900 dark:text-neutral-50">
              {children}
            </h1>
          ),
          h2: ({ children }) => (
            <h2 className="text-[16px] font-semibold text-neutral-900 dark:text-neutral-50">
              {children}
            </h2>
          ),
          h3: ({ children }) => (
            <h3 className="mt-3 font-semibold text-neutral-900 dark:text-neutral-50">
              {children}
            </h3>
          ),
          a: ({ href, children }) => (
            <a
              href={href}
              className="text-[#007aff] hover:underline dark:text-[#0a84ff]"
              target="_blank"
              rel="noopener noreferrer"
            >
              {children}
            </a>
          ),
          strong: ({ children }) => (
            <strong className="font-semibold">{children}</strong>
          ),
          code: ({ children }) => (
            <code className="rounded bg-neutral-100 px-1 py-0.5 font-mono text-[12px] dark:bg-neutral-800">
              {children}
            </code>
          ),
        }}
      >
        {trimmed}
      </Markdown>
    </div>
  )
}
