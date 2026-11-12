# ADR-0004: Prefix notation with fixed arity, no parentheses (default)

Status: Accepted (provisional, subject to A/B in milestone M5)
Date: 2026-11-12

## Context
Two candidate surface syntaxes: (A) prefix with fixed arity, no parentheses — fewest tokens, one arity mistake shifts the parse; (B) delimited S-expressions — more tokens, more robust. The choice must be made with benchmark data, but implementation needs a default now.

## Decision
Implement (A). Parsing is arity-directed: pass 1 collects signatures and record types, pass 2 parses bodies. Arity errors produce P002 with the expected arity.

## Consequences
- The printer, not the parser, owns layout; the AST is independent of the surface syntax, so (B) can be added later as an alternative printer/parser for the A/B test.
- Diagnostics for arity errors must be excellent, since they are the main failure mode of (A).
