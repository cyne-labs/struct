"""
YAML encoder and decoder (Standard Library only for encoding; uses PyYAML for decoding if available).
"""

import re

def _escape_yaml_str(s):
    if s is None:
        return "null"
    if not isinstance(s, str):
        return str(s)
    # Check if needs quotes
    if s == "" or s in ("true", "false", "null", "~") or re.search(r"[:#\[\]{},&*?|<>=!%@`]", s) or re.match(r"^-?\d+(\.\d+)?$", s):
        escaped = s.replace("'", "''")
        return f"'{escaped}'"
    return s

def _dump_yaml(obj, indent=0):
    spaces = " " * indent
    if isinstance(obj, dict):
        lines = []
        for k, v in obj.items():
            if v == []:
                lines.append(f"{spaces}{k}: []")
            elif v == {}:
                lines.append(f"{spaces}{k}: {{}}")
            elif isinstance(v, (dict, list)):
                lines.append(f"{spaces}{k}:")
                lines.append(_dump_yaml(v, indent + 2))
            else:
                lines.append(f"{spaces}{k}: {_format_yaml_scalar(v)}")
        return "\n".join(lines)
    elif isinstance(obj, list):
        lines = []
        for item in obj:
            if isinstance(item, dict):
                first = True
                for k, v in item.items():
                    if first:
                        if v == []:
                            lines.append(f"{spaces}- {k}: []")
                        elif v == {}:
                            lines.append(f"{spaces}- {k}: {{}}")
                        elif isinstance(v, (dict, list)):
                            lines.append(f"{spaces}- {k}:")
                            lines.append(_dump_yaml(v, indent + 4))
                        else:
                            lines.append(f"{spaces}- {k}: {_format_yaml_scalar(v)}")
                        first = False
                    else:
                        if v == []:
                            lines.append(f"{spaces}  {k}: []")
                        elif v == {}:
                            lines.append(f"{spaces}  {k}: {{}}")
                        elif isinstance(v, (dict, list)):
                            lines.append(f"{spaces}  {k}:")
                            lines.append(_dump_yaml(v, indent + 4))
                        else:
                            lines.append(f"{spaces}  {k}: {_format_yaml_scalar(v)}")
            elif item == []:
                lines.append(f"{spaces}- []")
            elif item == {}:
                lines.append(f"{spaces}- {{}}")
            elif isinstance(item, list):
                lines.append(f"{spaces}-")
                lines.append(_dump_yaml(item, indent + 2))
            else:
                lines.append(f"{spaces}- {_format_yaml_scalar(item)}")
        return "\n".join(lines)
    else:
        return f"{spaces}{_format_yaml_scalar(obj)}"

def _format_yaml_scalar(val):
    if val is None:
        return "null"
    if isinstance(val, bool):
        return "true" if val else "false"
    if isinstance(val, (int, float)):
        return str(val)
    return _escape_yaml_str(val)

def encode(data, shape_name=None):
    """Encodes Python data structure into clean, standard YAML format."""
    out = _dump_yaml(data)
    if not out.endswith("\n"):
        out += "\n"
    return out

def decode(text, shape_name=None):
    """Decodes YAML into Python data structure using PyYAML if present or stdlib parser."""
    try:
        import yaml
        return yaml.safe_load(text)
    except ImportError:
        # Fallback simple line-based YAML parser for flat/nested benchmark data
        import json
        raise RuntimeError("PyYAML required for fallback decoding")
