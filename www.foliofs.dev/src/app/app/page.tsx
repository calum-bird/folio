import { FolioBrowser } from "@/app/_components/folio-browser";

type AppPageProps = {
  searchParams: Promise<{
    path?: string | string[];
  }>;
};

export default async function AppPage({ searchParams }: AppPageProps) {
  const params = await searchParams;
  return <FolioBrowser pathParam={params.path} />;
}
