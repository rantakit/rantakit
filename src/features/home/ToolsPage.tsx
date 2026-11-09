import { useState } from "react";
import { SearchX } from "@keyline-icons/react";
import { ToolCard } from "@/components/blocks/ToolCard";
import { useT } from "@/i18n";
import { SearchBox } from "./SearchBox";
import { searchTools } from "./searchTools";
import { JOBS, TOOL_ICONS, useTools, type ToolId } from "./tools";

interface ToolsPageProps {
  onOpenTool: (id: ToolId) => void;
}

/**
 * "Peralatan": search, then every Tool grouped by Job with its description,
 * on one scrolling page. Coming soon Tools follow the shipped ones.
 */
export function ToolsPage({ onOpenTool }: ToolsPageProps) {
  const { t } = useT();
  const [query, setQuery] = useState("");
  const searching = query.trim() !== "";
  const found = searchTools(useTools(), query);
  const groups = JOBS.map((job) => {
    const inJob = found.filter((tool) => tool.job === job);
    return { job, tools: [...inJob.filter((tool) => !tool.comingSoon), ...inJob.filter((tool) => tool.comingSoon)] };
  }).filter((group) => group.tools.length > 0);

  return (
    <div className="mx-auto flex w-full max-w-5xl flex-1 flex-col gap-9 px-5 pt-6 pb-10 md:px-12 md:pt-10 md:pb-12">
      <h1 className="font-display text-title">{t("home.all")}</h1>
      <SearchBox value={query} onChange={setQuery} />

      {searching && found.length === 0 && <NoToolFound />}

      {groups.length > 0 && (
        <section aria-label={searching ? t("home.search.results") : t("home.all")} className="flex flex-col gap-7">
          {groups.map(({ job, tools }) => (
            <div key={job} className="flex flex-col gap-3.5">
              <h2 className="font-semibold text-text-muted">{t(`job.${job}`)}</h2>
              <ul className="grid grid-cols-[repeat(auto-fill,minmax(272px,1fr))] gap-x-4 gap-y-5">
                {tools.map(({ id, comingSoon }) => (
                  <li key={id} className="flex">
                    <ToolCard
                      name={t(`tool.${id}`)}
                      description={t(`tool.${id}.description`)}
                      icon={TOOL_ICONS[id]}
                      comingSoon={comingSoon ? t("home.comingSoon") : undefined}
                      onPress={() => onOpenTool(id)}
                    />
                  </li>
                ))}
              </ul>
            </div>
          ))}
        </section>
      )}
    </div>
  );
}

/** "Peralatan tidak ditemukan", for a search that matches nothing. */
export function NoToolFound() {
  const { t } = useT();
  return (
    <div role="status" className="flex flex-col items-center gap-4 pt-18">
      <span className="flex size-22 items-center justify-center rounded-full bg-badge-bg text-text-muted">
        <SearchX aria-hidden className="size-10" />
      </span>
      <p className="text-section">{t("home.search.none")}</p>
    </div>
  );
}
