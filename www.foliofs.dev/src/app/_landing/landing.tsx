import Link from "next/link";

import { ConnectorTree } from "./connector-tree";

import "./landing.css";

const SOURCE_URL = "https://github.com/calumbird/folio";

export function Landing() {
  return (
    <main className="folio-landing relative isolate flex min-h-full flex-1 flex-col overflow-hidden">
      <div className="folio-landing__container mx-auto flex w-full max-w-[1180px] flex-1 flex-col px-6 sm:px-10">
        <SiteHeader />
        <Hero />
        <SiteFooter />
      </div>
    </main>
  );
}

function SiteHeader() {
  return (
    <header className="flex items-center justify-between py-6 sm:py-8">
      <Link href="/" aria-label="FolioFS" className="group flex items-center gap-3">
        <Glyph className="h-5 w-5 transition-colors group-hover:text-vermillion" />
        <span className="folio-display text-[20px] tracking-tight sm:text-[22px]">folio.fs</span>
      </Link>
      <nav className="flex items-center gap-6 text-[12px] uppercase tracking-[0.18em]">
        <Link href={SOURCE_URL} className="folio-link">
          source
        </Link>
        <Link href="/sign-in" className="folio-link">
          sign in
        </Link>
      </nav>
    </header>
  );
}

function Hero() {
  return (
    <section className="flex flex-1 flex-col pt-10 pb-40 sm:pt-14 sm:pb-56">
      <p className="folio-marginalia">¶ ~/folio.fs/README.md</p>

      <h1 className="folio-display folio-fade-up mt-6 text-[13vw] leading-[0.95] tracking-tight sm:text-[68px] lg:text-[88px]">
        <span style={{ color: "var(--vermillion)" }}>#</span>{" "}
        your-cloud-data<span style={{ color: "var(--vermillion)" }}>.MD</span>
        <br />
        <span style={{ color: "var(--vermillion)" }}>#</span> for your agents
        <span style={{ color: "var(--vermillion)" }}>.</span>
        <span className="folio-cursor" />
      </h1>

      <div
        className="folio-fade-up mt-10 grid grid-cols-12 items-end gap-y-8 sm:mt-14"
        style={{ animationDelay: "120ms" }}
      >
        <p
          className="col-span-12 max-w-[58ch] text-[15px] leading-[1.7] sm:col-span-7 sm:text-[16px]"
          style={{ color: "var(--ink-soft)" }}
        >
          A network drive that renders the services you already use as Markdown
          files on your machine. Read your stack with{" "}
          <code style={{ color: "var(--vermillion)" }}>ls</code>,{" "}
          <code style={{ color: "var(--vermillion)" }}>cat</code>,{" "}
          <code style={{ color: "var(--vermillion)" }}>grep</code> — for you and
          your agents.
        </p>
        <div className="col-span-12 flex flex-wrap items-center gap-5 sm:col-span-5 sm:justify-end">
          <Link href="/sign-up" className="folio-button">
            mount your folio
            <Arrow className="h-3 w-3" />
          </Link>
          <Link
            href={SOURCE_URL}
            className="folio-link text-[12px] uppercase tracking-[0.2em]"
          >
            source
          </Link>
        </div>
      </div>

      <div
        className="folio-fade-up mt-20 sm:mt-28"
        style={{ animationDelay: "240ms" }}
      >
        <div className="mb-6 flex items-baseline justify-between gap-4">
          <h2
            className="folio-display text-[26px] leading-none sm:text-[34px]"
            style={{ color: "var(--ink)" }}
          >
            <span style={{ color: "var(--vermillion)" }}>##</span> what mounts
          </h2>
          <span
            className="folio-marginalia"
            style={{ letterSpacing: "0.06em", textTransform: "none" }}
          >
            click a path to connect ↘
          </span>
        </div>
        <ConnectorTree />
      </div>
    </section>
  );
}

function SiteFooter() {
  return (
    <footer
      className="flex flex-col gap-3 border-t py-7 text-[12px] sm:flex-row sm:items-center sm:justify-between"
      style={{ borderColor: "var(--ink)", color: "var(--ink-soft)" }}
    >
      <div className="flex items-center gap-3">
        <Glyph className="h-3.5 w-3.5" />
        <span>folio.fs</span>
        <span style={{ color: "var(--ink-faint)" }}>·</span>
        <span style={{ color: "var(--ink-faint)" }}>mit / apache-2.0</span>
      </div>
      <div className="flex items-center gap-5">
        <Link className="folio-link" href={SOURCE_URL}>
          github
        </Link>
        <Link className="folio-link" href="/sign-in">
          sign in
        </Link>
        <span style={{ color: "var(--ink-faint)" }}>rust + webdav</span>
      </div>
    </footer>
  );
}

function Glyph({ className }: { className?: string }) {
  return (
    <svg
      viewBox="0 0 24 24"
      className={className}
      fill="none"
      stroke="currentColor"
      strokeWidth="1.6"
      aria-hidden
    >
      <path d="M3 6h7l2 3h9v11H3z" />
      <line x1="7" y1="13" x2="17" y2="13" />
      <line x1="7" y1="16" x2="14" y2="16" />
    </svg>
  );
}

function Arrow({ className }: { className?: string }) {
  return (
    <svg
      viewBox="0 0 12 12"
      className={className}
      fill="none"
      stroke="currentColor"
      strokeWidth="1.5"
      aria-hidden
    >
      <path d="M2 6h8" />
      <path d="M6.5 2.5L10 6l-3.5 3.5" />
    </svg>
  );
}
