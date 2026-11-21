"""
TSV encoder and decoder (Standard Library only).
Per R4 maintainer decision:
Runs only on flat datasets (flat_tabular, flat_100).
Raises ValueError for nested shapes (nested_fixed, nested_variable).
"""

import re

def encode(data, shape_name=None):
    """Encodes flat tabular records to TSV."""
    if not isinstance(data, list):
        raise ValueError("Data must be a list of records")
    if len(data) == 0:
        return ""

    first = data[0]
    if "notes" in first or ("source" in first and isinstance(first["source"], dict)):
        raise ValueError("TSV format does not support nested structures without loss (skipped per R4 Decision 1)")

    cols = list(first.keys())
    lines = ["\t".join(cols) + "\n"]

    for row in data:
        row_vals = []
        for col in cols:
            val = row.get(col)
            if val is None:
                row_vals.append("")
            elif isinstance(val, bool):
                row_vals.append("true" if val else "false")
            else:
                row_vals.append(str(val))
        lines.append("\t".join(row_vals) + "\n")

    return "".join(lines)

def decode(text, shape_name=None):
    """Decodes TSV back to flat tabular records."""
    lines = [l for l in text.splitlines() if l.strip()]
    if not lines:
        return []

    cols = lines[0].split("\t")
    records = []

    for line in lines[1:]:
        vals = line.split("\t")
        rec = {}
        for col, val in zip(cols, vals):
            val = val.strip()
            if val == "" or val == "null" or val == "~":
                rec[col] = None
            elif val.lower() == "true":
                rec[col] = True
            elif val.lower() == "false":
                rec[col] = False
            elif re.match(r"^-?\d+$", val):
                rec[col] = int(val)
            else:
                rec[col] = val
        records.append(rec)

    return records
