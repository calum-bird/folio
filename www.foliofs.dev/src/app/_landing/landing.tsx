import Link from "next/link";

import {
  Arrow,
  Marginalia,
  PageShell,
  SectionHeading,
  SiteFooter,
  SiteHeader,
} from "@/app/_components/site-chrome";

import { ConnectorTree } from "./connector-tree";

export function Landing() {
  return (
    <PageShell>
      <SiteHeader />
      <Hero />
      <SiteFooter />
    </PageShell>
  );
}

function Hero() {
  return (
    <section className="flex flex-1 flex-col pt-10 pb-40 sm:pt-14 sm:pb-56">
      <Marginalia>/mnt/foliofs.dev/README.md</Marginalia>

      <h1 className="folio-display folio-fade-up mt-6 text-[14vw] leading-none tracking-tight sm:text-[64px] md:text-[88px] lg:text-[108px] xl:text-[128px]">
        <span style={{ color: "var(--vermillion)" }}>/</span>mnt
        <span style={{ color: "var(--vermillion)" }}>/</span>the
        <span style={{ color: "var(--vermillion)" }}>/</span>cloud.md
      </h1>

      <div
        className="folio-fade-up mt-10 grid grid-cols-12 items-end gap-y-8 sm:mt-14"
        style={{ animationDelay: "120ms" }}
      >
        <p
          className="col-span-12 max-w-[58ch] text-[15px] leading-[1.7] sm:col-span-7 sm:text-[16px]"
          style={{ color: "var(--ink-soft)" }}
        >
          FolioFS is a network drive that puts the cloud services you use every
          day onto your machine as Markdown files. Your agents will love it —
          <code style={{ color: "var(--vermillion)" }}> ls</code>,{" "}
          <code style={{ color: "var(--vermillion)" }}>cat</code>,{" "}
          <code style={{ color: "var(--vermillion)" }}>grep</code> now work with
          all your cloud data.
        </p>
        <div className="col-span-12 flex flex-wrap items-center gap-5 sm:col-span-5 sm:justify-end">
          <Link href="/sign-up" className="folio-button">
            mount your cloud data
            <Arrow className="h-3 w-3" />
          </Link>
        </div>
      </div>

      <div
        className="folio-fade-up mt-12 grid gap-3 border p-4 sm:max-w-[680px]"
        style={{
          animationDelay: "180ms",
          borderColor: "var(--ink)",
          background: "color-mix(in srgb, var(--paper) 88%, var(--ink) 12%)",
        }}
      >
        <p
          className="folio-marginalia"
          style={{ letterSpacing: "0.06em", textTransform: "none" }}
        >
          Apple Silicon · macOS
        </p>
        <code className="block overflow-x-auto whitespace-nowrap text-[13px] sm:text-[15px]">
          curl -fsSL https://foliofs.dev/install.sh | sh
        </code>
      </div>

      <div
        className="folio-fade-up mt-20 sm:mt-28"
        style={{ animationDelay: "240ms" }}
      >
        <SectionHeading id="integrations" meta="3 live · 7 coming soon">
          integrations
        </SectionHeading>
        <ConnectorTree />
      </div>
    </section>
  );
}
