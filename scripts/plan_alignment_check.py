#!/usr/bin/env python3
"""Deterministic project-plan alignment check; makes no LLM calls."""
from __future__ import annotations

import json
import sys
from datetime import datetime, timezone
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
PLAN = ROOT / ".hermes" / "plans" / "2026-07-14_181851-onderzoek-en-aanpak-voxel-engine.md"
REQUIRED_FILES = [
    ROOT / "AGENTS.md",
    ROOT / ".hermes" / "PROJECT_STATE.md",
    ROOT / "docs" / "governance" / "research-protocol.md",
    ROOT / "docs" / "governance" / "budget-policy.md",
    ROOT / "docs" / "governance" / "alignment-log.md",
    ROOT / "docs" / "benchmarks" / "contract.md",
]
REQUIRED_PLAN_TERMS = [
    "North star",
    "Levend-plancontract",
    "Drie-maandenroadmap",
    "Hermes/OpenRouter-workflow",
    "na iedere derde",
    "Authoritative multiplayer",
]


def main() -> int:
    missing = [str(path.relative_to(ROOT)) for path in REQUIRED_FILES + [PLAN] if not path.exists()]
    terms_missing = []
    if PLAN.exists():
        text = PLAN.read_text(encoding="utf-8")
        terms_missing = [term for term in REQUIRED_PLAN_TERMS if term.lower() not in text.lower()]
    report = {
        "checked_at_utc": datetime.now(timezone.utc).isoformat(),
        "missing_files": missing,
        "missing_plan_terms": terms_missing,
        "aligned": not missing and not terms_missing,
    }
    out = ROOT / ".hermes" / "reports"
    out.mkdir(parents=True, exist_ok=True)
    path = out / "plan-alignment-latest.json"
    path.write_text(json.dumps(report, indent=2, ensure_ascii=False), encoding="utf-8")
    if report["aligned"]:
        print(f"OK plan-alignment: canoniek plan en governancebestanden aanwezig. Rapport: {path}")
        return 0
    print(f"ALERT plan-alignment: ontbrekende bestanden={missing}; ontbrekende termen={terms_missing}. Rapport: {path}")
    return 1


if __name__ == "__main__":
    sys.exit(main())
