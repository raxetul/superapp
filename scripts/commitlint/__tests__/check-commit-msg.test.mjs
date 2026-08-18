import { test } from "node:test";
import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { mkdtempSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { firstSubjectLine, lintCommitMessage } from "../check-commit-msg.mjs";

const SCRIPT = fileURLToPath(new URL("../check-commit-msg.mjs", import.meta.url));

test("firstSubjectLine skips comment and blank lines", () => {
  const raw = "\n# Please enter the commit message\n\nfeat(ci): add pipeline\n\nbody text\n";
  assert.equal(firstSubjectLine(raw), "feat(ci): add pipeline");
});

test("accepts a conforming subject with scope", () => {
  const r = lintCommitMessage("feat(ci): add root CI pipeline");
  assert.equal(r.ok, true);
});

test("accepts a conforming subject with breaking-change marker", () => {
  const r = lintCommitMessage("feat(backend)!: drop legacy auth endpoint");
  assert.equal(r.ok, true);
});

test("accepts a conforming subject without scope", () => {
  const r = lintCommitMessage("chore: mark P10 complete");
  assert.equal(r.ok, true);
});

test("rejects an unknown type", () => {
  const r = lintCommitMessage("update: tweak config");
  assert.equal(r.ok, false);
  assert.match(r.reason, /type one of/);
});

test("rejects a missing colon/subject", () => {
  const r = lintCommitMessage("fixed the login bug");
  assert.equal(r.ok, false);
});

test("rejects an empty message", () => {
  const r = lintCommitMessage("\n\n# only a comment\n");
  assert.equal(r.ok, false);
  assert.match(r.reason, /no subject/);
});

test("exempts a merge commit", () => {
  const r = lintCommitMessage("Merge branch 'main' into feat/p10");
  assert.equal(r.ok, true);
});

test("exempts a revert commit", () => {
  const r = lintCommitMessage('Revert "feat(ci): add root CI pipeline"');
  assert.equal(r.ok, true);
});

test("rejects a subject over the length budget", () => {
  const long = "feat(ci): " + "x".repeat(120);
  const r = lintCommitMessage(long);
  assert.equal(r.ok, false);
});

test("CLI: exits 0 for a conforming message file", () => {
  const dir = mkdtempSync(path.join(tmpdir(), "commitlint-"));
  const file = path.join(dir, "MSG");
  writeFileSync(file, "feat(ci): add root CI pipeline\n");
  assert.doesNotThrow(() => execFileSync("node", [SCRIPT, file], { stdio: "pipe" }));
});

test("CLI: exits non-zero for a non-conforming message file", () => {
  const dir = mkdtempSync(path.join(tmpdir(), "commitlint-"));
  const file = path.join(dir, "MSG");
  writeFileSync(file, "made some changes\n");
  assert.throws(() => execFileSync("node", [SCRIPT, file], { stdio: "pipe" }));
});

test("CLI: --range lints every commit subject in the range", () => {
  const dir = mkdtempSync(path.join(tmpdir(), "commitlint-repo-"));
  const run = (cmd) => execFileSync("bash", ["-c", cmd], { cwd: dir, stdio: "pipe" });
  run("git init -q -b main");
  run("git config user.email test@example.com");
  run("git config user.name test");
  // --no-verify: this fixture repo may inherit an ambient global commit-msg
  // hook; these commits are fixtures for the --range reader, not a hook test.
  run("git commit -q --no-verify --allow-empty -m 'feat: base commit'");
  run("git commit -q --no-verify --allow-empty -m 'not conventional'");
  let threw = false;
  try {
    execFileSync("node", [SCRIPT, "--range", "HEAD~1..HEAD"], { cwd: dir, stdio: "pipe" });
  } catch {
    threw = true;
  }
  assert.equal(threw, true);
});
