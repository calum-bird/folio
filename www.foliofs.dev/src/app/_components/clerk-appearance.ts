/**
 * Shared Clerk appearance config so SignIn / SignUp / UserButton match the
 * editorial paper-and-ink palette. The css vars are resolved at runtime so
 * dark-mode flips just work.
 *
 * No `"use client"` here — this is plain data and is consumed from both
 * server components (page.tsx files) and client components (UserButton).
 */
export const clerkAppearance = {
  variables: {
    colorPrimary: "var(--vermillion)",
    colorBackground: "var(--paper)",
    colorInputBackground: "var(--paper-deep)",
    colorInputText: "var(--ink)",
    colorText: "var(--ink)",
    colorTextSecondary: "var(--ink-soft)",
    colorNeutral: "var(--ink)",
    colorDanger: "var(--vermillion)",
    fontFamily: "var(--font-mono), ui-monospace, monospace",
    fontFamilyButtons: "var(--font-mono), ui-monospace, monospace",
    borderRadius: "0",
    spacingUnit: "0.95rem",
  },
  elements: {
    rootBox: "w-full",
    card:
      "bg-paper border border-ink shadow-none rounded-none w-full p-7",
    headerTitle: "text-ink font-normal text-lg tracking-tight",
    headerSubtitle: "text-ink-soft text-sm",
    socialButtonsBlockButton:
      "border border-ink bg-transparent rounded-none uppercase tracking-[0.16em] text-[12px] hover:bg-ink hover:text-paper transition-colors",
    socialButtonsBlockButtonText: "font-normal",
    dividerLine: "bg-ink/20",
    dividerText: "text-ink-faint uppercase tracking-[0.18em] text-[10px]",
    formFieldLabel: "text-ink-soft uppercase tracking-[0.18em] text-[10px]",
    formFieldInput:
      "bg-paper-deep border border-ink/30 rounded-none text-ink focus:border-ink focus:ring-0 font-mono",
    formButtonPrimary:
      "bg-ink text-paper rounded-none uppercase tracking-[0.16em] text-[12px] hover:bg-vermillion transition-colors",
    footer: "bg-transparent",
    footerAction: "text-ink-soft",
    footerActionLink: "text-ink hover:text-vermillion underline-offset-4",
    identityPreviewEditButton: "text-vermillion hover:text-vermillion",
    formFieldAction: "text-vermillion hover:text-vermillion",
    formFieldErrorText: "text-vermillion text-[12px]",
    alertText: "text-ink",
    alert: "border border-vermillion bg-transparent rounded-none",
  },
};
