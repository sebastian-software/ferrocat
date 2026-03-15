# ADR 0001: Rust-Native, Performance-First Core

- Status: Accepted
- Date: 2026-03-15

## Context

The project started from the goal of porting feature coverage from `pofile-ts`, but not as a literal translation.

A direct port would preserve many source-language tradeoffs:

- string-heavy control flow
- allocation patterns optimized for a different runtime
- APIs shaped more by JavaScript ergonomics than by Rust ownership and borrowing

The project target is a Rust library that can later support Node/N-API and other integrations.

## Decision

Build `ferrox` as a Rust-native implementation with performance as a first-class constraint.

This means:

- optimize around bytes and slices in hot paths
- isolate structural scanning from semantic parsing
- prefer data models that reflect real PO semantics instead of JavaScript legacy shapes
- treat profiling results as the source of truth for optimization work

## Consequences

Positive:

- cleaner long-term architecture
- more headroom for low-level optimization such as SIMD/NEON
- APIs can expose both ergonomic and high-performance paths

Negative:

- implementation diverges from the source project more quickly
- some design decisions require more up-front architecture work
- benchmark and profiling infrastructure becomes part of the core development loop
