"""
Struct encoder and decoder (Standard Library only).
Uses the typed, with schema v0.2 form from Milestone R1.
Can decode natively in Python or via doctor_tool.exe.
"""

import os
import re
import json
import subprocess

SCHEMAS = {
    "flat_tabular": "schema:claim{id, title, verdict:enum(supported,refuted,unverified), year:int, verified:bool}\n",
    "flat_100": "schema:claim{id, title, verdict:enum(supported,refuted,unverified), year:int, verified:bool}\n",
    "nested_fixed": "schema:claim{text, verdict:enum(supported,refuted,unverified), source:record{id, year:int}, verified:bool}\n",
    "nested_variable": "schema:claim{text, verdict:enum(supported,refuted,unknown), source:record{id, year:int}, notes:list[record{text, subnotes:list[str]}]}\n",
}

def _format_struct_scalar(val):
    if val is None:
        return "~"
    if isinstance(val, bool):
        return "1" if val else "0"
    s = str(val)
    s = s.replace("\\", "\\\\").replace("|", "\\|")
    return s

def encode(data, shape_name=None):
    """Encodes dataset into Struct format with schema."""
    if not isinstance(data, list):
        raise ValueError("Data must be a list of records")
    if not data:
        return ""

    first = data[0]
    if shape_name is None:
        if "notes" in first:
            shape_name = "nested_variable"
        elif "source" in first and isinstance(first["source"], dict) and "verified" in first:
            shape_name = "nested_fixed"
        else:
            shape_name = "flat_tabular"

    schema_line = SCHEMAS.get(shape_name, SCHEMAS["flat_tabular"])
    lines = [schema_line]

    if shape_name in ("flat_tabular", "flat_100"):
        for row in data:
            c_id = _format_struct_scalar(row.get("id"))
            c_title = _format_struct_scalar(row.get("title"))
            c_verdict = _format_struct_scalar(row.get("verdict"))
            c_year = _format_struct_scalar(row.get("year"))
            c_ver = _format_struct_scalar(row.get("verified"))
            lines.append(f"{c_id}|{c_title}|{c_verdict}|{c_year}|{c_ver}|\n")

    elif shape_name == "nested_fixed":
        for row in data:
            c_text = _format_struct_scalar(row.get("text"))
            c_verdict = _format_struct_scalar(row.get("verdict"))
            src = row.get("source") or {}
            c_src_id = _format_struct_scalar(src.get("id"))
            c_src_year = _format_struct_scalar(src.get("year"))
            c_ver = _format_struct_scalar(row.get("verified"))
            lines.append(f"{c_text}|{c_verdict}|{c_src_id}|{c_src_year}|{c_ver}|\n")

    elif shape_name == "nested_variable":
        for row in data:
            c_text = _format_struct_scalar(row.get("text"))
            c_verdict = _format_struct_scalar(row.get("verdict"))
            src = row.get("source") or {}
            c_src_id = _format_struct_scalar(src.get("id"))
            c_src_year = _format_struct_scalar(src.get("year"))
            lines.append(f"{c_text}|{c_verdict}|{c_src_id}|{c_src_year}|\n")

            notes = row.get("notes") or []
            for note in notes:
                n_text = _format_struct_scalar(note.get("text"))
                lines.append(f">{n_text}|\n")
                subnotes = note.get("subnotes") or []
                for sub in subnotes:
                    s_text = _format_struct_scalar(sub)
                    lines.append(f">>{s_text}|\n")

    return "".join(lines)

def _parse_struct_scalar(val, field_type=None):
    if val == "~":
        return None
    val = val.replace("\\|", "|").replace("\\\\", "\\")
    if field_type == "bool":
        return val == "1"
    if field_type == "int":
        return int(val) if val != "~" else None
    return val

def decode(text, shape_name=None):
    """Decodes Struct text back to Python data structure."""
    lines = [l.strip() for l in text.splitlines() if l.strip()]
    if not lines:
        return []

    data_lines = [l for l in lines if not l.startswith("schema:")]
    if not data_lines:
        return []

    # Infer shape if not given
    if shape_name is None:
        if any(l.startswith(">") for l in data_lines):
            shape_name = "nested_variable"
        else:
            # Check column count of first line
            cols = [c for c in data_lines[0].split("|")[:-1]]
            if len(cols) == 5:
                # Could be flat or nested_fixed
                shape_name = "flat_tabular"

    if shape_name in ("flat_tabular", "flat_100"):
        records = []
        for line in data_lines:
            parts = line.split("|")[:-1]
            if len(parts) >= 5:
                rec = {
                    "id": _parse_struct_scalar(parts[0], "str"),
                    "title": _parse_struct_scalar(parts[1], "str"),
                    "verdict": _parse_struct_scalar(parts[2], "str"),
                    "year": _parse_struct_scalar(parts[3], "int"),
                    "verified": _parse_struct_scalar(parts[4], "bool"),
                }
                records.append(rec)
        return records

    elif shape_name == "nested_fixed":
        records = []
        for line in data_lines:
            parts = line.split("|")[:-1]
            if len(parts) >= 5:
                rec = {
                    "text": _parse_struct_scalar(parts[0], "str"),
                    "verdict": _parse_struct_scalar(parts[1], "str"),
                    "source": {
                        "id": _parse_struct_scalar(parts[2], "str"),
                        "year": _parse_struct_scalar(parts[3], "int"),
                    },
                    "verified": _parse_struct_scalar(parts[4], "bool"),
                }
                records.append(rec)
        return records

    elif shape_name == "nested_variable":
        records = []
        current_rec = None
        current_note = None

        for line in data_lines:
            if line.startswith(">>"):
                # Subnote
                val = line[2:].rstrip("|")
                if current_note is not None:
                    current_note["subnotes"].append(_parse_struct_scalar(val, "str"))
            elif line.startswith(">"):
                # Note
                val = line[1:].rstrip("|")
                current_note = {"text": _parse_struct_scalar(val, "str"), "subnotes": []}
                if current_rec is not None:
                    current_rec["notes"].append(current_note)
            else:
                # Top-level claim
                parts = line.split("|")[:-1]
                if len(parts) >= 4:
                    current_rec = {
                        "text": _parse_struct_scalar(parts[0], "str"),
                        "verdict": _parse_struct_scalar(parts[1], "str"),
                        "source": {
                            "id": _parse_struct_scalar(parts[2], "str"),
                            "year": _parse_struct_scalar(parts[3], "int"),
                        },
                        "notes": []
                    }
                    records.append(current_rec)
                    current_note = None

        return records

    return []
