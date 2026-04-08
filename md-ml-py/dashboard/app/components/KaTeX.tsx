"use client";

import { useEffect, useRef, useState } from "react";
import katex from "katex";
import "katex/dist/katex.min.css";

interface MathProps {
  tex: string;
  display?: boolean;
  className?: string;
}

export function Math({ tex, display = false, className = "" }: MathProps) {
  const ref = useRef<HTMLSpanElement>(null);
  const [mounted, setMounted] = useState(false);

  useEffect(() => {
    setMounted(true);
  }, []);

  useEffect(() => {
    if (mounted && ref.current) {
      try {
        katex.render(tex, ref.current, {
          displayMode: display,
          throwOnError: false,
        });
      } catch {
        if (ref.current) ref.current.textContent = tex;
      }
    }
  }, [tex, display, mounted]);

  // Render placeholder on server, KaTeX on client
  if (!mounted) {
    return <span className={className}>{tex}</span>;
  }

  return <span ref={ref} className={className} suppressHydrationWarning />;
}
