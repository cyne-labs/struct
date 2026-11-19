# Llama.cpp Lazy Grammars and Trigger Mechanisms: Investigation for Struct v0.3

This report documents the mechanism of lazy grammars and triggers in `llama.cpp` (and `llama-server`), based on direct inspection of the server and sampler source code in `llama.cpp` (`tools/server/`, `common/sampling.cpp`, `src/llama-sampler.cpp`, `src/llama-grammar.cpp`).

---

## 1. Request Fields

When interacting with `llama-server` (e.g. at `/completion` or `/v1/chat/completions`), the following request parameters configure lazy grammar sampling:

- **`grammar`** (`string`): The full GBNF grammar specification text.
- **`grammar_lazy`** (`bool`, default `false`): When set to `true`, enables lazy grammar evaluation. The grammar sampler initializes in an `awaiting_trigger` state. In this state, normal token sampling proceeds unrestricted by the grammar until a trigger fires.
- **`grammar_triggers`** (`array` of JSON objects): The list of triggers that activate the grammar. Each trigger object contains:
  - `"type"` (`int`): The trigger type enum:
    - `0`: `COMMON_GRAMMAR_TRIGGER_TYPE_TOKEN`
    - `1`: `COMMON_GRAMMAR_TRIGGER_TYPE_WORD`
    - `2`: `COMMON_GRAMMAR_TRIGGER_TYPE_PATTERN`
    - `3`: `COMMON_GRAMMAR_TRIGGER_TYPE_PATTERN_FULL`
  - `"value"` (`string`): The trigger string or regular expression pattern (e.g. `"</think>"`).
  - `"token"` (`int`, optional): Explicit token ID when `type == 0`.
- **`preserved_tokens`** (`array` of `string`s): List of token strings that must not be split into multiple pieces during tokenization.
  - *Crucial requirement in `common/sampling.cpp`:* When a trigger word (`COMMON_GRAMMAR_TRIGGER_TYPE_WORD`) maps to a single token in the vocabulary, `common_sampling_add_grammar_triggers` verifies that the token is present in `preserved_tokens`. If absent, `llama-server` throws:
    ```
    "Grammar trigger word should be marked as preserved token: " + word
    ```
    Therefore, client requests using a single-token trigger word (such as `</think>`) must also include the trigger string in `preserved_tokens`.

---

## 2. Trigger Types and Resolution

In `common/common.h` and `common/sampling.cpp`, four trigger types are defined:

1. **`COMMON_GRAMMAR_TRIGGER_TYPE_TOKEN` (type 0):**
   Matches a specific token ID directly. As soon as the model samples this token ID, the trigger fires immediately.
2. **`COMMON_GRAMMAR_TRIGGER_TYPE_WORD` (type 1):**
   Matches a literal string. When registered:
   - If `common_tokenize` reveals the string consists of exactly 1 token in the model vocabulary, it is converted into a `COMMON_GRAMMAR_TRIGGER_TYPE_TOKEN` (and requires preservation in `preserved_tokens`).
   - If the string spans multiple tokens, it is converted into a regex pattern escaped via `regex_escape(word)`.
3. **`COMMON_GRAMMAR_TRIGGER_TYPE_PATTERN` (type 2):**
   A regex pattern searched anywhere within the generated text buffer (`std::regex_search`).
4. **`COMMON_GRAMMAR_TRIGGER_TYPE_PATTERN_FULL` (type 3):**
   A regex pattern anchored with `^...$` matched against the entire accumulated output prefix (`std::regex_match`).

---

## 3. Does the Grammar See the Trigger Text, or Only What Comes After It?

**Finding: The grammar SEES the trigger text itself.**

In `src/llama-grammar.cpp`:
- **For single-token triggers (`COMMON_GRAMMAR_TRIGGER_TYPE_TOKEN` or single-token `WORD`):**
  When the trigger token is generated while `grammar.awaiting_trigger` is `true`:
  ```cpp
  if (std::find(grammar.trigger_tokens.begin(), grammar.trigger_tokens.end(), token) != grammar.trigger_tokens.end()) {
      grammar.awaiting_trigger = false;
      grammar.trigger_buffer.clear();
      llama_grammar_accept_token(grammar, token, piece);
      return;
  }
  ```
  `llama_grammar_accept_token` is called on the trigger token. The grammar's starting rule must accept this token.

- **For multi-token word triggers (`COMMON_GRAMMAR_TRIGGER_TYPE_WORD`):**
  Tokens are accumulated in `grammar.trigger_buffer` with character position tracking in `trigger_buffer_positions`. When `trigger_pattern.find(grammar.trigger_buffer)` matches at offset `start`:
  Because `regex_escape(word)` has no capturing groups, `find_start_pos` returns `match.position(0)` (the beginning of the trigger string).
  The replay loop:
  ```cpp
  for (const auto & [tok, tok_pos] : grammar.trigger_buffer_positions) {
      auto [tok_start, tok_end] = tok_pos;
      if (tok_end <= start) continue;
      size_t piece_start = (tok_start < start) ? start : tok_start;
      size_t piece_len = tok_end - piece_start;
      auto tok_piece = grammar.trigger_buffer.substr(piece_start, piece_len);
      llama_grammar_accept_token(grammar, tok, tok_piece);
  }
  ```
  replays all tokens from the beginning of the trigger word into `llama_grammar_accept_token`.

Thus, in both paths, **the trigger text is accepted into the grammar**. The grammar's initial rule (`root`) must account for the trigger literal before transitioning to the structured output rules.

---

## 4. Construction of the Triggered GBNF Form

In `gbnf_triggered(schemas, trigger)`:
- The grammar must accept the literal trigger string first.
- In canonical Struct form, lines end with `\n` and have no leading empty lines. However, language models after emitting an end-of-thinking tag like `</think>` frequently emit a newline (`\n` or `\r?\n`) before emitting the structured payload.
- To maintain strict canonical validity while accommodating standard model generation conventions, the triggered root matches the trigger literal, followed optionally by a single line break (`("\n" | "\r\n")?`), transitioning directly into the canonical block and data line grammar rules.
