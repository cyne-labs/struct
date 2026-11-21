"""
XML encoder and decoder (Standard Library only).
Uses clean, standard XML tags for all datasets.
Decodes using xml.etree.ElementTree.
"""

import xml.etree.ElementTree as ET
import re

def _escape_xml(s):
    if s is None:
        return ""
    return str(s).replace("&", "&amp;").replace("<", "&lt;").replace(">", "&gt;").replace('"', "&quot;")

def _dump_xml_elem(key, val, indent=4):
    sp = " " * indent
    if val is None:
        return f"{sp}<{key}/>\n"
    if isinstance(val, bool):
        bstr = "true" if val else "false"
        return f"{sp}<{key}>{bstr}</{key}>\n"
    if isinstance(val, (int, float)):
        return f"{sp}<{key}>{val}</{key}>\n"
    if isinstance(val, str):
        return f"{sp}<{key}>{_escape_xml(val)}</{key}>\n"
    if isinstance(val, dict):
        lines = [f"{sp}<{key}>\n"]
        for sub_k, sub_v in val.items():
            lines.append(_dump_xml_elem(sub_k, sub_v, indent + 2))
        lines.append(f"{sp}</{key}>\n")
        return "".join(lines)
    if isinstance(val, list):
        if len(val) == 0:
            return f"{sp}<{key}/>\n"
        # Determine child tag name
        child_tag = key[:-1] if key.endswith("s") else "item"
        lines = [f"{sp}<{key}>\n"]
        for item in val:
            lines.append(_dump_xml_elem(child_tag, item, indent + 2))
        lines.append(f"{sp}</{key}>\n")
        return "".join(lines)
    return f"{sp}<{key}>{_escape_xml(str(val))}</{key}>\n"

def encode(data, shape_name=None):
    """Encodes dataset into XML-style tags."""
    if not isinstance(data, list):
        raise ValueError("Data must be a list of records")

    lines = ["<claims>\n"]
    for rec in data:
        lines.append("  <claim>\n")
        for k, v in rec.items():
            lines.append(_dump_xml_elem(k, v, indent=4))
        lines.append("  </claim>\n")
    lines.append("</claims>\n")
    return "".join(lines)

def _parse_xml_node(elem):
    if elem.tag in ("notes", "subnotes") or elem.attrib.get("type") == "list":
        if len(elem) == 0:
            return []
        return [_parse_xml_node(c) for c in elem]

    # If children exist
    if len(elem) > 0:
        # Check if list (all children same tag, or parent is plural)
        child_tags = [c.tag for c in elem]
        if len(set(child_tags)) == 1 and child_tags[0] in ("note", "subnote", "item"):
            return [_parse_xml_node(c) for c in elem]
        else:
            d = {}
            for c in elem:
                d[c.tag] = _parse_xml_node(c)
            return d

    # Leaf
    text = elem.text
    if text is None or text.strip() == "" or text.strip() in ("null", "~"):
        return None
    text = text.strip()
    if text == "true":
        return True
    if text == "false":
        return False
    if re.match(r"^-?\d+$", text):
        return int(text)
    return text

def decode(text, shape_name=None):
    """Decodes XML back to list of record dicts."""
    root = ET.fromstring(text.strip())
    records = []
    for claim in root:
        rec = {}
        for field in claim:
            rec[field.tag] = _parse_xml_node(field)
        records.append(rec)
    return records
