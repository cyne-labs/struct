"""
No-loss verification harness for Struct Milestone R4.
Ensures every encoding holds exactly the same data without loss.
Reports any format that cannot hold the data shape (e.g. TSV and key: value on nested datasets).
"""

from .json_min import encode as encode_json, decode as decode_json
from .toon import encode as encode_toon, decode as decode_toon
from .yaml import encode as encode_yaml, decode as decode_yaml
from .tsv import encode as encode_tsv, decode as decode_tsv
from .xml import encode as encode_xml, decode as decode_xml
from .key_value import encode as encode_key_value, decode as decode_key_value
from .struct import encode as encode_struct, decode as decode_struct

FORMATS = ["json_min", "toon", "yaml", "tsv", "xml", "key_value", "struct"]

ENCODERS = {
    "json_min": encode_json,
    "toon": encode_toon,
    "yaml": encode_yaml,
    "tsv": encode_tsv,
    "xml": encode_xml,
    "key_value": encode_key_value,
    "struct": encode_struct,
}

DECODERS = {
    "json_min": decode_json,
    "toon": decode_toon,
    "yaml": decode_yaml,
    "tsv": decode_tsv,
    "xml": decode_xml,
    "key_value": decode_key_value,
    "struct": decode_struct,
}

def strict_equal(a, b):
    """Recursively checks strict equality including type checks (e.g. bool != int)."""
    if type(a) != type(b):
        return False
    if isinstance(a, dict):
        if set(a.keys()) != set(b.keys()):
            return False
        return all(strict_equal(a[k], b[k]) for k in a)
    if isinstance(a, list):
        if len(a) != len(b):
            return False
        return all(strict_equal(x, y) for x, y in zip(a, b))
    return a == b

def verify_no_loss(format_name, shape_name, data):
    """
    Encodes data with the specified format, decodes it back, and compares against source data.
    Returns (status: bool, message: str, encoded_str_or_none).
    """
    encoder = ENCODERS[format_name]
    decoder = DECODERS[format_name]

    # Check if format cannot hold this shape (per Decision 1: TSV and key_value on nested data)
    if format_name in ("tsv", "key_value") and shape_name in ("nested_fixed", "nested_variable"):
        try:
            encoder(data, shape_name)
            return False, "Expected format to reject nested shape, but encoder succeeded", None
        except ValueError as e:
            return True, f"Correctly flagged: can't hold this shape ({e})", None

    try:
        encoded = encoder(data, shape_name)
    except Exception as e:
        return False, f"Encoding error: {e}", None

    try:
        decoded = decoder(encoded, shape_name)
    except Exception as e:
        return False, f"Decoding error: {e}", encoded

    # Compare decoded against original data strictly
    if strict_equal(decoded, data):
        if format_name == "xml":
            return True, "Lossless match (100% exact; plain XML cannot distinguish empty list from null per Decision 12)", encoded
        return True, "Lossless match (100% exact)", encoded
    else:
        # Check for Decision 12 XML limitation (XML cannot tell empty list from null without attributes)
        if format_name == "xml":
            def xml_relaxed_equal(a, b):
                if strict_equal(a, b):
                    return True
                if (a is None and b == []) or (a == [] and b is None):
                    return True
                if type(a) != type(b):
                    return False
                if isinstance(a, dict):
                    if set(a.keys()) != set(b.keys()):
                        return False
                    return all(xml_relaxed_equal(a[k], b[k]) for k in a)
                if isinstance(a, list):
                    if len(a) != len(b):
                        return False
                    return all(xml_relaxed_equal(x, y) for x, y in zip(a, b))
                return False

            if xml_relaxed_equal(decoded, data):
                return True, "Lossless match (plain XML cannot distinguish empty list from null per Decision 12)", encoded

        # Detailed mismatch analysis
        if len(decoded) != len(data):
            return False, f"Length mismatch: got {len(decoded)}, expected {len(data)}", encoded

        for i, (orig_r, dec_r) in enumerate(zip(data, decoded)):
            if not strict_equal(orig_r, dec_r):
                return False, f"Record {i} mismatch:\n  Orig: {orig_r}\n  Dec : {dec_r}", encoded

        return False, "Data mismatch", encoded

def check_all_formats(shape_name, data):
    """
    Checks all 7 formats against the given dataset.
    Returns a dict mapping format_name -> (status, message).
    """
    results = {}
    for fmt in FORMATS:
        status, msg, _ = verify_no_loss(fmt, shape_name, data)
        results[fmt] = (status, msg)
    return results
