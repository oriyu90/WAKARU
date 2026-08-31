import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";
import rehypeKatex from "rehype-katex";
import "katex/dist/katex.min.css";
import styles from "./Markdown.module.css";

/** Read-only Markdown for AI output and reader views. No `rehype-raw` — raw HTML
 * from a model or a fetched page is never injected (docs/02 §1). */
export function Markdown({ children }: { children: string }) {
  return (
    <div className={`${styles.md} reading`}>
      <ReactMarkdown
        remarkPlugins={[remarkGfm]}
        rehypePlugins={[rehypeKatex]}
        components={{
          a: ({ node: _node, ...props }) => (
            <a {...props} target="_blank" rel="noreferrer noopener" />
          ),
        }}
      >
        {children}
      </ReactMarkdown>
    </div>
  );
}
