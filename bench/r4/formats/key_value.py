"""
Plain key: value encoder and decoder (Standard Library only).
Per R4 maintainer decision:
Runs only on flat datasets (flat_tabular, flat_100).
Raises ValueError for nested shapes (nested_fixed, nested_variable).
"""

import re

def encode(data, shape_name=None):
    """Encodes flat tabular records to plain key: value lines."""
    if not isinstance(data, list):
        raise ValueError("Data must be a list of records")
    if len(data) == 0:
        return ""

    first = data[0]
    if "notes" in first or ("source" in first and isinstance(first["source"], dict)):
        raise ValueError("key: value format does not support nested structures without loss (skipped per R4 Decision 1)")

    blocks = []
    cols = list(first.keys())

    for row in data:
        lines = []
        for col in cols:
            val = row.get(col)
            if val is None:
                val_str = "null"
            elif isinstance(val, bool):
                val_str = "true" if val else "false"
            else:
                val_str = str(val)
            lines.append(f"{col}: {val_str}")
        blocks.append("\n".join(lines))

    return "\n---\n".join(blocks) + "\n"

def decode(text, shape_name=None):
    """Decodes plain key: value lines back to flat tabular records."""
    blocks = [b.strip() for b in re.split(r'(?m)^---\s*$', text) if b.strip()]
    records = []

    for block in blocks:
        rec = {}
        for line in block.splitlines():
            line = line.strip()
            if not line or ":" not in line:
                continue
            k, v = line.split(":", 1)
            k = k.strip()
            v = v.strip()
            if v == "null" or v == "~" or v == "":
                rec[k] = None
            elif v.lower() == "true":
                rec[k] = True
            elif v.lower() == "false":
                rec[k] = False
            elif re.match(r"^-?\d+$", v):
                rec[k] = int(v)
            else:
                rec[k] = v
        if rec:
            records.append(rec)

    return records
