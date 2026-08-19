#!/usr/bin/env node
/**
 * Conventional Commits linter (TR-10-008, realizes TR-01-005).
 *
 * Validates a commit message's subject line against
 * `type(scope)!: subject`, `type` being one of the Conventional Commits
 * types below. Used two ways:
 *   - as the `commit-msg` git hook (scripts/hooks/commit-msg), which passes
 *     the path git gives it (the message as the author is about to write it);
 *   - as a CI check (`--range <rev>..<rev>`) which lints every commit
 *     subject in a push/PR range via `git log`.
 *
 * Merge, revert, and fixup!/squash! commits (git- or workflow-generated) are
 * exempt — they don't carry a human-chosen type/scope.
 */
import { readFileSync } from "node:fs";
import { execFileSync } from "node:child_process";
import { fileURLToPath } from "node:url";

export const TYPES = [
  "feat",
  "fix",
  "docs",
  "style",
  "refactor",
  "perf",
  "test",
  "build",
  "ci",
  "chore",
  "revert",
];

const HEADER_RE = new RegExp(`^(${TYPES.join("|")})(\\([a-z0-9./-]+\\))?(!)?: .{1,88}$`);
const EXEMPT_RE = /^(Merge |Revert "|fixup!|squash!)/;

/** The first non-blank, non-`#`-comment line of a raw commit message. */
export function firstSubjectLine(rawMessage) {
  for (const line of rawMessage.split(/\r?\n/)) {
    if (line.trim() === "" || line.startsWith("#")) continue;
    return line;
  }
  return "";
}

/** Lint a raw commit message; returns `{ ok, subject, reason }`. */
export function lintCommitMessage(rawMessage) {
  const subject = firstSubjectLine(rawMessage);
  if (subject === "") {
    return { ok: false, subject, reason: "commit message has no subject line" };
  }
  if (EXEMPT_RE.test(subject)) {
    return { ok: true, subject, reason: "merge/revert/fixup commit — exempt" };
  }
  if (!HEADER_RE.test(subject)) {
    return {
      ok: false,
      subject,
      reason:
        `subject line must match "type(scope): subject" with type one of ` +
        `${TYPES.join(", ")} (Conventional Commits). Got: "${subject}"`,
    };
  }
  return { ok: true, subject, reason: "ok" };
}

/** Lint every commit subject in `range` (a `git log`-compatible rev range). */
export function lintRange(range) {
  const out = execFileSync("git", ["log", "--format=%H%x00%s", range], {
    encoding: "utf8",
  });
  const results = [];
  for (const line of out.split("\n")) {
    if (line.trim() === "") continue;
    const [hash, subject] = line.split("\x00");
    const result = lintCommitMessage(subject ?? "");
    results.push({ hash, ...result });
  }
  return results;
}

function main(argv) {
  const rangeIdx = argv.indexOf("--range");
  if (rangeIdx !== -1) {
    const range = argv[rangeIdx + 1];
    if (!range) {
      console.error("usage: check-commit-msg.mjs --range <rev>..<rev>");
      process.exit(2);
    }
    const results = lintRange(range);
    const failures = results.filter((r) => !r.ok);
    for (const r of results) {
      console.log(`${r.ok ? "ok  " : "FAIL"} : ${r.hash.slice(0, 12)} ${r.subject}`);
      if (!r.ok) console.error(`        ${r.reason}`);
    }
    process.exit(failures.length > 0 ? 1 : 0);
  }

  const file = argv[0];
  if (!file) {
    console.error("usage: check-commit-msg.mjs <commit-msg-file> | --range <rev>..<rev>");
    process.exit(2);
  }
  const raw = readFileSync(file, "utf8");
  const result = lintCommitMessage(raw);
  if (!result.ok) {
    console.error(`commit-msg: ${result.reason}`);
    console.error("Conventional Commits: https://www.conventionalcommits.org/");
    process.exit(1);
  }
  process.exit(0);
}

if (import.meta.url === `file://${process.argv[1]}`) {
  main(process.argv.slice(2));
}
