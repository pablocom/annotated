# Testing

Tests are living documentation. Reading the suite tells you what the system does and the rules it follows.

## Tests as documentation

- Name each test after the behavior it shows, as a sentence in domain language: `settling_a_debt_marks_it_as_paid`, not `test_settle_1`. Follow the project's naming convention when it has one.
- One behavior per test, laid out as arrange, act, assert, separated by blank lines rather than labeling comments.
- Show only what matters to the behavior. Hide incidental setup behind builders or fixtures with intention-revealing names, so every value visible in the test is one that matters.
- No logic in tests: no loops, no conditionals, no expected values computed the way production computes them. Expected values are literals.
- Prefer assertions whose failure message explains what went wrong: compare whole values (`assert_eq`) over `assert!(condition)`. When asserting an error, assert the specific kind or message, not just that something failed.
- Test through public behavior, not internals, so tests describe what callers rely on and survive refactoring.

## Test data builders

- Build test data with several parts through a test data builder: `an_account().with_email("lucia@example.com").build()`. The builder starts from valid defaults, and a test sets only the parts its behavior depends on.
- Every value a test asserts on is set in that test, never left to a default.
- Defaults are fixed literals, with no randomness and no counters, so a failure reproduces.
- A builder reaches a state through the domain's public behavior, the way production does. It never builds an aggregate the domain couldn't produce.
- Name the entry function after what it builds, so the setup reads as a sentence: `an_account()`, `sign_up_details()`. Each `with_` method changes one part, and `build()` returns the value.
- A builder that one test file needs lives in that file until a second one needs it; then it moves to the crate's `tests/support/` module.
- A value with a single part, like an email address or a password, keeps a plain helper function: `email("lucia@example.com")`.

## Compile-time behavior

- Every compile error the derive emits has a compile-fail test under `tests/ui/`, run with `trybuild`, whose expected output pins the message and the span it points at.
- What the derive accepts is shown by tests that compile and run; what it refuses is shown by compile-fail tests. Never leave a refusal untested.

## Every test fails for the right reason

A test justifies the production code it covers. If that code were missing or wrong, the test must fail, and the failure must point at that code, not at setup.

- **New behavior: test first.** Run the test before the implementation exists and watch it fail. Check that the failure is the assertion about the missing behavior, not a compile error, a setup panic, a wrong URL or a missing fixture. A compile error doesn't count as red: add a stub that compiles and returns the wrong thing, then see the assertion fail. Then write the minimum code to pass, and refactor with the tests green.
- **Existing code, or tests written after the implementation:** break the production code on purpose (remove the line, invert the condition, return a default). Run the test, confirm it fails with a message that names the broken behavior, then restore the code and confirm green. If it still passes, the test doesn't cover what it claims; fix the test.
- If production code can be deleted without any test failing, either the code is unjustified or a test is missing. Say which.
- Never weaken an assertion just to make a test pass.
- When reporting work, list the tests you saw fail and the failure reason for each, in one line apiece.
