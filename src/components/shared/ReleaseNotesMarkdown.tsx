import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";

export function ReleaseNotesMarkdown({ body }: { body: string }) {
  return (
    <div className="min-w-0 max-h-72 overflow-auto break-words rounded-md border bg-muted/50 p-3 text-sm text-foreground">
      <ReactMarkdown
        remarkPlugins={[remarkGfm]}
        components={{
          h1: ({ children }) => <h3 className="mb-2 text-base font-semibold">{children}</h3>,
          h2: ({ children }) => <h3 className="mb-2 text-base font-semibold">{children}</h3>,
          h3: ({ children }) => <h4 className="mb-2 font-semibold">{children}</h4>,
          p: ({ children }) => <p className="mb-2 last:mb-0">{children}</p>,
          ul: ({ children }) => <ul className="mb-2 list-disc space-y-1 pl-5 last:mb-0">{children}</ul>,
          ol: ({ children }) => <ol className="mb-2 list-decimal space-y-1 pl-5 last:mb-0">{children}</ol>,
          a: ({ children, href }) => <a href={href} target="_blank" rel="noopener noreferrer" className="text-primary underline underline-offset-2">{children}</a>,
          code: ({ children }) => <code className="rounded bg-background px-1 font-mono text-xs">{children}</code>,
          pre: ({ children }) => <pre className="mb-2 overflow-x-auto rounded bg-background p-2 last:mb-0">{children}</pre>,
          blockquote: ({ children }) => <blockquote className="mb-2 border-l-2 border-primary pl-3 text-muted-foreground last:mb-0">{children}</blockquote>,
          table: ({ children }) => <div className="mb-2 overflow-x-auto"><table className="w-full border-collapse text-left">{children}</table></div>,
          th: ({ children }) => <th className="border px-2 py-1 font-semibold">{children}</th>,
          td: ({ children }) => <td className="border px-2 py-1">{children}</td>,
          img: ({ alt }) => <span>{alt}</span>,
        }}
      >
        {body}
      </ReactMarkdown>
    </div>
  );
}
