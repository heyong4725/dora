#!/usr/bin/env python3
"""Validate the canonical user-story feature tracker CSV."""

from __future__ import annotations

import csv
import re
import sys
from pathlib import Path


TRACKER = Path("docs/user-story-feature-status.csv")

EXPECTED_HEADER = [
    "story_id",
    "feature_area",
    "feature",
    "source_refs",
    "user_story",
    "expected_behavior",
    "feature_status",
    "test_status",
    "test_evidence",
    "last_verified_utc",
    "observed_errors",
    "fix_status",
    "retest_status",
    "notes",
]

ALLOWED_FEATURE_STATUS = {
    "active",
    "bypassed",
    "story-drafted",
    "tested-pass",
}

ALLOWED_TEST_STATUS = {
    "bypassed-external",
    "external-dependency",
    "mixed",
    "passed",
    "platform-dependent",
    "source-audited",
}

ALLOWED_FIX_STATUS = {
    "fixed",
    "none",
    "not-applicable",
    "test-coverage-added",
    "tracker-corrected; test-coverage-added",
}

ALLOWED_RETEST_STATUS = {
    "bypassed-external",
    "external-dependency",
    "not-applicable",
    "passed",
    "platform-dependent",
}

ID_RE = re.compile(r"^[A-Z][A-Z0-9]+-\d{3}$")
DATE_RE = re.compile(r"^\d{4}-\d{2}-\d{2}$")


def fail(errors: list[str], message: str) -> None:
    errors.append(message)


def require_value(errors: list[str], row: dict[str, str], column: str) -> None:
    if not row[column].strip():
        fail(errors, f"{row.get('story_id', '<missing-id>')}: `{column}` must not be empty")


def validate() -> list[str]:
    errors: list[str] = []
    if not TRACKER.exists():
        return [f"{TRACKER} does not exist"]

    with TRACKER.open(newline="") as f:
        reader = csv.DictReader(f)
        if reader.fieldnames != EXPECTED_HEADER:
            fail(
                errors,
                f"unexpected header: {reader.fieldnames!r}; expected {EXPECTED_HEADER!r}",
            )
            return errors
        rows = list(reader)

    if not rows:
        fail(errors, f"{TRACKER} must contain at least one story row")

    seen: set[str] = set()
    for index, row in enumerate(rows, start=2):
        story_id = row["story_id"].strip()
        if not story_id:
            fail(errors, f"line {index}: `story_id` must not be empty")
            continue
        if not ID_RE.fullmatch(story_id):
            fail(errors, f"{story_id}: story_id must match {ID_RE.pattern}")
        if story_id in seen:
            fail(errors, f"{story_id}: duplicate story_id")
        seen.add(story_id)

        for column in [
            "feature_area",
            "feature",
            "source_refs",
            "user_story",
            "expected_behavior",
            "test_evidence",
        ]:
            require_value(errors, row, column)

        if row["feature_status"] not in ALLOWED_FEATURE_STATUS:
            fail(errors, f"{story_id}: unknown feature_status `{row['feature_status']}`")
        if row["test_status"] not in ALLOWED_TEST_STATUS:
            fail(errors, f"{story_id}: unknown test_status `{row['test_status']}`")
        if row["fix_status"] not in ALLOWED_FIX_STATUS:
            fail(errors, f"{story_id}: unknown fix_status `{row['fix_status']}`")
        if row["retest_status"] not in ALLOWED_RETEST_STATUS:
            fail(errors, f"{story_id}: unknown retest_status `{row['retest_status']}`")

        if not DATE_RE.fullmatch(row["last_verified_utc"]):
            fail(
                errors,
                f"{story_id}: last_verified_utc must use YYYY-MM-DD, got `{row['last_verified_utc']}`",
            )

        if row["test_status"] == "passed" and row["retest_status"] != "passed":
            fail(errors, f"{story_id}: passed test_status must have passed retest_status")

        if row["test_status"] == "bypassed-external":
            if row["feature_status"] != "bypassed":
                fail(
                    errors,
                    f"{story_id}: bypassed-external rows must have feature_status `bypassed`",
                )
            if row["retest_status"] != "bypassed-external":
                fail(
                    errors,
                    f"{story_id}: bypassed-external rows must have retest_status `bypassed-external`",
                )
            if "bypass" not in f"{row['observed_errors']} {row['notes']}".lower():
                fail(
                    errors,
                    f"{story_id}: bypassed-external rows must document the bypass",
                )

        if row["test_status"] != "passed" and not (
            row["observed_errors"].strip() or row["notes"].strip()
        ):
            fail(
                errors,
                f"{story_id}: non-passed rows must document observed_errors or notes",
            )

        if row["fix_status"] == "fixed" and row["retest_status"] not in {
            "bypassed-external",
            "passed",
            "platform-dependent",
            "external-dependency",
        }:
            fail(errors, f"{story_id}: fixed rows must have a concrete retest status")

    return errors


def main() -> int:
    errors = validate()
    if errors:
        print(f"{TRACKER}: validation failed", file=sys.stderr)
        for error in errors:
            print(f"  - {error}", file=sys.stderr)
        return 1

    print(f"{TRACKER}: OK")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
