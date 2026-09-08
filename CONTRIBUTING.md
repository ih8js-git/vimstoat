# Contributing Guide

First off, thank you for considering contributing to vimstoat!

## Pull Request Process

**CRITICAL:** All Pull Requests MUST be made against the `staging` branch.

Do **NOT** open Pull Requests directly against the `main` branch. The `main` branch is reserved for production releases only.

### Steps to Contribute:
1. Fork the repository.
2. Create a new branch off `staging` for your feature or bugfix.
3. Make your changes and commit them using standard naming practices (e.g. `fix: the thing`).
4. Push your branch to your fork.
5. Open a Pull Request targeting the `staging` branch of the upstream repository using the same naming format for your PR title.

## Commit & PR Naming Conventions

Standard naming practices (Conventional Commits) are advised for both commit messages and PR titles:

Format: `<type>: <description>`

Common types:
- `fix: <description>` — Bug fixes (e.g. `fix: the thing`)
- `feat: <description>` — New features
- `docs: <description>` — Documentation changes
- `refactor: <description>` — Code refactoring without behavioral changes
- `test: <description>` — Adding or updating tests
- `chore: <description>` — Routine tasks, dependencies, or maintenance

**Example:**
* `fix: the thing`
* `feat: add support for custom keybindings`

Thanks for helping make this project better!
