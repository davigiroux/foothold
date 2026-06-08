---
name: rust-learnings
description: Use when answering any Rust question in this project — scan patterns before responding, surface the relevant section if one matches so the user can connect it to something they already learned.
---

# Rust learnings

## How to use

Before answering a Rust question:
1. Scan the quick-reference table for a matching trigger
2. If one matches, quote the full pattern section back — the user may have seen it before and a reminder beats a re-explanation
3. If the question reveals a new pattern worth remembering, add it here

---

## Quick reference

| Trigger in their code | Pattern to suggest |
|---|---|
| `if let Some(x) = ... { Some(f(x)) } else { None }` | `.map()` |
| Needs both of two Options to be Some | `.zip().map()` |
| `let mut flag = false` updated in multiple places | Extract method, inline in condition |
| `if x == Variant::A { } else if x == Variant::B { }` on an enum | `match` |
| `if let Some(x) = ... { use(x) } else { return None }` | `?` operator |
| `return value` as the last line of a function | Drop `return`, no semicolon |
| `if cond { Some(x) } else { None }` | `condition.then_some(x)` |
| `{ field: field, other: other }` in a struct literal | `{ field, other }` shorthand |
| `&mut x` in a function signature | Swap to `x: &mut T` |
| `let Reverse(price) = *key` inside `.map()` | Destructure directly in closure param: `|(Reverse(price), _)|` |

---

## Patterns

### `.map()` — transform an Option's inner value

**Trigger:** `if let Some(x) = ... { Some(f(x)) } else { None }`

TS analogy: `arr.map(fn)[0]` vs manually checking `arr.length > 0`

```rust
// before
if let Some((price, _)) = self.asks.first_key_value() {
    Some(*price)
} else {
    None
}

// after
self.asks.first_key_value().map(|(price, _)| *price)
```

Rule: if the whole body is `if Some → Some(transform) else → None`, it's always `.map()`.

---

### `.zip().map()` — combine two Options

**Trigger:** needs both Options to be Some to produce a result

TS analogy: `a !== undefined && b !== undefined ? f(a, b) : undefined`

```rust
// before
if let (Some(ask), Some(bid)) = (ask_price, bid_price) {
    Some(ask - bid)
} else {
    None
}

// after
self.best_ask().zip(self.best_bid()).map(|(ask, bid)| ask - bid)
```

Rule: two Options needed together → `.zip().map()`.

---

### Extract method for state-derived booleans

**Trigger:** `let mut is_crossable = false` updated before and inside a loop

```rust
// before — flag computed once, recomputed manually at loop bottom
let mut is_crossable = false;
if order.side == Side::Bid { is_crossable = ...; }
while ... && is_crossable {
    // ...
    is_crossable = ...; // easy to forget this
}

// after — live query, never stale
fn is_crossable(&self, side: Side, price: Price) -> bool { ... }

while remaining > Decimal::ZERO && self.is_crossable(order.side, order_price) {
```

Rule: if a boolean is always derived from current state, don't store it — query it.

---

### `match` over enum instead of `if/else if`

**Trigger:** `if x == Variant::A { } else if x == Variant::B { }`

TS analogy: `switch` — but Rust's `match` is exhaustive (compiler errors if you miss a variant)

```rust
// before
if order.side == Side::Bid { ... } else if order.side == Side::Ask { ... }

// after
match order.side {
    Side::Bid => { ... }
    Side::Ask => { ... }
}
```

Rule: comparing an enum? Always `match`. Compiler will catch missing variants.

---

### `?` operator — early return on None/Err

**Trigger:** `if let Some(x) = maybe { use(x) } else { return None }`

TS analogy: `const x = maybe ?? return null` (not real TS, but the concept)

```rust
// before
if let Some((side, price)) = self.index.get(&id).copied() {
    // ... long block ...
} else {
    None
}

// after
let (side, price) = self.index.get(&id).copied()?;
// ... continues naturally, returns None early if missing
```

Rule: any time you unwrap and return early on None, use `?`.

---

### Drop `return` on final expression

**Trigger:** `return value;` as the last statement in a function

```rust
// before
return MatchResult { trades, remaining: ... };

// after
MatchResult { trades, remaining: ... }
```

Rule: `return` is only for early exits inside a function. Last expression = no `return`, no `;`.

---

### `.then_some()` — conditional Option

**Trigger:** `if condition { Some(x) } else { None }`

TS analogy: `condition ? x : undefined`

```rust
// before
remaining: if incoming_remaining > Decimal::ZERO { Some(order) } else { None }

// after
remaining: (incoming_remaining > Decimal::ZERO).then_some(order)
```

---

### Struct field shorthand

**Trigger:** `{ field: field }` when variable name matches field name

TS analogy: same — `{ bids: bids }` → `{ bids }`

```rust
Depth { bids: bids, asks: asks }  // before
Depth { bids, asks }               // after
```

---

### `&mut` placement in signatures

**Trigger:** `&mut x: SomeType` — `&mut` before the name

```rust
fn fill(&mut queue: VecDeque<Order>)  // wrong — &mut goes on the type
fn fill(queue: &mut VecDeque<Order>)  // correct
```

---

## Adding new patterns

When a new Rust pattern comes up in the conversation that's worth remembering, add it here:
1. Row in the quick-reference table (trigger → pattern name)
2. Full section with trigger, TS analogy, before/after, rule
