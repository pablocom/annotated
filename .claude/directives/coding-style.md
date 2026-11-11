# Coding style

Code explains itself. A reader understands what it does, and why it is shaped that way, from its structure and names alone.

## Naming

- Name things after what they mean in the domain, not how they are built: `overdue_invoices`, not `filtered_list`. Use the project's ubiquitous language when it has one.
- Booleans read as questions: `is_settled`, `has_members`.
- No abbreviations beyond universal ones (`id`, `url`, `http`). The wider a name's scope, the more descriptive it is.
- When an expression needs explaining, extract it into a well-named variable or function.

## Structure

- Small functions, each at one level of abstraction. The top-level function reads like a summary of the steps.
- Guard clauses and early returns over nesting.
- Use types to make illegal states unrepresentable (enums, newtypes, validating constructors) instead of scattering runtime checks.

## Design patterns

- Patterns are tools to keep in mind, not a goal. Don't go looking for places to fit them. Plain code is the default.
- Reach for one when a problem in front of you is the kind it solves, and using it makes the behavior more obvious: a strategy for interchangeable policies, a builder for objects with many optional parts.
- Default to the clearest structure. Give up clarity for performance only on a measured hot path or a known hard constraint. Keep the faster code contained behind a clear name.

## Comments

- Almost never. Before writing one, try to make it unnecessary by renaming, extracting or restructuring.
- Write one only for context the code cannot carry: an external constraint, a workaround for a third-party bug (with a link), a deliberate performance trade-off (with the measurement), or a surprising invariant.
- Never restate the code, narrate changes ("added X", "fixed Y") or leave commented-out code. History belongs in commit messages and PR bodies.
- This is a published library, so every public item has a doc comment, enforced by the `missing_docs` lint. It says what the item means and promises, with an example when that's quicker to read than prose. Private items follow the rule above: almost never.
- A doc example is a test: it compiles and runs, and shows the shortest real use.
- Leave existing comments alone unless they are wrong or your change makes them obsolete.
