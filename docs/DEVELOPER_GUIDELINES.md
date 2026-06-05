# Developer Guidelines

This document defines backend development conventions for SwissKnife. It should be used together with `AGENTS.md`, `README.md`, and the Makefile.

## Architecture

SwissKnife follows a clean architecture direction:

- handlers deal with API concerns and delegate business work
- services and use cases own orchestration and business rules
- repositories expose data access contracts
- infrastructure modules implement repository, database, Lightning, wallet, authentication, server, and external adapter details
- DTOs belong at API boundaries, while domain entities belong in business logic

Keep dependencies pointing inward. Domain code should depend on traits and application errors, not on concrete infrastructure implementations.

Do not add infrastructure shortcuts or test-only production seams to make unit tests easier. If a component is hard to test, improve the production design instead.

## Unit Tests

Unit tests check individual business units in isolation. They should cover services, handlers, use cases, and pure domain logic. Infrastructure implementations and database adapters should usually be covered by integration tests instead.

Unit tests must not start real databases, Lightning nodes, HTTP servers, or other external dependencies. Injected dependencies should be mocked with generated `mockall` mocks.

### Structure

Write Rust unit tests next to the code they test, under `#[cfg(test)] mod tests`.

Use nested modules to keep the same hierarchy that `describe`/`it` provides in TypeScript:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    mod method_or_function_name {
        use super::*;

        mod when_context_is_true {
            use super::*;

            #[test]
            fn returns_expected_result() {
                // arrange

                // act

                // assert
            }
        }

        mod when_dependency_fails {
            use super::*;

            #[tokio::test]
            async fn propagates_error() {
                // arrange

                // act

                // assert
            }
        }
    }
}
```

Use this naming convention:

- method module: the method or function under test, for example `validate_amount`
- context module: `when_*`, `with_*`, or `without_*`
- test function: expected behavior, for example `returns_amount`, `rejects_zero`, or `propagates_error`

Use this full hierarchy for components that orchestrate logic, branch on meaningful state, enforce permissions, or coordinate dependencies. For very small pure helpers, keep tests compact while preserving clear method and behavior names.

### Behavior Expectations

For each service method or business function, cover:

- happy path
- validation branches
- meaningful conditional branches
- permission or ownership checks, when applicable
- not-found and conflict cases, when applicable
- dependency error propagation, when dependencies are mocked

Keep each test focused on one behavior. Do not combine several flows in one unit test.

### Mocks

Use generated `mockall` mocks for injected dependencies:

```rust
let mut dependency = MockDependency::new();
dependency
    .expect_call_name()
    .with(/* predicates */)
    .times(1)
    .returning(/* result */);
```

Prefer `.with(...)` or `.withf(...)` plus `.times(...)` for interactions that are part of the behavior being tested. Avoid hand-written mocks unless `mockall` cannot express the dependency.

## Integration Tests

Integration tests treat SwissKnife as a black box and exercise public APIs with reproducible dependencies. These tests may use real Postgres, SQLite, Lightning nodes in regtest, or mocked external servers, depending on the capability being tested.

Integration tests should be concise and capability-focused. They should not combine multiple user stories into one scenario.

## End-to-End Tests

End-to-end tests cover complete user stories across the deployed system and dashboard.
