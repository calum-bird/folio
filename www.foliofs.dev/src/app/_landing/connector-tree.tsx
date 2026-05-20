import Link from "next/link";

const SOURCE_URL = "https://github.com/calumbird/folio";

type Line = {
  prefix: string;
  text: string;
  isLeaf?: boolean;
};

type Row = {
  branch: "├──" | "└──";
  trunk: "│   " | "    ";
  domain: string;
  subtree: string;
  href: string;
  status: "live" | "soon";
  expansion: Line[];
};

const ROWS: Row[] = [
  {
    branch: "├──",
    trunk: "│   ",
    domain: "github.com",
    subtree: "/{org}/{repo}/{readme,issues,pulls}",
    href: "/api/connections/github/start",
    status: "live",
    expansion: [
      { prefix: "├── ", text: "index.md" },
      { prefix: "└── ", text: "acme/example/" },
      { prefix: "    ├── ", text: "issues/01-acme-doesnt-exist.md", isLeaf: true },
      { prefix: "    ├── ", text: "pulls/" },
      { prefix: "    └── ", text: "readme.md" },
    ],
  },
  {
    branch: "├──",
    trunk: "│   ",
    domain: "slack.com",
    subtree: "/{channels,group-messages,direct-messages}",
    href: "/api/connections/slack/start",
    status: "live",
    expansion: [
      { prefix: "├── ", text: "index.md" },
      { prefix: "└── ", text: "acme/" },
      {
        prefix: "    ├── ",
        text: "channels/#general/2026-05-19-still-no-acme.md",
        isLeaf: true,
      },
      { prefix: "    ├── ", text: "group-messages/" },
      { prefix: "    └── ", text: "direct-messages/" },
    ],
  },
  {
    branch: "├──",
    trunk: "│   ",
    domain: "salesforce.com",
    subtree: "/{companies,opportunities,contacts,meetings}",
    href: SOURCE_URL,
    status: "soon",
    expansion: [
      { prefix: "├── ", text: "index.md" },
      { prefix: "└── ", text: "acme/" },
      {
        prefix: "    ├── ",
        text: "opportunities/acme-corp-q4-pending.md",
        isLeaf: true,
      },
      { prefix: "    ├── ", text: "companies/" },
      { prefix: "    ├── ", text: "contacts/" },
      { prefix: "    └── ", text: "meetings/" },
    ],
  },
  {
    branch: "└──",
    trunk: "    ",
    domain: "linear.app",
    subtree: "/{team}/{project}/issues",
    href: "/api/connections/linear/start",
    status: "live",
    expansion: [
      { prefix: "├── ", text: "index.md" },
      { prefix: "└── ", text: "acme/" },
      { prefix: "    ├── ", text: "eng/ENG-142-fix-acme.md", isLeaf: true },
      { prefix: "    └── ", text: "design/" },
    ],
  },
];

export function ConnectorTree() {
  return (
    <div className="folio-tree">
      <p className="folio-tree__root">~/mnt/folio/</p>
      <ul className="folio-tree__list">
        {ROWS.map((row, index) => (
          <li
            key={row.domain}
            className="folio-tree__node folio-fade-up"
            style={{ animationDelay: `${160 + index * 70}ms` }}
          >
            <Link
              href={row.href}
              {...(row.status === "soon"
                ? { target: "_blank", rel: "noreferrer" }
                : {})}
              className="folio-tree__row"
              aria-label={`${row.domain}${row.subtree} — ${
                row.status === "soon" ? "coming soon" : "connect"
              }`}
            >
              <span aria-hidden className="folio-tree__branch">
                {row.branch}{" "}
              </span>
              <span className="folio-tree__domain">{row.domain}</span>
              <span className="folio-tree__sub">{row.subtree}</span>
              {row.status === "soon" ? (
                <span className="folio-tree__tag">soon</span>
              ) : null}
              <span aria-hidden className="folio-tree__arrow">
                →
              </span>
            </Link>
            <div className="folio-tree__expansion" aria-hidden>
              <div className="folio-tree__expansion-inner">
                {row.expansion.map((line, lineIdx) => (
                  <p
                    key={`${row.domain}-${lineIdx}`}
                    className={
                      "folio-tree__leaf" +
                      (line.isLeaf ? " folio-tree__leaf--leaf" : "")
                    }
                    style={{ ["--i" as string]: lineIdx } as React.CSSProperties}
                  >
                    <span className="folio-tree__leaf-branch">
                      {row.trunk}
                      {line.prefix}
                    </span>
                    <span className="folio-tree__leaf-name">{line.text}</span>
                  </p>
                ))}
              </div>
            </div>
          </li>
        ))}
      </ul>
    </div>
  );
}
