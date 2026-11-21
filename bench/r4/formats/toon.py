"""
TOON encoder and decoder (Standard Library only).
Encodes per TOON specification:
- Flat tabular: [N]{col1,col2,...}:\n  val1,val2,...
- Nested fixed (tabular with nested field group): [N]{text,verdict,source{id,year},verified}:\n  val1,val2,val3,val4,val5
- Nested variable: [N]:\n  - text: ...\n    notes[M]:\n      - text: ...\n        subnotes[K]: ... (subnotes: [] for empty)

Per TOON spec (and R4 Decision 13):
- Tabular headers support uniform nested objects via Nested Field Groups (e.g. source{id,year}).
- Strings that look like numbers or booleans, or contain delimiters, are quoted.
- Empty arrays emit key: [] per TOON spec 9.1 (e.g. subnotes: []).
"""

import re

def _format_scalar(val):
    if val is None:
        return "null"
    if isinstance(val, bool):
        return "true" if val else "false"
    if isinstance(val, (int, float)):
        return str(val)
    s = str(val)
    # Per TOON spec: quote strings that look like numbers, booleans, null, or contain delimiters/quotes
    if (
        re.match(r"^-?\d+(\.\d+)?$", s)
        or s.lower() in ("true", "false", "null", "~")
        or "," in s
        or '"' in s
        or ":" in s
        or s.startswith("-")
        or s.startswith("#")
    ):
        escaped = s.replace('"', '\\"')
        return f'"{escaped}"'
    return s

def _split_row(line):
    """Splits a comma-separated row while preserving quoted strings."""
    parts = []
    current = []
    in_quote = False
    quote_char = None
    for ch in line.strip():
        if not in_quote and ch in ('"', "'"):
            in_quote = True
            quote_char = ch
            current.append(ch)
        elif in_quote and ch == quote_char:
            in_quote = False
            quote_char = None
            current.append(ch)
        elif not in_quote and ch == ',':
            parts.append("".join(current).strip())
            current = []
        else:
            current.append(ch)
    if current:
        parts.append("".join(current).strip())
    return parts

def encode(data, shape_name=None):
    if not isinstance(data, list):
        raise ValueError("Data must be a list of records")
    n = len(data)
    if n == 0:
        return "[0]:\n"

    first = data[0]

    # Detect shape if shape_name not given
    if shape_name is None:
        if "notes" in first:
            shape_name = "nested_variable"
        elif "source" in first and isinstance(first["source"], dict) and "verified" in first:
            shape_name = "nested_fixed"
        else:
            shape_name = "flat_tabular"

    if shape_name in ("flat_tabular", "flat_100"):
        cols = list(first.keys())
        header = f"[{n}]{{{','.join(cols)}}}:\n"
        lines = [header]
        for row in data:
            vals = [_format_scalar(row.get(col)) for col in cols]
            lines.append("  " + ",".join(vals) + "\n")
        return "".join(lines)

    elif shape_name == "nested_fixed":
        header = f"[{n}]{{text,verdict,source{{id,year}},verified}}:\n"
        lines = [header]
        for row in data:
            src = row.get("source") or {}
            vals = [
                _format_scalar(row.get("text")),
                _format_scalar(row.get("verdict")),
                _format_scalar(src.get("id")),
                _format_scalar(src.get("year")),
                _format_scalar(row.get("verified")),
            ]
            lines.append("  " + ",".join(vals) + "\n")
        return "".join(lines)

    elif shape_name == "nested_variable":
        lines = [f"[{n}]:\n"]
        for row in data:
            lines.append(f"  - text: {_format_scalar(row.get('text'))}\n")
            lines.append(f"    verdict: {_format_scalar(row.get('verdict'))}\n")
            src = row.get("source") or {}
            lines.append("    source:\n")
            lines.append(f"      id: {_format_scalar(src.get('id'))}\n")
            lines.append(f"      year: {_format_scalar(src.get('year'))}\n")
            notes = row.get("notes") or []
            if not notes:
                lines.append("    notes: []\n")
            else:
                lines.append(f"    notes[{len(notes)}]:\n")
                for note in notes:
                    lines.append(f"      - text: {_format_scalar(note.get('text'))}\n")
                    subnotes = note.get("subnotes") or []
                    if not subnotes:
                        lines.append("        subnotes: []\n")
                    else:
                        sub_str = ",".join(_format_scalar(s) for s in subnotes)
                        lines.append(f"        subnotes[{len(subnotes)}]: {sub_str}\n")
        return "".join(lines)
    else:
        raise ValueError(f"Unknown shape {shape_name}")

def _parse_scalar(val):
    val = val.strip()
    # If quoted string, strip quotes and unescape
    if (val.startswith('"') and val.endswith('"')) or (val.startswith("'") and val.endswith("'")):
        inner = val[1:-1].replace('\\"', '"')
        return inner
    if val == "null" or val == "~":
        return None
    if val == "true":
        return True
    if val == "false":
        return False
    if re.match(r"^-?\d+$", val):
        return int(val)
    return val

def decode(text, shape_name=None):
    lines = [l.rstrip("\r\n") for l in text.splitlines() if l.strip()]
    if not lines:
        return []

    header = lines[0]
    # Check if tabular header [N]{...}:
    m_tab = re.match(r"^\[(\d+)\]\{(.*)\}:$", header)
    if m_tab:
        n = int(m_tab.group(1))
        spec = m_tab.group(2)
        if "source{id,year}" in spec or shape_name == "nested_fixed":
            # Nested fixed tabular with nested field group
            records = []
            for line in lines[1:]:
                parts = _split_row(line)
                if len(parts) >= 5:
                    rec = {
                        "text": _parse_scalar(parts[0]),
                        "verdict": _parse_scalar(parts[1]),
                        "source": {
                            "id": _parse_scalar(parts[2]),
                            "year": _parse_scalar(parts[3]),
                        },
                        "verified": _parse_scalar(parts[4]),
                    }
                    records.append(rec)
            return records
        else:
            # Flat tabular
            cols = [c.strip() for c in spec.split(",")]
            records = []
            for line in lines[1:]:
                parts = _split_row(line)
                rec = {}
                for col, val in zip(cols, parts):
                    rec[col] = _parse_scalar(val)
                records.append(rec)
            return records

    # Otherwise nested variable: [N]:
    records = []
    current_rec = None
    current_note = None

    for line in lines[1:]:
        stripped = line.strip()
        indent = len(line) - len(line.lstrip(" "))

        if indent == 2 and stripped.startswith("- text:"):
            text_val = stripped[len("- text:"):].strip()
            current_rec = {
                "text": _parse_scalar(text_val),
                "verdict": None,
                "source": {},
                "notes": []
            }
            records.append(current_rec)
        elif indent == 4 and stripped.startswith("verdict:"):
            current_rec["verdict"] = _parse_scalar(stripped[len("verdict:"):])
        elif indent == 6 and stripped.startswith("id:"):
            current_rec["source"]["id"] = _parse_scalar(stripped[len("id:"):])
        elif indent == 6 and stripped.startswith("year:"):
            current_rec["source"]["year"] = _parse_scalar(stripped[len("year:"):])
        elif indent == 4 and (stripped == "notes: []" or stripped == "notes[0]:"):
            current_rec["notes"] = []
        elif indent == 6 and stripped.startswith("- text:"):
            note_text = stripped[len("- text:"):].strip()
            current_note = {"text": _parse_scalar(note_text), "subnotes": []}
            current_rec["notes"].append(current_note)
        elif indent == 8 and (stripped == "subnotes: []" or stripped == "subnotes[0]:"):
            current_note["subnotes"] = []
        elif indent == 8 and re.match(r"^subnotes\[\d+\]:", stripped):
            content = stripped.split(":", 1)[1].strip()
            if content:
                subs = [_parse_scalar(s) for s in _split_row(content)]
                current_note["subnotes"] = subs
            else:
                current_note["subnotes"] = []

    return records
