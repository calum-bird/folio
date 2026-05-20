"use client";

import { useEffect, useRef, useState } from "react";

type CopyButtonProps = {
  value: string;
  label?: string;
  ariaLabel?: string;
  className?: string;
};

export function CopyButton({
  value,
  label = "copy",
  ariaLabel,
  className,
}: CopyButtonProps) {
  const [copied, setCopied] = useState(false);
  const timeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  useEffect(() => {
    return () => {
      if (timeoutRef.current) clearTimeout(timeoutRef.current);
    };
  }, []);

  async function handleCopy() {
    try {
      await navigator.clipboard.writeText(value);
    } catch {
      return;
    }
    setCopied(true);
    if (timeoutRef.current) clearTimeout(timeoutRef.current);
    timeoutRef.current = setTimeout(() => setCopied(false), 1400);
  }

  return (
    <button
      type="button"
      className={className}
      data-copied={copied ? "true" : undefined}
      onClick={handleCopy}
      aria-label={ariaLabel ?? `Copy ${label}`}
    >
      {copied ? <CheckIcon /> : <CopyIcon />}
      <span>{copied ? "copied" : label}</span>
    </button>
  );
}

function CopyIcon() {
  return (
    <svg
      viewBox="0 0 16 16"
      width="12"
      height="12"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.4"
      aria-hidden
    >
      <rect x="5.5" y="5.5" width="8.5" height="8.5" />
      <path d="M10.5 5.5V3.4a1.4 1.4 0 0 0-1.4-1.4H3.4A1.4 1.4 0 0 0 2 3.4v5.7a1.4 1.4 0 0 0 1.4 1.4H5.5" />
    </svg>
  );
}

function CheckIcon() {
  return (
    <svg
      viewBox="0 0 16 16"
      width="12"
      height="12"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.6"
      aria-hidden
    >
      <path d="M2.5 8.5L6 12l7.5-8" />
    </svg>
  );
}
