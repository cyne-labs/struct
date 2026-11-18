"""
Tokenizer interface for Milestone R1.
Wraps llama.cpp vocabulary loading via llama.dll (ctypes) for ultra-fast,
in-process, CPU-only tokenization with vocab_only=True.
Silences verbose loader logs for clean benchmark execution.
"""

import os
import ctypes
from typing import List, Tuple

LLAMA_DIR = r"C:\Users\Ashman Das\Documents\turboquant-llama-cpp"
LLAMA_DLL = os.path.join(LLAMA_DIR, "llama.dll")
LLAMA_TOKENIZE_EXE = os.path.join(LLAMA_DIR, "llama-tokenize.exe")

MODELS = {
    "Qwen": r"C:\Users\Ashman Das\Documents\turboquant-llama-cpp\models\unsloth\Qwen3.5-4B-GGUF\Qwen3.5-4B-Q4_K_M.gguf",
    "Llama": r"C:\Users\Ashman Das\Documents\turboquant-llama-cpp\models\lmstudio-community\Llama-3.2-3B-Instruct-GGUF\Llama-3.2-3B-Instruct-Q4_K_M.gguf",
    "Gemma": r"C:\Users\Ashman Das\Downloads\model file\gemma-3-4b-it-Q4_K_M.gguf",
    "LFM": r"C:\Users\Ashman Das\Downloads\model file\LFM2.5-8B-A1B-Q4_K_M.gguf",
}

class llama_model_params(ctypes.Structure):
    _fields_ = [
        ("n_gpu_layers", ctypes.c_int32),
        ("split_mode", ctypes.c_int32),
        ("main_gpu", ctypes.c_int32),
        ("tensor_split", ctypes.c_void_p),
        ("progress_callback", ctypes.c_void_p),
        ("progress_callback_user_data", ctypes.c_void_p),
        ("kv_overrides", ctypes.c_void_p),
        ("vocab_only", ctypes.c_bool),
        ("use_mmap", ctypes.c_bool),
        ("use_mlock", ctypes.c_bool),
        ("check_tensors", ctypes.c_bool),
    ]

_llama_lib = None
_log_cb_ref = None

def _get_llama_lib():
    global _llama_lib, _log_cb_ref
    if _llama_lib is None:
        if not os.path.exists(LLAMA_DLL):
            raise FileNotFoundError(f"llama.dll not found at {LLAMA_DLL}")
        lib = ctypes.CDLL(LLAMA_DLL)
        if hasattr(lib, "llama_backend_init"):
            lib.llama_backend_init()

        # Silence verbose logging
        if hasattr(lib, "llama_log_set"):
            LOG_CALLBACK = ctypes.CFUNCTYPE(None, ctypes.c_int, ctypes.c_char_p, ctypes.c_void_p)
            def dummy_log(level, text, user_data):
                pass
            _log_cb_ref = LOG_CALLBACK(dummy_log)
            lib.llama_log_set(_log_cb_ref, None)

        lib.llama_model_default_params.restype = llama_model_params
        lib.llama_model_load_from_file.argtypes = [ctypes.c_char_p, llama_model_params]
        lib.llama_model_load_from_file.restype = ctypes.c_void_p

        lib.llama_model_get_vocab.argtypes = [ctypes.c_void_p]
        lib.llama_model_get_vocab.restype = ctypes.c_void_p

        lib.llama_tokenize.argtypes = [
            ctypes.c_void_p,
            ctypes.c_char_p,
            ctypes.c_int32,
            ctypes.POINTER(ctypes.c_int32),
            ctypes.c_int32,
            ctypes.c_bool,
            ctypes.c_bool,
        ]
        lib.llama_tokenize.restype = ctypes.c_int32

        lib.llama_token_to_piece.argtypes = [
            ctypes.c_void_p,
            ctypes.c_int32,
            ctypes.c_char_p,
            ctypes.c_int32,
            ctypes.c_int32,
            ctypes.c_bool,
        ]
        lib.llama_token_to_piece.restype = ctypes.c_int32

        if hasattr(lib, "llama_model_free"):
            lib.llama_model_free.argtypes = [ctypes.c_void_p]
            lib.llama_model_free.restype = None
        elif hasattr(lib, "llama_free_model"):
            lib.llama_free_model.argtypes = [ctypes.c_void_p]
            lib.llama_free_model.restype = None

        _llama_lib = lib
    return _llama_lib


class VocabTokenizer:
    def __init__(self, family: str, model_path: str):
        self.family = family
        self.model_path = model_path
        self.lib = _get_llama_lib()
        
        params = self.lib.llama_model_default_params()
        params.vocab_only = True
        params.n_gpu_layers = 0  # CPU only
        
        p = model_path.encode("utf-8")
        self.model = self.lib.llama_model_load_from_file(p, params)
        if not self.model:
            raise RuntimeError(f"Failed to load model vocab from {model_path}")
        self.vocab = self.lib.llama_model_get_vocab(self.model)

    def tokenize(self, text: str, add_special: bool = False, parse_special: bool = False) -> Tuple[int, List[int], List[str]]:
        b = text.encode("utf-8")
        max_tokens = max(len(b) * 2 + 16, 128)
        out_tokens = (ctypes.c_int32 * max_tokens)()
        
        n = self.lib.llama_tokenize(
            self.vocab, b, len(b), out_tokens, max_tokens, add_special, parse_special
        )
        if n < 0:
            max_tokens = -n + 64
            out_tokens = (ctypes.c_int32 * max_tokens)()
            n = self.lib.llama_tokenize(
                self.vocab, b, len(b), out_tokens, max_tokens, add_special, parse_special
            )

        token_ids = [out_tokens[i] for i in range(n)]
        pieces = []
        buf = ctypes.create_string_buffer(512)
        for tid in token_ids:
            length = self.lib.llama_token_to_piece(self.vocab, tid, buf, 512, 0, False)
            if length < 0:
                pieces.append("")
            else:
                pieces.append(buf.raw[:length].decode("utf-8", "replace"))
        return n, token_ids, pieces

    def close(self):
        if self.model and hasattr(self.lib, "llama_model_free"):
            self.lib.llama_model_free(self.model)
            self.model = None
        elif self.model and hasattr(self.lib, "llama_free_model"):
            self.lib.llama_free_model(self.model)
            self.model = None

    def __enter__(self):
        return self

    def __exit__(self, exc_type, exc_val, exc_tb):
        self.close()
