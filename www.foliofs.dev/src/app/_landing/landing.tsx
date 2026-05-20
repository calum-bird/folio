import { auth } from "@clerk/nextjs/server";
import Link from "next/link";

import { CopyButton } from "@/app/_components/copy-button";
import {
  Marginalia,
  PageShell,
  SectionHeading,
  SiteFooter,
  SiteHeader,
} from "@/app/_components/site-chrome";

import { ConnectorTree } from "./connector-tree";
import { PathBadge } from "./path-badge";

const INSTALL_COMMAND = "curl -fsSL https://foliofs.dev/install.sh | sh";

export async function Landing() {
  const { userId } = await auth();
  const signedIn = !!userId;

  return (
    <PageShell>
      <SiteHeader />
      <Hero signedIn={signedIn} />
      <SiteFooter />
    </PageShell>
  );
}

function Hero({ signedIn }: { signedIn: boolean }) {
  return (
    <section className="flex flex-1 flex-col pt-10 pb-40 sm:pt-14 sm:pb-56">
      <Marginalia>/mnt/foliofs.dev/README.md</Marginalia>

      <h1 className="folio-display folio-fade-up mt-6 text-[14vw] leading-none tracking-tight sm:text-[64px] md:text-[88px] lg:text-[108px] xl:text-[128px]">
        <span style={{ color: "var(--vermillion)" }}>/</span>mnt
        <span style={{ color: "var(--vermillion)" }}>/</span>the
        <span style={{ color: "var(--vermillion)" }}>/</span>cloud.md
      </h1>

      <div className="mt-10 grid grid-cols-12 items-center gap-x-10 gap-y-8 sm:mt-14">
        <p
          className="folio-fade-up col-span-12 max-w-[58ch] text-[15px] leading-[1.7] lg:col-span-6 sm:text-[16px]"
          style={{ color: "var(--ink-soft)", animationDelay: "120ms" }}
        >
          FolioFS is a network drive that puts the cloud services you use every
          day onto your machine as Markdown files. Your agents will love it —
          now they can just <PathBadge />
        </p>

        <div
          className="folio-fade-up col-span-12 flex flex-col gap-3 lg:col-span-6"
          style={{ animationDelay: "180ms" }}
        >
          <InstallCTA />
          <div
            className="flex flex-wrap items-center justify-between gap-x-4 gap-y-1 text-[11px] uppercase tracking-[0.2em]"
            style={{ color: "var(--ink-faint)" }}
          >
            <span>macOS only</span>
            {signedIn ? (
              <span className="flex items-center gap-x-2">
                <span aria-hidden>·</span>
                <Link href="/app" className="folio-link">
                  open dashboard →
                </Link>
              </span>
            ) : null}
          </div>
     
        </div>
      </div>

      <div
        className="folio-fade-up mt-20 sm:mt-28"
        style={{ animationDelay: "260ms" }}
      >
        <SectionHeading id="integrations" meta="3 live · 7 coming soon">
          integrations
        </SectionHeading>
        <ConnectorTree />
      </div>
    </section>
  );
}

function InstallCTA() {
  return (
    <div className="folio-install-cta" role="group" aria-label="Install FolioFS">
      <span aria-hidden className="folio-install-cta__prompt">
        $
      </span>
      <code className="folio-install-cta__command">{INSTALL_COMMAND}</code>
      <CopyButton
        value={INSTALL_COMMAND}
        label="copy"
        ariaLabel="Copy install command"
        className="folio-install-cta__copy"
      />
    </div>
  );
}
