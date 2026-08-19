import { test } from "node:test";
import assert from "node:assert/strict";
import { deriveVersion } from "../derive-version.mjs";

test("a semver tag push publishes the bare version", () => {
  const v = deriveVersion({ ref: "refs/tags/v1.2.3", refName: "v1.2.3", sha: "a".repeat(40) });
  assert.equal(v, "1.2.3");
});

test("a semver-with-prerelease tag keeps the prerelease suffix", () => {
  const v = deriveVersion({
    ref: "refs/tags/v1.2.3-rc.1",
    refName: "v1.2.3-rc.1",
    sha: "a".repeat(40),
  });
  assert.equal(v, "1.2.3-rc.1");
});

test("a non-semver tag publishes verbatim", () => {
  const v = deriveVersion({ ref: "refs/tags/nightly", refName: "nightly", sha: "a".repeat(40) });
  assert.equal(v, "nightly");
});

test("a branch push publishes <branch>-<short-sha>", () => {
  const v = deriveVersion({
    ref: "refs/heads/main",
    refName: "main",
    sha: "0123456789abcdef0123456789abcdef01234567",
  });
  assert.equal(v, "main-0123456789ab");
});

test("a branch name with slashes/special chars is sanitized", () => {
  const v = deriveVersion({
    ref: "refs/heads/feat/p10-testing-docker-deploy",
    refName: "feat/p10-testing-docker-deploy",
    sha: "b".repeat(40),
  });
  assert.equal(v, "feat-p10-testing-docker-deploy-" + "b".repeat(12));
});

test("throws when a branch build is missing refName or sha", () => {
  assert.throws(() => deriveVersion({ ref: "refs/heads/main", refName: "main", sha: "" }));
  assert.throws(() => deriveVersion({ ref: "refs/heads/main", refName: "", sha: "a".repeat(40) }));
});

test("throws when sha is too short to be unique", () => {
  assert.throws(() => deriveVersion({ ref: "refs/heads/main", refName: "main", sha: "abc123" }));
});
