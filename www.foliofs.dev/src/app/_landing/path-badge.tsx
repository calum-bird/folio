"use client";

import { useEffect, useState } from "react";

const PATHS = [
  "linear/issues/DEV-153.md",
  "slack/channels/#general.md",
  "github/folio/README.md",
  "gmail/inbox/q4-plan.md",
  "salesforce/opportunities/acme-corp.md",
];

const ROTATE_MS = 2400;

export function PathBadge() {
  const [index, setIndex] = useState(0);
  const [paused, setPaused] = useState(false);

  useEffect(() => {
    if (paused) return;
    if (typeof window !== "undefined") {
      const prefersReducedMotion = window.matchMedia(
        "(prefers-reduced-motion: reduce)",
      ).matches;
      if (prefersReducedMotion) return;
    }
    const interval = setInterval(() => {
      setIndex((i) => (i + 1) % PATHS.length);
    }, ROTATE_MS);
    return () => clearInterval(interval);
  }, [paused]);

  const path = PATHS[index];

  return (
    <span
      className="folio-badge"
      onMouseEnter={() => setPaused(true)}
      onMouseLeave={() => setPaused(false)}
      onFocus={() => setPaused(true)}
      onBlur={() => setPaused(false)}
      tabIndex={0}
      aria-live="polite"
    >
      <span key={path} className="folio-badge__path">
        $ cat /mnt/foliofs.dev/{path}
      </span>
    </span>
  );
}
