"""
Minified JSON encoder and decoder (Standard Library only).
"""

import json

def encode(data, shape_name=None):
    """Encodes Python data structure as minified JSON."""
    return json.dumps(data, separators=(',', ':'), ensure_ascii=False)

def decode(text, shape_name=None):
    """Decodes minified JSON back to Python data structure."""
    return json.loads(text)
