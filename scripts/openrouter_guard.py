#!/usr/bin/env python3
"""Zero-inference OpenRouter budget and free-model guard."""
from __future__ import annotations

import json
import os
import sys
import urllib.error
import urllib.request
from datetime import datetime, timezone
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / ".hermes" / "reports"
OUT.mkdir(parents=True, exist_ok=True)


def fetch_json(url: str, api_key: str | None = None) -> dict:
    headers = {"User-Agent": "land-of-the-voxel-engine-budget-guard/1.0"}
    if api_key:
        headers["Authorization"] = f"Bearer {api_key}"
    req = urllib.request.Request(url, headers=headers)
    with urllib.request.urlopen(req, timeout=30) as response:
        return json.load(response)


def as_float(value, default=0.0) -> float:
    try:
        return float(value)
    except (TypeError, ValueError):
        return default


def load_openrouter_key() -> str | None:
    key = os.getenv("OPENROUTER_API_KEY")
    if key:
        return key
    candidates = []
    if os.getenv("HERMES_HOME"):
        candidates.append(Path(os.environ["HERMES_HOME"]) / ".env")
    if os.getenv("LOCALAPPDATA"):
        candidates.append(Path(os.environ["LOCALAPPDATA"]) / "hermes" / ".env")
    candidates.append(Path.home() / ".hermes" / ".env")
    for env_path in candidates:
        if not env_path.is_file():
            continue
        for raw in env_path.read_text(encoding="utf-8-sig").splitlines():
            line = raw.strip()
            if not line or line.startswith("#") or "=" not in line:
                continue
            name, value = line.split("=", 1)
            if name.strip() == "OPENROUTER_API_KEY":
                return value.strip().strip('"').strip("'") or None
    return None


def main() -> int:
    now = datetime.now(timezone.utc).isoformat()
    key = load_openrouter_key()
    report: dict = {"checked_at_utc": now, "inference_calls": 0}

    try:
        models = fetch_json("https://openrouter.ai/api/v1/models").get("data", [])
    except Exception as exc:
        print(f"ALERT OpenRouter modelcatalogus onbereikbaar: {exc}")
        return 2

    free = []
    for model in models:
        pricing = model.get("pricing") or {}
        if as_float(pricing.get("prompt"), 1) == 0 and as_float(pricing.get("completion"), 1) == 0:
            params = set(model.get("supported_parameters") or [])
            free.append({
                "id": model.get("id"),
                "context_length": model.get("context_length"),
                "tools": "tools" in params,
                "structured_outputs": "structured_outputs" in params,
                "expiration_date": model.get("expiration_date"),
            })
    free.sort(key=lambda item: item["id"] or "")
    report["free_models"] = free
    report["free_tool_model_count"] = sum(1 for item in free if item["tools"])

    status = "OK"
    messages = [f"{len(free)} gratis modellen; {report['free_tool_model_count']} met tools"]
    if key:
        try:
            info = fetch_json("https://openrouter.ai/api/v1/key", key).get("data", {})
            safe = {
                "label": info.get("label"),
                "limit": info.get("limit"),
                "limit_remaining": info.get("limit_remaining"),
                "usage": info.get("usage"),
                "usage_daily": info.get("usage_daily"),
                "usage_weekly": info.get("usage_weekly"),
                "usage_monthly": info.get("usage_monthly"),
                "is_free_tier": info.get("is_free_tier"),
            }
            report["key"] = safe
            usage = as_float(safe.get("usage"))
            remaining = safe.get("limit_remaining")
            spent = as_float(safe.get("limit")) - as_float(remaining) if safe.get("limit") is not None and remaining is not None else None
            report["key_limit_spent"] = spent
            messages.append(f"all-time usage=${usage:.4f}")
            if remaining is not None:
                messages.append(f"key remaining=${as_float(remaining):.4f}")
            if spent is not None:
                messages.append(f"project-key spent=${spent:.4f}")
                if spent >= 36:
                    status = "STOP"
                elif spent >= 30:
                    status = "PAID-DICHT"
                elif spent >= 22:
                    status = "BLOCKERS-ONLY"
                elif spent >= 10:
                    status = "REVIEW"
            else:
                status = "MONITOR"
        except urllib.error.HTTPError as exc:
            status = "ALERT"
            messages.append(f"key endpoint HTTP {exc.code}")
        except Exception as exc:
            status = "ALERT"
            messages.append(f"keycontrole mislukt: {exc}")
    else:
        status = "ALERT"
        messages.append("OPENROUTER_API_KEY ontbreekt in cron-environment")

    report["status"] = status
    path = OUT / "openrouter-latest.json"
    path.write_text(json.dumps(report, indent=2, ensure_ascii=False), encoding="utf-8")
    print(f"{status} OpenRouter guard: " + "; ".join(messages) + f". Rapport: {path}")
    return 0 if status not in {"STOP", "ALERT"} else 1


if __name__ == "__main__":
    sys.exit(main())
