#!/usr/bin/env python3
"""Validate CHANGELOG.md against the repository changelog conventions.

The project follows Keep a Changelog. This checker intentionally avoids third-
party dependencies so it can run quickly in CI and locally through `make`.
"""

from __future__ import annotations

from dataclasses import dataclass, field
from datetime import date
from pathlib import Path
import re
import sys

REPO_URL = "https://github.com/bitcoin-numeraire/swissknife"
CHANGELOG_PATH = Path(__file__).resolve().parents[1] / "CHANGELOG.md"

ALLOWED_CATEGORIES = (
    "Added",
    "Changed",
    "Deprecated",
    "Removed",
    "Fixed",
    "Security",
)

VERSION_RE = re.compile(
    r"^## \[(?P<version>Unreleased|(?:0|[1-9]\d*)\.(?:0|[1-9]\d*)\."
    r"(?:0|[1-9]\d*)(?:[-+][0-9A-Za-z.-]+)?)\]"
    r"(?: - (?P<date>\d{4}-\d{2}-\d{2}))?$"
)
CATEGORY_RE = re.compile(r"^### (?P<name>.+)$")
LINK_DEF_RE = re.compile(r"^\[([^\]]+)\]:\s+(\S+)\s*$")
REFERENCE_RE = re.compile(r"\[([^\]\n]+)\](?!\()")
SEMVER_RE = re.compile(
    r"^(?:0|[1-9]\d*)\.(?:0|[1-9]\d*)\.(?:0|[1-9]\d*)"
    r"(?:[-+][0-9A-Za-z.-]+)?$"
)
COMMIT_REF_RE = re.compile(r"^[0-9a-f]{7,40}$")


@dataclass
class Release:
    version: str
    line: int
    date_text: str | None
    categories: dict[str, int] = field(default_factory=dict)


def main() -> int:
    lines = CHANGELOG_PATH.read_text(encoding="utf-8").splitlines()
    errors: list[str] = []

    if not lines:
        errors.append("CHANGELOG.md is empty")
    elif lines[0] != "# Changelog":
        errors.append("line 1: expected '# Changelog'")

    releases = _validate_release_structure(lines, errors)
    _validate_links(lines, releases, errors)

    if errors:
        print("CHANGELOG.md validation failed:", file=sys.stderr)
        for error in errors:
            print(f"- {error}", file=sys.stderr)
        return 1

    print(f"CHANGELOG.md OK ({len(releases)} release section(s) checked)")
    return 0


def _validate_release_structure(lines: list[str], errors: list[str]) -> list[Release]:
    releases: list[Release] = []
    current_release: Release | None = None
    current_category: str | None = None
    last_category_index = -1
    seen_versions: set[str] = set()

    def finish_release() -> None:
        if current_release is None:
            return
        for category, count in current_release.categories.items():
            if count == 0:
                errors.append(
                    f"line {current_release.line}: category '{category}' has no entries"
                )
        if current_release.version != "Unreleased" and not current_release.categories:
            errors.append(
                f"line {current_release.line}: release '{current_release.version}' has no change categories"
            )

    for line_number, line in enumerate(lines, start=1):
        if line.startswith("## "):
            finish_release()
            match = VERSION_RE.match(line)
            if not match:
                errors.append(
                    f"line {line_number}: release headings must look like "
                    "'## [Unreleased]' or '## [X.Y.Z] - YYYY-MM-DD'"
                )
                current_release = None
                current_category = None
                last_category_index = -1
                continue

            version = match.group("version")
            date_text = match.group("date")
            if version in seen_versions:
                errors.append(f"line {line_number}: duplicate release heading '{version}'")
            seen_versions.add(version)

            if version == "Unreleased":
                if releases:
                    errors.append(f"line {line_number}: [Unreleased] must be the first release")
                if date_text is not None:
                    errors.append(f"line {line_number}: [Unreleased] must not have a date")
            else:
                if date_text is None:
                    errors.append(
                        f"line {line_number}: released version '{version}' must include a date"
                    )
                else:
                    try:
                        date.fromisoformat(date_text)
                    except ValueError:
                        errors.append(
                            f"line {line_number}: '{date_text}' is not a valid ISO date"
                        )

            current_release = Release(version, line_number, date_text)
            releases.append(current_release)
            current_category = None
            last_category_index = -1
            continue

        if line.startswith("### "):
            if current_release is None:
                errors.append(f"line {line_number}: category heading appears before a release")
                continue

            match = CATEGORY_RE.match(line)
            category = match.group("name") if match else ""
            if category not in ALLOWED_CATEGORIES:
                allowed = ", ".join(ALLOWED_CATEGORIES)
                errors.append(
                    f"line {line_number}: unsupported category '{category}' "
                    f"(allowed: {allowed})"
                )
                current_category = None
                continue

            category_index = ALLOWED_CATEGORIES.index(category)
            if category_index < last_category_index:
                errors.append(
                    f"line {line_number}: category '{category}' is out of order; "
                    f"use {', '.join(ALLOWED_CATEGORIES)}"
                )
            if category in current_release.categories:
                errors.append(
                    f"line {line_number}: duplicate category '{category}' in "
                    f"release '{current_release.version}'"
                )

            current_release.categories[category] = 0
            current_category = category
            last_category_index = category_index
            continue

        if line.startswith("#") and line != "# Changelog":
            errors.append(f"line {line_number}: unsupported markdown heading '{line}'")
            continue

        if line.lstrip().startswith("- "):
            if current_release is None:
                errors.append(f"line {line_number}: change entry appears before a release")
            elif current_category is None:
                errors.append(f"line {line_number}: change entry appears before a category")
            else:
                current_release.categories[current_category] += 1

    finish_release()

    if not releases:
        errors.append("missing '## [Unreleased]' section")
    elif releases[0].version != "Unreleased":
        errors.append("first release section must be '## [Unreleased]'")

    _validate_release_order(releases, errors)
    return releases


def _validate_release_order(releases: list[Release], errors: list[str]) -> None:
    released_versions = [release for release in releases if release.version != "Unreleased"]
    previous: tuple[int, int, int] | None = None
    previous_line = 0
    for release in released_versions:
        version = _parse_core_version(release.version)
        if version is None:
            continue
        if previous is not None and version >= previous:
            errors.append(
                f"line {release.line}: release '{release.version}' should appear before "
                f"older versions; previous release heading is on line {previous_line}"
            )
        previous = version
        previous_line = release.line


def _parse_core_version(version: str) -> tuple[int, int, int] | None:
    core = re.split(r"[-+]", version, maxsplit=1)[0]
    parts = core.split(".")
    if len(parts) != 3:
        return None
    return tuple(int(part) for part in parts)  # type: ignore[return-value]


def _validate_links(lines: list[str], releases: list[Release], errors: list[str]) -> None:
    definitions: dict[str, tuple[str, int]] = {}

    for line_number, line in enumerate(lines, start=1):
        match = LINK_DEF_RE.match(line)
        if not match:
            continue
        label, url = match.groups()
        if label in definitions:
            errors.append(f"line {line_number}: duplicate link definition '[{label}]'")
        definitions[label] = (url, line_number)
        _validate_link_definition(label, url, line_number, errors)

    for release in releases:
        if release.version not in definitions:
            errors.append(
                f"line {release.line}: missing link definition for '[{release.version}]'"
            )

    for line_number, line in enumerate(lines, start=1):
        if LINK_DEF_RE.match(line):
            continue
        for label in REFERENCE_RE.findall(line):
            if label not in definitions:
                errors.append(f"line {line_number}: missing link definition for '[{label}]'")


def _validate_link_definition(
    label: str, url: str, line_number: int, errors: list[str]
) -> None:
    if label == "Unreleased":
        if not url.startswith(f"{REPO_URL}/compare/") or not url.endswith("...HEAD"):
            errors.append(
                f"line {line_number}: [Unreleased] must compare a release tag to HEAD"
            )
        return

    if SEMVER_RE.match(label):
        expected = f"{REPO_URL}/releases/tag/v{label}"
        if url != expected:
            errors.append(f"line {line_number}: [{label}] should link to {expected}")
        return

    if label.startswith("#") and label[1:].isdigit():
        pull_url = f"{REPO_URL}/pull/{label[1:]}"
        issue_url = f"{REPO_URL}/issues/{label[1:]}"
        if url not in {pull_url, issue_url}:
            errors.append(
                f"line {line_number}: [{label}] should link to {pull_url} or {issue_url}"
            )
        return

    if COMMIT_REF_RE.match(label):
        expected = f"{REPO_URL}/commit/{label}"
        if url != expected:
            errors.append(f"line {line_number}: [{label}] should link to {expected}")
        return

    errors.append(f"line {line_number}: unsupported link definition label '[{label}]'")


if __name__ == "__main__":
    raise SystemExit(main())
