import { auth } from "@clerk/nextjs/server";

import { FolioBrowser } from "./_components/folio-browser";
import { Landing } from "./_landing/landing";

type HomeProps = {
  searchParams: Promise<{
    path?: string | string[];
  }>;
};

export default async function Home({ searchParams }: HomeProps) {
  const { userId } = await auth();

  if (!userId) {
    return <Landing />;
  }

  const params = await searchParams;
  return <FolioBrowser pathParam={params.path} />;
}
