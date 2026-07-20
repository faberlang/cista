# Test Decomposition Analysis — cista

## Summary

| Lens | Findings | Critical | High | Medium | Low |
|---|---|---|---|---|---|
| Coverage gaps | 26 | 3 | 5 | 8 | 10 |
| Missing negatives | 28 | 4 | 10 | 10 | 4 |
| Redundancy | 9 | 0 | 3 | 4 | 2 |
| Setup complexity | 6 | 0 | 2 | 3 | 1 |

**Test inventory:**
- 13 test files covering 13 source files (inline `#[cfg(test)]` modules)
- 1 integration test (`tests/hygiene.rs`)
- 23 source files with **no dedicated test file at all**
- ~94 test functions total across all test files (approximate, based on `#[test]` count)

---

## Top 10 recommendations (ranked by impact)

1. **[Critical] `src/commands/rust_target.rs`: No tests for cargo/rustc build logic** — `build_rust_artifact`, `verify_target_build`, `rust_host_triple`, `rustc_version`, and `contained_cargo_manifest` shell out to external toolchains. None of these functions have tests. A missing rust toolchain, a cargo build failure, or a misidentified host triple could silently break install, yet no test exercises these paths. The `run_cargo` function also constructs command-line args (including the subcommand-first ordering workaround at line 111), which is untested.

2. **[Critical] `src/manifest.rs`: Manifest parsing has zero dedicated tests** — `read_manifest` and `read_meta_manifest` handle file I/O, TOML deserialization, and schema validation (`deny_unknown_fields`). They are the entry point for all package validation, used by `install`, `check`, `inspect`, `publish`, and `fetch`. Every consumer tests them indirectly, but no test directly validates:
   - Malformed TOML in `read_manifest` (garbage bytes)
   - Missing required fields (e.g., `source.package`, `target.language`)
   - Unknown fields rejected (`deny_unknown_fields`)
   - `read_meta_manifest` correctly distinguishing meta from ordinary manifests
   - `manifest_path` with `None` vs `Some` argument
   
   **Bug risk**: A regression in TOML parsing or schema validation would surface only through downstream tests, making the root cause harder to isolate.

3. **[Critical] `src/registry_http.rs`: No tests for HTTP transport errors (connection refused, timeout, 5xx, 4xx non-401, malformed body)** — The `HermeticRegistry` mock only returns 401, 404, and 409. The real `UreqTransport` handles `ureq` errors (DNS resolution failure, connection refused, TLS handshake failure, timeout). None of these are tested. The `read_response` function has a `MAX_RESPONSE_BYTES` guard — no test exercises the overflow path. Similarly, no test covers a 200 response with a garbage body (not a valid tar archive), though `unpack_archive` would catch that later. The `RegistryHttpClient::new` function also rejects empty bearer tokens — that path is tested in `credentials_test.rs` but not in the registry HTTP module itself.

4. **[Critical] `src/commands/fetch.rs`: No command-level test for remote fetch (registry_url path)** — `fetch.rs` routes to `registry::fetch_remote_to_cache` when `--registry-url` is set. The `registry_test.rs` tests only the local registry path. No test exercises the remote fetch code path, including the `authenticated_client` call, the `publish_remote`/`fetch_remote_to_cache` functions, or the error path when no credentials are stored.

5. **[High] `src/commands/inspect.rs`: No tests for the inspection logic** — `inspect.rs` resolves package-or-path, reads manifests, and prints summaries. The path-based inspection branch (`inspect_path`), the installed-package branch, and the missing-manifest error all lack tests. This is a user-facing command; regressions would be highly visible.

6. **[High] `src/commands/publish.rs`: No command-level test for remote publish** — `publish.rs` calls `registry::publish_remote` when `--registry-url` is set. The `registry_test.rs` only exercises the local registry publish path. Remote publish (including authentication, HTTP transport, and the `--registry-url` / `--registry` exclusivity check) has no test.

7. **[High] `src/commands/login.rs` and `src/commands/logout.rs`: No CLI-level tests** — Both commands interact with the environment (`CISTA_REGISTRY_TOKEN`) and the credentials file. No test verifies: missing token env var, successful login→logout round trip, or the case where `remove` returns `false` (no credentials to remove).

8. **[High] `src/commands/remove.rs` remove_test: Missing test for directory deletion failure** — `remove.rs` handles `fs::remove_dir_all` errors and `remove_empty_name_dir` edge cases (`NotFound`, `DirectoryNotEmpty`). No test injects a permission error or a busy directory to verify the error path is surfaced correctly.

9. **[High] `src/commands/shared.rs` shared_test: No test for `validate_package` with `verify_build=true`** — The `shared::validate_package` function has an optional `verify_build` parameter that calls `rust_target::verify_target_build`. None of the `shared_test.rs` tests set this to `true`. The build verification path (which shells out to `cargo check`) is completely untested.

10. **[High] Test boilerplate: 9 separate `temp_dir`/`fixture`/`temp_root` implementations** — Every test file reimplements its own temporary directory creation with a nonce-based naming convention. This is not just code duplication (see Redundancy section below) — it's also inconsistent: some use `SystemTime::now().duration_since(UNIX_EPOCH).as_nanos()`, others use process ID directly, and naming prefixes vary. A shared `test_support` module would reduce the maintenance surface and make test cleanup more predictable.

---

## Per-file details

### `src/store_test.rs` → tests `src/store.rs`
**Line count**: 434 (test), 532 (source)

- **Coverage gaps**:
  - `store_root()`: Not tested when `HOME` is missing and `CISTAE_HOME` is not set (`Medium`). The error-branch at line 29-32 has no coverage.
  - `normalize_path()`: Not directly tested (`Low`). Only exercised through other functions.
  - `list_installed()`: No test for I/O errors during directory enumeration (permission denied, corrupted directory) (`Medium`).
  - `split_package_id()`: Not directly tested (`Low`). No test for empty string or `@` at boundaries.
  - `list_package_files()`: No test for empty directory, nested symlinks in subdirectories, or path stripping edge cases (`Low`).

- **Missing negatives**:
  - 16 tests: 8 negative (errors/rejections), 3 positive, 5 unix-only security. Well-balanced.
  - **Missing**: No test for `store_root` when HOME is unavailable.
  - **Missing**: No test for `validate_store_identity` on empty strings (though the function handles it at line 122-130). This is covered indirectly in `shared_test.rs`.
  - `High` quality: Thoroughly tests transaction directory filtering, identity validation, and symlink/special-entry rejection.

- **Redundancy**:
  - `write_target_manifest` helper is duplicated in `remove_test.rs` and similar in `run_test.rs` (`Medium`).
  - `installed_package` helper creates a fixed fixture — shared across multiple tests, but the pattern is simple (`Low`).

- **Setup complexity**:
  - Most tests require 5-10 lines of setup (create temp dir, write a fixture, call function, assert, cleanup).
  - `verified_resolution_rejects_mismatched_root_meta_manifest_identity` (17 lines) — slightly heavier due to manifest content creation but appropriate for what it tests (`Low`).

### `src/registry_http_test.rs` → tests `src/registry_http.rs`
**Line count**: 107 (test), 166 (source)

- **Coverage gaps**:
  - `UreqTransport::execute()`: **Not tested at all** (`Critical`). The mock `HermeticRegistry` replaces the transport layer. The real `ureq` transport — including connection refused, DNS failure, TLS errors, and timeout — has zero test coverage.
  - `read_response()`: Not tested (`Critical`). The `MAX_RESPONSE_BYTES` limit (64 MiB) has no test exercising `bytes.len() > MAX_RESPONSE_BYTES`. The `response.body_mut().as_reader().take()` pattern is untested. The error path for `read_to_end` failure is untested.
  - `package_path()`: Tested indirectly through `fetch_package`/`publish_package`. No test for invalid package/version segments passed to `validate_package_identity`.
  - `RegistryHttpClient::new`: No test for the empty-token rejection at line 103 (`Medium`).

- **Missing negatives**:
  - 5 tests: 3 negative, 1 positive, 1 security. Decent coverage for construction-time validation.
  - **Missing** (`Critical`): `UreqTransport` error paths — connection refused, TLS failure, DNS resolution failure, timeout.
  - **Missing** (`Critical`): HTTP 500, 503, 429 response codes.
  - **Missing** (`Critical`): Malformed response body (garbage bytes, truncated tar).
  - **Missing** (`Critical`): Response exceeding `MAX_RESPONSE_BYTES`.
  - **Missing** (`High`): Empty bearer token rejection in `RegistryHttpClient::new`.

- **Redundancy**:
  - The `HermeticRegistry` mock is clean and non-redundant (`Low`).

- **Setup complexity**:
  - All tests follow the same pattern: construct client, call method, assert. Setup is 3-10 lines. `Low`.

### `src/credentials_test.rs` → tests `src/credentials.rs`
**Line count**: 67 (test), 204 (source)

- **Coverage gaps**:
  - `default_path()`: Not tested when `HOME` is not set (`Medium`).
  - `remove()`: The `false` return (origin not found) is untested (`Medium`). The round-trip test only tests the `true` case.
  - `token()`: The `None` return for missing origin is tested in the round-trip test. `Low` risk.
  - `read()`: Not tested with corrupted/malformed TOML at line 99 (`High`). A credentials file with invalid TOML would cause an uncaught parse error.
  - `verify_permissions()`: Unix path tested via the owner-only test. Non-unix path (`_path`) is vacuously OK.
  - `secure_create()`: Not directly tested; exercised through `store()`.
  - `write_and_replace()`: Only tested for the directory-replacement error case. Not tested for the normal write path independently.

- **Missing negatives**:
  - 4 tests: 2 negative, 1 positive, 1 security/unix-only. `Medium` coverage.
  - **Missing** (`High`): Corrupted credentials file (invalid TOML).
  - **Missing** (`Medium`): `remove()` when origin not found.
  - **Missing** (`Medium`): Secure create failure (O_CREATE | O_EXCL collision).
  - **Missing** (`Medium`): `write_and_replace` normal path (only the failure path is tested).

- **Redundancy**:
  - The `temp_path()` function uses `std::thread::current().name()`, which is unusual — most other files use a nanos nonce. `Low`.

- **Setup complexity**:
  - All tests are compact (5-15 lines). `Low`.

### `src/faber_lock_test.rs` → tests `src/faber_lock.rs`
**Line count**: 264 (test), 347 (source)

- **Coverage gaps**:
  - `read_lock()`: No test for malformed TOML at line 90 (`Medium`).
  - `write_lock()`: Thoroughly tested for fault injection at every stage (before create, write, rename, cleanup, sync). `High` quality.
  - `upsert_package()`: Tested for single-name replacement and duplicate removal. No test for empty lock insertion.
  - `locked_from_install()`: **Not tested directly** (`High`). Only exercised through the install integration tests. No unit test verifies the path assembly, artifact construction, or the `has_artifact` flag logic.
  - `absolute_display()`: Not tested directly (`Low`).
  - `lock_path()`: Not tested directly (`Low`).
  - `sync_directory()`: Not tested; only exercised through `write_and_replace` (`Low`).

- **Missing negatives**:
  - 8 tests: 5 negative (injected faults), 2 positive. Extremely thorough for the atomic-write path.
  - **Missing** (`Medium`): `read_lock` with malformed TOML.
  - **Missing** (`High`): `locked_from_install` — no dedicated test.

- **Redundancy**:
  - The five fault-injection tests (`write_lock_create_failure`, `write_lock_write_failure`, `write_lock_rename_failure`, `write_lock_cleanup_failure`, `write_lock_parent_sync_failure`) follow an identical structure:
    1. Create temp dir
    2. Seed existing lock
    3. Inject fault
    4. Call `write_lock`, expect error
    5. Assert existing lock preserved
    6. Assert temp files absent/present (cleanup test differs)
    7. Cleanup
    
    `High`: These could be parameterized into a single table-driven test or macro. The only difference between the first four is the fault variant and the presence/absence of a temp file after failure.

- **Setup complexity**:
  - The `seed_existing_lock`, `package`, `replacement_lock`, and `temporary_lock_files` helpers are reused across all tests. Well-factored (`Low`).

### `src/cli_test.rs` → tests `src/cli.rs`
**Line count**: 40 (test), 360 (source)

- **Coverage gaps**:
  - Only tests the `install` subcommand (`Critical`). 18 other commands (`init`, `check`, `inspect`, `metadata`, `graph`, `resolve`, `fetch`, `run`, `remove`, `update`, `cache`, `package`, `runtime`, `target`, `publish`, `yank`, `login`, `logout`, `doctor`) have no CLI parsing tests at all.
  - `InstallArgs`: Tests --path vs name@version exclusivity. Does not test `--path`/`--package` conflicts with other flags like `--verify-target-build`.
  - `RegistryAuthArgs`: Not tested. The combination of `--registry-url` and `--token-env` defaults, plus the env-var-based token fetching in `login.rs`, is untested at the CLI layer.
  - `PublishArgs`: The `--registry`/`--registry-url` conflict is untested here (it's tested in `registry_test.rs` at line 482-503).
  - `PackageArg`: The `--registry`/`--registry-url` conflict is untested.

- **Missing negatives**:
  - 2 tests total. Severely under-tested (`Critical`).
  - Only `install` is tested. Every other command's argument parsing is untested.

- **Redundancy**:
  - N/A — only 2 tests, no redundancy.

- **Setup complexity**:
  - Trivial: `try_parse_from` calls with string arrays. `Low`.

### `src/project_manifest_test.rs` → tests `src/project_manifest.rs`
**Line count**: 40 (test), 120 (source)

- **Coverage gaps**:
  - `read_project_manifest()`: Only tested via the loose-parse fallback. The strict parse (`toml::from_str::<ProjectManifest>`) at line 45 is **not exercised** when it succeeds (`Medium`). No test for the strict path working correctly.
  - `require_exact_dependency()`: **Not tested at all** (`High`). This function handles version mismatch errors and missing-dependency errors. It is critical for lockfile correctness during install.
  - `ProjectManifest` schema: `deny_unknown_fields` at line 15 — no test verifies unknown top-level keys are rejected in the strict path.

- **Missing negatives**:
  - 2 tests: 1 negative, 1 positive. Very thin coverage (`High`).
  - **Missing** (`High`): `require_exact_dependency` with version mismatch.
  - **Missing** (`High`): `require_exact_dependency` with missing dependency.
  - **Missing** (`Medium`): Missing `[package]` section.
  - **Missing** (`Medium`): Missing `package.name`.
  - **Missing** (`Medium`): Forward-compatible unknown keys in `faber.toml`.
  - **Missing** (`Medium`): Strict parse success path for `read_project_manifest`.

- **Redundancy**:
  - None — only 2 tests (`Low`).

- **Setup complexity**:
  - Both tests use inline TOML strings. Trivial. `Low`.

### `src/commands/shared_test.rs` → tests `src/commands/shared.rs`
**Line count**: 328 (test), 610 (source)

- **Coverage gaps**:
  - `validate_package()`: Only tested for manifest shape violations. The full path (with `verify_target_build=true`, interface validation, binding validation against actual `.fab` files, and Cargo.toml checks) has **no test** (`Critical`).
  - `validate_interfaces()`: Not tested directly (`High`). The function parses `.fab` files looking for `functio` declarations and builds symbol maps. No test provides an actual `.fab` file and checks symbol extraction.
  - `validate_target_paths()`: Not tested directly (`High`). The function checks `target.source` is a directory, verifies `Cargo.toml` containment, and validates `target.artifact` is a file. None of this is exercised.
  - `validate_bindings()`: Not tested directly (`Medium`). The function checks that binding symbols exist in interface files.
  - `resolve_package_path()`: Tested through `package_manifest_paths_must_be_relative_and_contained`. Good coverage.
  - `resolve_meta_dependency_path()`: **Not tested** (`High`). The function handles path resolution for meta dependencies (sibling paths, traversal beyond collection root, symlink escapes). Used only in install.
  - `acquire_store_mutation_locks()`: Tested indirectly through install/remove tests that hold locks. The lock file creation and exclusive locking are exercised. `Medium`.

- **Missing negatives**:
  - 13 tests: essentially all negative (testing rejection of invalid manifest shapes). Very thorough for shape validation.
  - **Missing** (`Critical`): `validate_package` with `verify_build=true`.
  - **Missing** (`High`): Interface validation with actual `.fab` files.
  - **Missing** (`High`): Target path validation (Cargo.toml, directory checks).
  - **Missing** (`Medium`): Binding symbol validation.
  - **Missing** (`High`): `resolve_meta_dependency_path` — any test.

- **Redundancy**:
  - The manifest shape tests each follow the pattern: create manifest, call `validate_manifest_shape`, assert specific diagnostic. Well-structured and minimal duplication given the combinatorial space (`Low`).
  - The `manifest()` helper and `buildable_manifest()` helper are reused across all tests. Good factoring.

- **Setup complexity**:
  - `package_manifest_paths_must_be_relative_and_contained` (30 lines) — creates a multi-directory fixture with external paths and validates multiple field errors. The setup is proportional to the validation surface. `Medium`.
  - `package_manifest_paths_must_resolve_symlinks_inside_package_root` (25 lines) — uses OS symlinks, requires FUSE-level behavior. Necessary complexity. `Medium`.

### `src/commands/install_test.rs` → tests `src/commands/install.rs`
**Line count**: 1218 (test), 718 (source)

- **Coverage gaps**:
  - `install_package_from_path()`: Well-covered by integration tests (`Low`).
  - `install_meta_package()`: Covered for basic expansion, containment, and failure rollback. `Medium`.
  - `prepare_package_snapshot()`: Exercised through integration tests. Not tested independently. `Low`.
  - `ensure_rust_source_install()`: **Not tested** for non-rust `target_language` rejection at line 448 (`High`).
  - `verify_install_store_disjoint()`: Covered by store-inside-source and source-inside-store tests. `Low`.
  - `resolve_project_root()`: Tested indirectly through `--project` integration tests. `Medium`.
  - `rewrite_project_lock()`: Tested in the lock-failure and platform-default tests. `Low`.

- **Missing negatives**:
  - ~18 tests: Thorough negative coverage for transaction failures, lock ordering, store overlap, identity validation, and rollback.
  - **Missing** (`High`): `ensure_rust_source_install` rejection for non-rust language.
  - **Missing** (`Medium`): `install` with `--verify-target-build=true` and a compilation failure.
  - **Missing** (`Medium`): `install` when `cargo` or `rustc` is not on PATH.
  - **Missing** (`Medium`): `install_package_from_path` when `manifest::read_meta_manifest` detects meta (the meta path) but the store overlap check fails.

- **Redundancy**:
  - `write_interfaces_only_package` helper (lines 500-528) is duplicated between this file and `registry_test.rs` (`High`).
  - `temp_root` helper (lines 12-19) follows the same nonce-nanos pattern as 8 other test files (`High`).
  - Several tests follow the pattern: create package, create store, inject fault, run install, assert error, assert cleanup. The fault injection pattern could be parameterized.

- **Setup complexity**:
  - `install_real_norma_platform_default_builds_nested_import_without_dependency` (lines 1129-1218, ~90 lines): The most complex test in the codebase. Creates a project, fake library home, installs real Norma from the workspace, runs faber check/build, and asserts output. Depends on the external workspace layout and `cargo`/`faber` availability. `High` — this is essentially an end-to-end integration test that should live in `tests/` rather than an inline unit test.
  - `meta_dependency_commit_failure_rolls_back_prior_snapshots` (lines 964-1063, ~100 lines): Creates two dependencies + meta with interleaved transaction failure injection. `High`.
  - `install_lock_post_rename_sync_failure_keeps_store_aligned_with_committed_lock` (lines 838-898, ~60 lines): Creates package, project, injects fault, verifies store-lock alignment. `Medium`.
  - `install_by_name_waits_for_store_mutation_lock_before_cache_mutation` (lines 370-418, ~50 lines): Thread synchronization test with channels and timed waits. `Medium`.

### `src/commands/registry_test.rs` → tests `src/commands/registry.rs`
**Line count**: 662 (test), 445 (source)

- **Coverage gaps**:
  - `publish_remote()`: Not tested (`Critical`). The remote publish path (line 16-27) calls `authenticated_client(origin)`, creates an archive, and publishes via HTTP. Only the local registry publish path is tested.
  - `fetch_remote_to_cache()`: Not tested (`Critical`). Same as above — only the local path.
  - `authenticated_client()`: Not tested directly (`High`). The function resolves credentials path and constructs an HTTP client. No test covers the "no credentials stored" error path.
  - `archive_directory()`: Tested for symlink rejection and normal round-trip. `Low`.
  - `unpack_archive()`: Tested for link entries, world-writable modes, setuid modes. Good coverage. `Low`.
  - `validate_archive_entry_mode()`: Tested via `unpack_archive` tests. `Low`.
  - `validate_cached_meta_package()`: Tested via meta dependency validation tests. `Low`.

- **Missing negatives**:
  - ~18 tests: Strong negative coverage for archive validation, identity mismatch, cache preservation, and symlink escapes.
  - **Missing** (`Critical`): Remote publish (publish_remote).
  - **Missing** (`Critical`): Remote fetch (fetch_remote_to_cache).
  - **Missing** (`High`): `authenticated_client` with no stored credentials.
  - **Missing** (`Medium`): Concurrent fetches of the same package (two threads fetching simultaneously).
  - **Missing** (`Medium`): Archive with deeply nested paths or path components that traverse upward.
  - **Missing** (`Medium`): `publish` when the destination already has a partial/corrupted package (not just empty).
  - **Missing** (`Low`): `verify_registry_publish_path` edge cases (registry root that doesn't exist).

- **Redundancy**:
  - `write_interfaces_only_registry_package` duplicates `write_interfaces_only_package` from `install_test.rs` with a different name but identical structure (`High`).
  - The "preserves cached package" pattern has 4 near-identical tests (`invalid_remote_archive_preserves_cached_package`, `mismatched_remote_archive_preserves_cached_package`, `invalid_remote_package_preserves_cached_package`, `invalid_local_registry_package_preserves_cached_package`, `mismatched_local_registry_package_preserves_cached_package`). Each differs only in source (remote archive vs local package) and the specific validation that fails. These could be parameterized (`Medium`).

- **Setup complexity**:
  - `publish_and_fetch_exact_package_snapshot` (lines 508-561, ~50 lines): Creates a full package with interfaces, Rust source, Cargo.toml, publishes, fetches, and verifies exact identity. The setup is proportional to testing a real publish-fetch round trip. `Medium`.
  - Several tests require canonical path resolution and thread synchronization (lock waiting). `Medium`.

### `src/commands/remove_test.rs` → tests `src/commands/remove.rs`
**Line count**: 220 (test), 63 (source)

- **Coverage gaps**:
  - `run()`: Tested for success, identity mismatch, missing identity, and lock waiting. `Medium`.
  - `remove_empty_name_dir()`: Tested for empty and non-empty cases. `Low`.
  - `fs::remove_dir_all` error path: Not tested (`Medium`).

- **Missing negatives**:
  - 5 tests: 3 negative, 2 positive. `Medium`.
  - **Missing** (`Medium`): Remove failure when `fs::remove_dir_all` returns an error (permission denied, busy directory).

- **Redundancy**:
  - `write_target_manifest` helper is identical to the one in `store_test.rs` (`Medium`).
  - `fixture` helper follows the standard temp-dir pattern (`Low`).

- **Setup complexity**:
  - `remove_waits_for_store_mutation_lock` (lines 156-220, ~65 lines): Thread synchronization test with custom lock observer. `Medium`.

### `src/commands/run_test.rs` → tests `src/commands/run.rs`
**Line count**: 166 (test), 80 (source)

- **Coverage gaps**:
  - `executable_path()`: Tested for library rejection. Not tested for wrong triple or missing artifact file (`Medium`).
  - `run()`: Tested for successful binary execution. Not tested for the case where the binary returns a non-zero exit code (lines 32-38) — the happy path covers success but the error path is untested (`Medium`).

- **Missing negatives**:
  - 3 tests: 2 negative, 1 positive. `Medium`.
  - **Missing** (`Medium`): Run failure when executable exits with non-zero status.
  - **Missing** (`Medium`): Run with wrong host triple.
  - **Missing** (`Medium`): Run with missing `target.artifact` file on disk.

- **Redundancy**:
  - `write_installed_binary_manifest` is similar to `write_target_manifest` in `store_test.rs` and `remove_test.rs` but uses a different pattern (creates via `manifest::CistaManifest` struct instead of TOML string). `Low`.

- **Setup complexity**:
  - `run_installed_binary_with_passthrough_argument` (lines 41-108, ~67 lines): Installs a binary via the `install` module, then runs it. This couples the run test to the install module — if install breaks, run tests also fail. Could be simplified by writing a pre-built binary fixture directly. `High` — unnecessary coupling.

### `src/commands/package_test.rs` → tests `src/commands/package.rs`
**Line count**: 91 (test), 135 (source)

- **Coverage gaps**:
  - `list()`: Not tested (`Medium`). The list command resolves the store root and prints installed packages.
  - `show()`: Not tested directly. Only tested via `package_inspection_does_not_resolve_reserved_cache_namespace` which exercises the rejection path. No test for successful `show`.
  - `files()`, `interfaces()`, `runtimes()`: Not tested directly. Only the cache-namespace rejection test covers them.

- **Missing negatives**:
  - 4 tests: 1 negative, 3 positive (filter functions). `Medium`.
  - **Missing** (`Medium`): Show/fetch with missing package identity.
  - **Missing** (`Low`): `list` with empty store.

- **Redundancy**:
  - `temp_root` follows the standard pattern (`Low`).

- **Setup complexity**:
  - Trivial — the filter tests are pure functions with no filesystem setup. `Low`.

### `src/commands/fs_util_test.rs` → tests `src/commands/fs_util.rs`
**Line count**: 134 (test), 432 (source)

- **Coverage gaps**:
  - `stage_directory()`: Not tested directly. Exercised through install integration tests. `Low`.
  - `discard_staged_directory()`: Not tested directly. `Low`.
  - `commit_staged_directory_transaction()`: Exercised through install tests. Not tested independently. `Low`.
  - `DirectoryReplacement::rollback()`: Not tested directly. `Low`.
  - `sync_directory_tree()`: Not tested directly. `Low`.
  - `replace_directory()`: Not tested directly. `Low`.
  - `copy_dir_new()`: Not tested directly. `Low`.
  - `resolve_path_against_existing_parent()`: Not tested directly. `Low`.

- **Missing negatives**:
  - 6 tests: 3 negative, 2 positive, 1 unix-only. `Medium`.
  - **Missing** (`Medium`): Copy failure when source directory contains an unreadable file.
  - **Missing** (`Medium`): `copy_dir_new` when destination already exists.

- **Redundancy**:
  - All tests follow the same pattern: create temp dir, set up fixtures, call function, assert, cleanup. `Low`.

- **Setup complexity**:
  - Most tests use inline directory creation with 5-10 lines of setup. `Low`.

### Source files with NO test files

| File | Lines | Risk | Notes |
|---|---|---|---|
| `src/bin/cista.rs` | 18 | Low | Thin binary entry point. |
| `src/lib.rs` | 25 | Low | Module declarations only. |
| `src/manifest.rs` | 206 | **Critical** | `read_manifest`, `read_meta_manifest` — manifest parsing has zero dedicated tests. See recommendation #2. |
| `src/package.rs` | 17 | Low | Trivial wrapper struct. |
| `src/resolver.rs` | 18 | Low | Trivial wrapper struct. |
| `src/runtime.rs` | 25 | Low | Trivial wrapper struct. |
| `src/target.rs` | 17 | Low | Trivial wrapper struct. |
| `src/commands/cache.rs` | 7 | Low | Stub routing to `staged`. |
| `src/commands/check.rs` | 22 | Medium | Routes to `shared::validate_package`. No CLI-level test. |
| `src/commands/doctor.rs` | 7 | Low | Stub. |
| `src/commands/graph.rs` | 7 | Low | Stub. |
| `src/commands/init.rs` | 7 | Low | Stub. |
| `src/commands/inspect.rs` | 60 | **High** | Has real logic for path/package resolution and manifest display. No tests. See recommendation #5. |
| `src/commands/login.rs` | 16 | High | Token-from-env, credentials storage. No tests. |
| `src/commands/logout.rs` | 15 | High | Credential removal. No tests. |
| `src/commands/metadata.rs` | 7 | Low | Stub. |
| `src/commands/mod.rs` | 71 | Low | Dispatch table. |
| `src/commands/publish.rs` | 15 | **High** | Remote publish path. No command-level tests. See recommendation #6. |
| `src/commands/resolve.rs` | 7 | Low | Stub. |
| `src/commands/runtime.rs` | 7 | Low | Stub. |
| `src/commands/rust_target.rs` | 171 | **Critical** | Cargo/rustc build logic. No tests. See recommendation #1. |
| `src/commands/staged.rs` | 11 | Low | Trivial stub. |
| `src/commands/target.rs` | 7 | Low | Stub. |
| `src/commands/update.rs` | 7 | Low | Stub. |
| `src/commands/yank.rs` | 7 | Low | Stub. |
| `src/commands/fetch.rs` | 17 | **Critical** | Remote fetch path. No command-level tests. See recommendation #4. |

### Integration tests

#### `tests/hygiene.rs`
**Line count**: 31. Tests production-code hygiene budgets (no unwrap, expect, panic, etc. in production code). Well-structured. `Low` risk.

#### `crates/hygiene-ratchet/src/lib.rs`
**Line count**: 334. Has no inline tests of its own (but it's the hygiene scanner itself, tested by `tests/hygiene.rs`). The `scrub_rust_source` function (220 lines) is a mini parser/lexer with no unit tests — a bug in comment/string/lifetime scrubbing could cause false positives or false negatives in hygiene budgets. `Medium`.

---

## Cross-cutting observations

### Positive patterns
- **Fault injection infrastructure**: The `#[cfg(test)]` fault injection in `faber_lock.rs` and `fs_util.rs` is well-designed. Thread-local state with `RefCell` and auto-reset makes tests deterministic and isolated.
- **Thorough transaction safety testing**: Install, faber_lock, and fs_util tests systematically exercise failure at every stage of atomic write-and-replace operations (before create, write, rename, cleanup, sync). This is strong.
- **Identity validation is well-covered**: The store module and shared validation have comprehensive tests for identity mismatch, missing identity, and @-sign injection.
- **Symlink and special-entry rejection**: Store, fs_util, and registry modules all test that symlinks and non-regular entries are rejected. Good security posture.

### Negative patterns
- **Untested external process boundaries**: `rust_target.rs` shells out to `cargo` and `rustc` with no test isolation. A missing toolchain, version mismatch, or unexpected output format causes untested error paths.
- **Remote registry entirely untested**: `publish_remote`, `fetch_remote_to_cache`, and `UreqTransport` have no test coverage. The mock `HermeticRegistry` replaces the transport layer but never exercises the real HTTP path. This is the highest-risk gap for a package manager.
- **CLI argument parsing is untested for 18/19 commands**: Only `install` has CLI parsing tests. A typo in a `#[arg]` attribute or a missing `conflicts_with` constraint would go undetected.
- **No test for `verify_target_build=true`**: The build verification path in `shared::validate_package` is completely uncovered.
- **Test isolation**: `run_test.rs::run_installed_binary_with_passthrough_argument` calls `super::super::install::run()` to set up its fixture. If install breaks, this test breaks too, even if run is correct.

### Parallel test improvement opportunities
The following areas are non-overlapping and could be improved in parallel:
1. `manifest.rs` unit tests (new test file)
2. `rust_target.rs` unit tests (new test file, isolated from real cargo/rustc)
3. `cli_test.rs` expansion (add tests for remaining commands)
4. Remote HTTP transport tests (expand `registry_http_test.rs`)
5. Shared test helpers extraction (create `src/test_support.rs`)
6. `inspect.rs` / `package.rs` / `login.rs` / `logout.rs` command tests
