import { auth } from "@clerk/nextjs/server";
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
          <Link href={signedIn ? "/app" : "/sign-up"} className="folio-button">
            {signedIn ? "dashboard" : "mount your cloud data"}
            <Arrow className="h-3 w-3" />
          </Link>
        </div>
      </div>

      <InstallPanel />

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

function InstallPanel() {
  return (
    <section
      className="folio-install folio-fade-up mt-12"
      style={{ animationDelay: "180ms" }}
      aria-labelledby="install-title"
    >
      <div className="folio-install__header">
        <div>
          <p className="folio-marginalia">Apple Silicon · macOS</p>
          <h2 id="install-title" className="folio-install__title">
            install FolioFS
          </h2>
        </div>
        <span className="folio-tag folio-tag--accent">read-only</span>
      </div>

      <div className="folio-install__command" aria-label="Install command">
        <span className="folio-install__prompt">$</span>
        <code>curl -fsSL https://foliofs.dev/install.sh | sh</code>
      </div>

      <ol className="folio-install__steps">
        <InstallStep
          index="01"
          label="log in once"
          command="folio login"
        />
        <InstallStep
          index="02"
          label="start the menu-bar app"
          command="folio start"
        />
        <InstallStep
          index="03"
          label="open the mounted drive"
          command="open /Volumes/foliofs.dev"
        />
      </ol>
    </section>
  );
}

function InstallStep({
  index,
  label,
  command,
}: {
  index: string;
  label: string;
  command: string;
}) {
  return (
    <li className="folio-install__step">
      <span className="folio-install__step-index">{index}</span>
      <span className="folio-install__step-label">{label}</span>
      <code className="folio-install__step-command">{command}</code>
    </li>
  );
}
