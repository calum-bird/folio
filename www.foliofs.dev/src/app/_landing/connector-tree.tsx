
import Link from "next/link";

type LiveRow = {
  domain: string;
  subtree: string;
  href: string;
  status: "live";
};

type SoonRow = {
  domain: string;
  subtree: string;
  status: "soon";
};

type Row = LiveRow | SoonRow;

const ROWS: Row[] = [
  {
    domain: "gdrive",
    subtree: "/{my-drive,shared-drives,starred}",
    status: "soon",
  },
  {
    domain: "fireflies",
    subtree: "/{meetings,transcripts,playlists}",
    status: "soon",
  },
  {
    domain: "github",
    subtree: "/{org}/{repo}/{readme,issues,pulls}",
    href: "/api/connections/github/start",
    status: "live",
  },
  {
    domain: "granola",
    subtree: "/{notes,meetings,actions}",
    status: "soon",
  },
  {
    domain: "linear",
    subtree: "/{team}/{project}/issues",
    href: "/api/connections/linear/start",
    status: "live",
  },
  {
    domain: "gmail",
    subtree: "/{inbox,sent,drafts,labels}",
    status: "soon",
  },
  {
    domain: "onedrive",
    subtree: "/{my-files,shared,starred}",
    status: "soon",
  },
  {
    domain: "outlook",
    subtree: "/{inbox,sent,calendar,contacts}",
    status: "soon",
  },
  {
    domain: "salesforce",
    subtree: "/{companies,opportunities,contacts,meetings}",
    status: "soon",
  },
  {
    domain: "slack",
    subtree: "/{channels,group-messages,direct-messages}",
    href: "/api/connections/slack/start",
    status: "live",
  },
];

function InnerRow({ row, index }: { row: Row, index: number }) {
  const isLast = index === ROWS.length - 1;
  const isSoon = row.status === "soon";
  const branch = isLast ? "└──" : "├──";

  return (
    <>
      <span aria-hidden className="folio-tree__branch">
        {branch}{" "}
      </span>
      <span className="folio-tree__domain">{row.domain}</span>
      <span className="folio-tree__sub">{row.subtree}</span>
      {isSoon ? <span className="folio-tree__tag">soon</span> : null}
      {!isSoon ? (
        <span aria-hidden className="folio-tree__arrow">
          →
        </span>
      ) : null}
    </>
  )
}

function Row({ row, index }: { row: Row, index: number }) {
  const isSoon = row.status === "soon";

  return (
    <li
      key={row.domain}
      className={
        "folio-tree__node folio-fade-up" +
        (isSoon ? " folio-tree__node--soon" : "")
      }
      style={{ animationDelay: `${160 + index * 45}ms` }}
    >
      {row.status === "live" ? (
        <Link
          href={row.href}
          className="folio-tree__row"
          aria-label={`${row.domain}${row.subtree} — connect`}
        >
          <InnerRow row={row} index={index} />
        </Link>
      ) : (
        <span
          className="folio-tree__row"
          aria-label={`${row.domain}${row.subtree} — coming soon`}
          aria-disabled="true"
          role="link"
        >
          <InnerRow row={row} index={index} />
        </span>
      )}
    </li>
  )
}

export function ConnectorTree() {
  return (
    <div className="folio-tree">
      <p className="folio-tree__root">/mnt/foliofs.dev/</p>
      <ul className="folio-tree__list" role="tree">
        {ROWS.map((row, index) => (
          <Row key={index} row={row} index={index} />
        ))}
      </ul>
 
    </div>
  );
}