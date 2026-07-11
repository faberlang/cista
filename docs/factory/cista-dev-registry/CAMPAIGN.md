# Campaign: cista.dev Public Registry

Date: 2026-07-11
Status: draft — architecture discovery selected

## Summary

Build `cista.dev` as the public registry and discovery service for Faber
applications and libraries. The existing Cista client already proves immutable
filesystem publishing and a hermetic authenticated HTTPS archive contract; this
campaign owns the real server, website, operations, trust, and public product.

## Desired End State

- Authenticated publishers can publish immutable package versions.
- Anyone can discover, inspect, and fetch public packages safely.
- Package identity, archive containment, checksums, ownership, yanking, and
  audit history are explicit and fail closed.
- The registry website presents useful package documentation and provenance.
- Operations include backups, restore drills, abuse response, observability,
  rate limits, and a security disclosure path.

## Current State

| Track | State | Next action |
| --- | --- | --- |
| CLI client | HTTP/auth/archive contract closed hermetically. | Freeze server-facing API contract from tests. |
| Live service | No deployed server selected. | Architecture and threat-model discovery. |
| Website | No dedicated design artifact. | CPO design/product brief after domain model. |
| Identity | Bearer client exists; issuer/account model unselected. | Security and product decision. |
| Storage | Immutable archive semantics exist client-side. | Select metadata DB, object storage, checksums, retention. |
| Trust/abuse | Unspecified. | CSO threat model and moderation/yank policy. |

## Campaign Path

### Stage 0 — Contract And Threat Model

- Source: existing Phase F/G client tests and path-safety theme
- Outputs: API inventory, package state machine, trust boundaries, threat model,
  abuse cases, privacy/data-retention requirements
- Gate: server can be designed without weakening immutable/fail-closed client
  semantics
- Batching: discovery-first
- Lowers to: delivery

### Stage 1 — Reference Architecture

- Compare established registry patterns: immutable blobs, transactional
  metadata, object storage, checksums/signatures, ownership grants, scoped
  tokens, indexes, yanks without deletion, CDN/download separation, audit logs,
  malware/abuse quarantine, backup and restore.
- Gate: architecture decision record with scale assumptions, cost envelope,
  failure modes, local-dev story, and migration strategy
- Batching: split-on-boundary (control plane, blob plane, public web)
- Lowers to: delivery

### Stage 2 — Registry Core

- Package metadata, ownership, scoped auth, immutable publish reservation,
  archive upload/fetch, checksums, conflict semantics, yank state, audit events
- Gate: hermetic client contract plus server integration suite; concurrent
  publish and path/archive attacks fail closed
- Batching: batch-by-default after publish/fetch vertical slice
- Lowers to: factory

### Stage 3 — Public Discovery Website

- Search/browse, package/version pages, README/docs rendering, owners,
  provenance/checksums, install command, yank/security state, ecosystem links
- Gate: safe rendering, accessibility, SEO, responsive design, no active-content
  injection from package metadata
- Batching: discovery-first visual prototype, then batch
- Lowers to: delivery, then factory

### Stage 4 — Operations And Security

- Rate limits, observability, alerting, backups, restore drill, token rotation,
  disclosure/abuse workflow, moderation, dependency/secret scanning decisions
- Gate: CSO sign-off on threat-model coverage and recovery evidence
- Batching: split-on-boundary at production credentials/external services

### Stage 5 — Live Proof And Launch

- Run the existing operator-gated disposable-package proof against the deployed
  service, then stage a limited public launch.
- Gate: explicit operator authorization, isolated credentials, TLS, immutable
  conflict proof, inventory match, cleanup evidence
- Lowers to: factory/ops

## Dependency Rules

- Do not change the client contract merely to simplify server implementation;
  evolve both through a versioned API decision.
- Package deletion is not ordinary product behavior. Prefer yank/quarantine and
  auditable retention; destructive/legal workflows require explicit policy.
- Never execute live publish tests without the existing environment and
  disposable-identity authorization gate.
- DNS, cloud resources, credentials, email, payments, and production deployment
  require operator approval.

## First Useful Milestones

1. API/state-machine and threat-model packet.
2. Local registry vertical slice compatible with the current CLI.
3. Searchable package-page prototype backed by fixture metadata.
4. Staging deployment and disposable live proof.

## Validation

- Existing Cista client tests remain green.
- Concurrent same-version publishes produce exactly one immutable winner.
- Archive traversal, symlink, nested-root, oversized upload, token leakage,
  malicious metadata/README, and unauthorized ownership changes fail closed.
- Restore drill proves metadata/blob consistency.

## Stop Conditions

Pause before selecting paid infrastructure, creating cloud resources, changing
DNS, issuing production credentials, collecting personal data, accepting public
uploads, or publishing terms/moderation promises.
