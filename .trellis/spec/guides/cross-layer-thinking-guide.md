# Cross-Layer Thinking Guide

> **Purpose**: Think through data flow across layers before implementing.

---

## The Problem

**Most bugs happen at layer boundaries**, not within layers.

Common cross-layer bugs:

- API returns format A, frontend expects format B
- Database stores X, service transforms to Y, but loses data
- Multiple layers implement the same logic differently

---

## Before Implementing Cross-Layer Features

### Step 1: Map the Data Flow

Draw out how data moves:

```
Source → Transform → Store → Retrieve → Transform → Display
```

For each arrow, ask:

- What format is the data in?
- What could go wrong?
- Who is responsible for validation?

### Step 2: Identify Boundaries

| Boundary               | Common Issues                     |
| ---------------------- | --------------------------------- |
| API ↔ Service         | Type mismatches, missing fields   |
| Service ↔ Database    | Format conversions, null handling |
| Backend ↔ Frontend    | Serialization, date formats       |
| Component ↔ Component | Props shape changes               |

### Step 3: Define Contracts

For each boundary:

- What is the exact input format?
- What is the exact output format?
- What errors can occur?

---

## Common Cross-Layer Mistakes

### Mistake 1: Implicit Format Assumptions

**Bad**: Assuming date format without checking

**Good**: Explicit format conversion at boundaries

### Mistake 2: Scattered Validation

**Bad**: Validating the same thing in multiple layers

**Good**: Validate once at the entry point

### Mistake 3: Leaky Abstractions

**Bad**: Component knows about database schema

**Good**: Each layer only knows its neighbors

### Mistake 4: Every Consumer Parses The Same Payload

**Bad**: Each renderer consumer locally casts the same raw Tauri, event, or
configuration payload field.

This looks local, but it means every consumer owns a private version of the
payload contract. The next field change will update one consumer and miss
another.

**Good**: Decode/normalize once at the owner boundary, then export typed
projections to every consumer.

**Rule**: For append-only logs, JSON streams, RPC payloads, or config files,
create one owner for:

- event / payload type definitions
- type guards and normalization from `unknown`
- metadata projections used by UI commands
- reducers that replay state from the source of truth

Rendering code may format fields, but it must not redefine the payload contract.

---

## Checklist for Cross-Layer Features

Before implementation:

- [ ] Mapped the complete data flow
- [ ] Identified all layer boundaries
- [ ] Defined format at each boundary
- [ ] Decided where validation happens

After implementation:

- [ ] Tested with edge cases (null, empty, invalid)
- [ ] Verified error handling at each boundary
- [ ] Checked data survives round-trip
- [ ] Checked that consumers import shared decoders / projections instead of
      casting payload fields locally
- [ ] Checked that derived state uses the existing source version/cursor rather
      than inventing a second one
- [ ] Put concrete signatures, DTO fields, validation matrices, and test
      requirements in the owning backend/frontend code-spec, not this guide

For a Tauri command, event, or serialized payload, read
[Frontend Type Safety](../frontend/type-safety.md) and the owning backend
contract before changing either side.

---

## Versioned FyAgent Documentation Boundary

For versioned FyAgent documentation, locate the owning backend code-spec and
its `Tests Required` section first. Keep version-specific paths, compatibility
boundaries, and validation requirements there; this guide is only a routing
prompt, not a parallel product document.

## Remote-Probe Boundary

When a provider, installer, or configuration flow changes behavior after a
remote probe:

- [ ] Make every call path distinguish a definitive absence from a transient or
      malformed response.
- [ ] Ensure a retry/shortcut path retains the same validation and credential
      boundary as the interactive path.
- [ ] Reset stale cached/prefetched state when the selected source changes.
- [ ] Consume complete bounded input before parsing metadata; do not parse an
      arbitrary prefix as a complete response.
- [ ] Put exact URLs, request/response fields, error codes, and test cases in
      the owning code-spec.

## When to Create Flow Documentation

Create detailed flow docs when:

- Feature spans 3+ layers
- Multiple teams are involved
- Data format is complex
- Feature has caused bugs before

---

## Tauri IPC and Event Boundary

For a new or changed host-to-renderer payload:

- [ ] Map the Rust service/type, command registration, TypeScript facade, hook
      or state owner, and rendering consumer before editing.
- [ ] Keep serialization/normalization at the owner boundary; consumers render
      typed projections instead of locally re-parsing raw payload fields.
- [ ] Decide which layer owns validation and structured errors, then test a
      success case, a rejected input, and an invalid/stale payload case.
- [ ] Keep event listeners bounded to the component/hook lifecycle and validate
      externally supplied event payloads before updating UI state.

The exact signature, DTO fields, error matrix, and test assertions belong in the
owning backend/frontend code-spec, not this thinking guide.
