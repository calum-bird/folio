import { ClerkProvider } from "@clerk/nextjs";
import type { Metadata } from "next";
import { Fragment_Mono, JetBrains_Mono, Tinos } from "next/font/google";

import "./globals.css";
import "./app.css";
import { cn } from "@/lib/utils";

const jetbrainsMono = JetBrains_Mono({
  subsets: ["latin"],
  variable: "--font-mono",
});

const fragmentMono = Fragment_Mono({
  weight: "400",
  subsets: ["latin"],
  variable: "--font-display",
});

// Brand guide specifies Tiempos Text (Klim, commercial); Tinos is the closest
// freely available Times-family analog (classical proportions, bracketed
// serifs, high stroke contrast) and serves as our open substitute, matching
// the same choice already used in the wordmark/lockup SVGs.
const tinos = Tinos({
  weight: ["400", "700"],
  style: ["normal", "italic"],
  subsets: ["latin"],
  variable: "--font-serif",
});

export const metadata: Metadata = {
  title: "FolioFS — mount the cloud",
  description:
    "FolioFS turns the services you already use into a real filesystem on your machine. So agents can read your work with ls, cat, and grep.",
};

export default function RootLayout({
  children,
}: Readonly<{
  children: React.ReactNode;
}>) {
  return (
    <html
      lang="en"
      className={cn(
        "h-full antialiased",
        jetbrainsMono.variable,
        fragmentMono.variable,
        tinos.variable,
        "font-mono",
      )}
    >
      <body className="min-h-full flex flex-col">
        <ClerkProvider>{children}</ClerkProvider>
      </body>
    </html>
  );
}
