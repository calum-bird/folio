import { SignIn } from "@clerk/nextjs";

import {
  Marginalia,
  PageShell,
  SiteFooter,
  SiteHeader,
} from "@/app/_components/site-chrome";
import { clerkAppearance } from "@/app/_components/clerk-appearance";

export default function SignInPage() {
  return (
    <PageShell>
      <SiteHeader />
      <main className="flex flex-1 flex-col items-center justify-center pt-6 pb-20 sm:pt-10">
        <div className="w-full max-w-[420px] folio-fade-up">
          <Marginalia className="mb-4">/mnt/foliofs.dev/.auth/sign-in</Marginalia>
          <h1
            className="folio-display mb-8 text-[28px] leading-none tracking-tight sm:text-[36px]"
            style={{ color: "var(--ink)" }}
          >
            <span style={{ color: "var(--vermillion)" }}>$</span> auth login
            <span className="folio-cursor folio-cursor--sm" />
          </h1>
          <SignIn appearance={clerkAppearance} />
        </div>
      </main>
      <SiteFooter />
    </PageShell>
  );
}
