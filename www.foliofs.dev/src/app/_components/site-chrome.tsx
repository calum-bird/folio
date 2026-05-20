import { UserButton } from "@clerk/nextjs";
import { auth } from "@clerk/nextjs/server";
import Link from "next/link";

type PageShellProps = {
  children: React.ReactNode;
  /** Vertical content alignment in the main column. Defaults to "start". */
  align?: "start" | "center";
  /** Override the max width of the inner container. */
  maxWidth?: string;
};

export function PageShell({
  children,
  align = "start",
  maxWidth = "1180px",
}: PageShellProps) {
  return (
    <div className="folio-page relative isolate flex min-h-full flex-1 flex-col overflow-x-hidden">
      <div
        className={`folio-page__container mx-auto flex w-full flex-1 flex-col px-6 sm:px-10 ${
          align === "center" ? "justify-center" : ""
        }`}
        style={{ maxWidth }}
      >
        {children}
      </div>
    </div>
  );
}

export async function SiteHeader() {
  const { userId } = await auth();
  const signedIn = !!userId;

  return (
    <header className="flex items-center justify-between py-6 sm:py-8">
      <Link href="/" aria-label="FolioFS" className="folio-brand-link">
        <BrandLockup className="folio-brand-mark" />
      </Link>
      <nav className="flex items-center gap-6 text-[12px] uppercase tracking-[0.18em]">
        {signedIn ? (
          <>
            <Link href="/app" className="folio-link">
              dashboard
            </Link>
            <Link href="/app/connections" className="folio-link">
              connections
            </Link>
            <UserButton
              appearance={{
                elements: {
                  avatarBox: "h-7 w-7 rounded-none border border-current",
                  userButtonPopoverCard:
                    "bg-paper text-ink border border-ink shadow-none rounded-none",
                },
              }}
            />
          </>
        ) : (
          <Link href="/sign-in" className="folio-link">
            sign in
          </Link>
        )}
      </nav>
    </header>
  );
}

export function SiteFooter() {
  return (
    <footer
      className="mt-auto flex flex-col gap-3 border-t py-7 text-[12px] sm:flex-row sm:items-center sm:justify-between"
      style={{ borderColor: "var(--ink)", color: "var(--ink-soft)" }}
    >
      <div className="flex items-center gap-3">
        {/* eslint-disable-next-line @next/next/no-img-element */}
        <img
          src="/branding/icon/foliofs-icon.svg"
          alt=""
          aria-hidden
          className="h-3.5 w-3.5"
        />
        <span>foliofs.dev</span>
      </div>
      <div className="flex items-center gap-5">
        <span style={{ color: "var(--ink-faint)" }}>built with rust + webdav</span>
      </div>
    </footer>
  );
}

type MarginaliaProps = {
  children: React.ReactNode;
  className?: string;
};

export function Marginalia({ children, className }: MarginaliaProps) {
  return (
    <p className={"folio-marginalia " + (className ?? "")}>¶ {children}</p>
  );
}

type SectionHeadingProps = {
  level?: 1 | 2 | 3;
  children: React.ReactNode;
  meta?: React.ReactNode;
  id?: string;
  className?: string;
};

export function SectionHeading({
  level = 2,
  children,
  meta,
  id,
  className,
}: SectionHeadingProps) {
  const marker = level === 1 ? "#" : level === 2 ? "##" : "###";
  const size =
    level === 1
      ? "text-[34px] sm:text-[44px]"
      : level === 2
        ? "text-[26px] sm:text-[34px]"
        : "text-[20px] sm:text-[24px]";

  return (
    <div
      id={id}
      className={
        "mb-6 flex items-baseline justify-between gap-4 " + (className ?? "")
      }
    >
      <h2 className={`folio-display ${size} leading-none`} style={{ color: "var(--ink)" }}>
        <span style={{ color: "var(--vermillion)" }}>{marker}</span> {children}
      </h2>
      {meta ? (
        <span
          className="folio-marginalia"
          style={{ letterSpacing: "0.06em", textTransform: "none" }}
        >
          {meta}
        </span>
      ) : null}
    </div>
  );
}

type PathDisplayProps = {
  /** Each segment renders as a vermillion `/` + ink segment text. */
  segments: { name: string; href?: string }[];
  /** Size variant. "hero" is the biggest, "sm" is small breadcrumb. */
  size?: "hero" | "lg" | "md" | "sm";
  /** Show a blinking cursor at the end. */
  cursor?: boolean;
  className?: string;
};

export function PathDisplay({
  segments,
  size = "md",
  cursor = false,
  className,
}: PathDisplayProps) {
  const sizeClass =
    size === "hero"
      ? "text-[14vw] sm:text-[64px] md:text-[88px] lg:text-[108px] xl:text-[128px]"
      : size === "lg"
        ? "text-[32px] sm:text-[44px] lg:text-[56px]"
        : size === "md"
          ? "text-[22px] sm:text-[28px]"
          : "text-[14px] sm:text-[15px]";

  return (
    <h1
      className={`folio-path folio-display leading-none tracking-tight ${sizeClass} ${className ?? ""}`}
    >
      {segments.map((segment, index) => {
        const isLast = index === segments.length - 1;
        return (
          <span key={`${segment.name}-${index}`} className="contents">
            <span className="folio-path__sep">/</span>
            {segment.href ? (
              <Link href={segment.href} className="folio-path__seg">
                {segment.name}
              </Link>
            ) : (
              <span
                className={`folio-path__seg ${isLast ? "folio-path__seg--current" : ""}`}
              >
                {segment.name}
              </span>
            )}
          </span>
        );
      })}
      {cursor ? <span className="folio-cursor" /> : null}
    </h1>
  );
}

function BrandLockup({ className }: { className?: string }) {
  return (
    <picture>
      <source
        srcSet="/branding/lockup/foliofs-lockup-dark.svg"
        media="(prefers-color-scheme: dark)"
      />
      <img
        src="/branding/lockup/foliofs-lockup-light.svg"
        alt="FolioFS"
        className={className}
      />
    </picture>
  );
}

export function Arrow({ className }: { className?: string }) {
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
