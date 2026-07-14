# cista.dev Stage 0 Threat Model

Status: draft — Stage 0 of `CAMPAIGN.md` (Contract and Threat Model)
Owner: head-cso (advisory) → reports To mind
Source evidence: `cista/src/**` as of HEAD `599422f`; client contract is the
floor the server must not weaken.

## Scope

Threat model for the **cista.dev public registry server** (not yet deployed)
and the trust contract between the existing Cista client and that server. The
client is already hermetic and fail-closed; this model defines what the server
must guarantee so the client's invariants survive contact with a public,
multi-tenant, untrusted-publisher environment.

Client invariants the server MUST NOT weaken:
- HTTPS-only for bearer credentials (`registry_http.rs` refuses HTTP+token).
- Bare origin, no userinfo/path/query (`credentials.rs`, `registry_http.rs`).
- 64 MiB response cap (`registry_http.rs::MAX_RESPONSE_BYTES`).
- Package identity validated (`shared::validate_identity`, `validate_store_segment`).
- Local registry immutability: existing destination = error (`registry.rs::publish`).
- Archive unpack rejects non-file/non-dir entries and path escape
  (`registry.rs::unpack_archive`, `tar::Archive::unpack_in`).
- Credential file 0600, atomic write+replace (`credentials.rs`).

## Assets

| Asset | Sensitivity |
| --- | --- |
| Publisher bearer tokens | high — full publish authority per origin |
| Published immutable archives | high — integrity, availability |
| Package metadata (manifest, README, owners) | medium — renders on website |
| Ownership map (package → owners) | high — controls takeover |
| Audit history | medium — forensics |
| Client filesystem (`$CISTAE_HOME`) | high — unpacked code/binaries run locally |

## Threats (prioritized)

### T1 — Dependency confusion / name squatting  (severity: high)

**Scenario:** attacker publishes `norma:json` (or a victim's package name) on
cista.dev before or alongside the legitimate owner. A project resolving
`norma:json` fetches the malicious archive and runs it (`cista run`, or
`faber build` compiling embedded native target).

**Evidence / gap:** client has **no checksum, signature, or provenance** field
(`rg checksum|sha256|signature` in `cista/src` → empty). Identity is name+version
string only. Fetch trusts TLS + manifest name/version match — both satisfiable
by an attacker who registers the name first.

**Impact:** arbitrary native code execution on install/run of a victim project.

**Server controls:** verified ownership, name reservation/transfer policy,
first-publisher claims with challenge, namespace scoping, publish-time
malware/secret scan, yank/quarantine, advisory feed.

**Client controls (future):** lockfile integrity field (checksum/signature),
trusted-owners allowlist, scope-restricted tokens.

**Owner:** head-cso → mind files Hand (client integrity field) + server design
task (ownership/namespace). **Open question:** registry-of-record vs. mirror
model? (affects name authority).

### T2 — Immutable publish race / non-atomic server  (severity: high)

**Scenario:** two concurrent PUTs for `pkg@1.0.0`. Client enforces "directory
already exists" locally; the server must enforce "exactly one immutable
winner" under concurrency. A server that writes-then-checks, or that retries
on conflict, can let a later publish overwrite or interleave archive bytes.

**Evidence:** client `publish_package` is a single PUT to
`/v1/packages/{name}/{version}/archive` with no precondition header (no
`If-None-Match`, no ETag, no CAS token). Server must treat the pair
`(name, version)` as a primary key with insert-only semantics.

**Impact:** immutable-archive invariant broken → supply chain integrity loss,
non-reproducible builds.

**Server controls:** insert-only metadata row keyed `(name, version)`,
atomic blob reservation (create-if-absent), conflict = 409 with the winner's
checksum, audit event for the loser. No PUT-overwrite path. Yank mutates a
state flag, never the blob.

**Owner:** server design (Stage 2). **Open question:** reservation vs. commit
two-phase (reserve name@version, then upload blob)?

### T3 — Ownership takeover / token scope  (severity: high)

**Scenario:** a leaked or over-scoped token lets an attacker add themselves as
owner, publish a malicious version, then remove the original owner. Or: a
token with no scope can yank/transfer ownership.

**Evidence:** client credential is a single bearer token per origin, no scopes
(`credentials.rs` `Credential { origin, token }`). All authenticated operations
(publish, and future yank/owner-add) use the same token. No token identity is
inspected client-side.

**Impact:** persistent compromise of a package namespace.

**Server controls:** scoped tokens (publish vs. yank vs. owner-manage),
per-package ownership grants, transfer requires existing-owner confirmation +
cool-off, mandatory 2FA for owner-add/transfer, token rotation + revocation,
audit trail for ownership mutations.

**Owner:** server identity design (Stage 1/2). **Open question:** issuer
model (cista.dev accounts vs. external OIDC) — Mind + CPO decision.

### T4 — Archive / path traversal in server-stored or server-served archives  (severity: high)

**Scenario:** a malicious publisher uploads an archive with `..` segments,
absolute paths, symlinks/hardlinks, device files, or setuid bits. The server
re-stores it without validation; a later fetch unpacks it (client
`unpack_archive`) and escapes `$CISTAE_HOME` or drops a privileged binary.

**Evidence:** client `unpack_archive` rejects non-file/non-dir entries, uses
`unpack_in` (path-escape-checked), and rejects entries requesting setuid,
setgid, sticky, or world-writable modes before cache installation. Server
should validate on ingest regardless.

**Impact:** local path escape, privilege escalation, persistence.

**Server controls (ingest):** reject entries with absolute paths, `..`,
symlinks/hardlinks/devices, oversized entries, nested-root escapes; cap total
uncompressed size; normalize modes (strip setuid/setgid/sticky); store a
canonical archive + checksum. Re-validate on fetch.

**Client controls (covered locally):** reject dangerous mode bits on unpack;
optionally re-validate against server-declared checksum when checksums exist.

**Owner:** server Stage 2 (ingest validation) + small client hardening Hand
(mode stripping in `unpack_archive`).

### T5 — Malicious metadata / README rendering  (severity: medium)

**Scenario:** package manifest/README contains HTML/JS/markdown that renders
on the cista.dev website (Stage 3). Stored XSS, link to malware, misleading
"install" command, or metadata that breaks the page.

**Evidence:** client `manifest.rs` has no rendering; risk is server/website.
Campaign Stage 3 gate already names "no active-content injection from package
metadata."

**Impact:** credential theft via XSS on cista.dev, phishing, reputational.

**Server controls:** server-side sanitization, strict allowlist render,
sandboxed iframe / no inline JS, Content-Security-Policy, per-package
README rendered in isolation, link rel="noopener noreferrer".

**Owner:** website design (Stage 3).

### T6 — Token leakage / logging / client storage  (severity: medium)

**Scenario:** server logs Authorization headers; client writes token to a
world-readable file on a non-Unix host; URL Referrer leaks token; error
messages echo the token.

**Evidence:** client is solid — 0600 file, atomic write, refuses HTTP+token,
debug impl is `finish_non_exhaustive()` (`registry_http.rs`) — does not print
the token. **Server** must match: never log `Authorization`, redact in error
responses, short-lived + refreshable tokens, no token in URL/path/query.

**Impact:** token theft → T3.

**Server controls:** token redaction in logs, structured logging with
allowlist, token rotation + short TTL.

**Owner:** server ops (Stage 4).

### T7 — Abuse / rate limits / resource exhaustion  (severity: medium)

**Scenario:** publisher floods versions, names, or huge archives; fetcher
scrapes; attacker publishes many names to squat. No rate limits → storage
exhaustion, metadata bloat, discovery poisoning.

**Evidence:** client enforces 64 MiB response cap and identity validation;
**server** has no rate/size policy yet (no server exists).

**Server controls:** per-token publish rate + version-count caps, max archive
size, max entries, name-squat policy, fetch rate limits + CDN, abuse
quarantine, audit alerts.

**Owner:** server ops (Stage 4).

### T8 — Audit / backup / restore  (severity: medium)

**Scenario:** incident requires yanking a version, revoking a token, restoring
metadata from backup. If audit history is mutable or backups don't cover
ownership + blob state, forensics and recovery fail.

**Evidence:** campaign Desired End State names audit history and restore
drills; no implementation yet. Client has no audit surface (local only).

**Server controls:** append-only audit log (publish, yank, owner change,
token issue/revoke), immutable blob retention (yank ≠ delete), periodic
backup + restore drill evidence, checksum-verified restore.

**Owner:** server ops (Stage 4). **Gate:** campaign Stage 4 requires "CSO
sign-off on threat-model coverage and recovery evidence."

## Prioritized recommendations To mind

| # | Action | Severity | file_to | effort |
| --- | --- | --- | --- | --- |
| 1 | Freeze server API contract: insert-only `(name,version)`, 409-on-conflict, no PUT-overwrite (T2) | high | decision_only (Stage 1 ADR) | S |
| 2 | Ownership + scoped-token model decision (T3) | high | decision_only (Mind + CPO) | S |
| 3 | Server archive ingest validation spec (T4) — absolute/`..`/symlink/device/mode/size | high | hand (cista server Stage 2) | M |
| 4 | Client integrity field: checksum (+ future signature) in lockfile + manifest (T1) | high | hand (cista client) | M |
| 5 | Client: reject setuid/setgid/sticky + world-writable on `unpack_archive` (T4) | medium | done (cista client) | S |
| 6 | Token redaction + audit-log schema (T6, T8) | medium | hand (server Stage 4) | M |
| 7 | Rate-limit + abuse/quarantine policy (T7) | medium | decision_only (Stage 4) | S |
| 8 | Website render sanitization + CSP (T5) | medium | hand (Stage 3 website) | M |

## Open questions for Mind

- Registry-of-record vs. mirror? (affects T1 name authority and T2 conflict
  semantics)
- Identity issuer: cista.dev native accounts vs. external OIDC? (T3, T6)
- Package deletion policy: confirm yank/quarantine-only, no hard delete except
  legal/PII workflow? (campaign already states this — confirm as threat-model
  invariant)
- Is a client-side trusted-owners allowlist desired before public launch? (T1)

## Validation ties to CAMPAIGN.md

This model maps to the campaign's Validation section ("Archive traversal,
symlink, nested-root, oversized upload, token leakage, malicious
metadata/README, and unauthorized ownership changes fail closed") — each
threat above should become a hermetic server test in Stage 2.
