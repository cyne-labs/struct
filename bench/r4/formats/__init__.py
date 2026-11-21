"""
Formats package for Struct Milestone R4.
Provides encoders and decoders for 7 formats:
1. Minified JSON
2. TOON
3. YAML
4. TSV (flat datasets only)
5. XML-style tags
6. Plain key: value lines (flat datasets only)
7. Struct (typed, with schema; v0.2 form from R1)
"""

from .json_min import encode as encode_json, decode as decode_json
from .toon import encode as encode_toon, decode as decode_toon
from .yaml import encode as encode_yaml, decode as decode_yaml
from .tsv import encode as encode_tsv, decode as decode_tsv
from .xml import encode as encode_xml, decode as decode_xml
from .key_value import encode as encode_key_value, decode as decode_key_value
from .struct import encode as encode_struct, decode as decode_struct
from .no_loss import verify_no_loss, check_all_formats

FORMATS = [
    "json_min",
    "toon",
    "yaml",
    "tsv",
    "xml",
    "key_value",
    "struct",
]

FORMAT_DISPLAY_NAMES = {
    "json_min": "Minified JSON",
    "toon": "TOON",
    "yaml": "YAML",
    "tsv": "TSV",
    "xml": "XML",
    "key_value": "Plain key: value",
    "struct": "Struct",
}

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
