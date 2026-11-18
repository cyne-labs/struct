"""
Fixture generator for Milestone R1.
Generates Struct v0.2 fixtures following SPEC.md v0.2.
"""

import os

FIXTURES_DIR = os.path.join(os.path.dirname(__file__), "fixtures")

# 1. Flat Tabular
FLAT_DATA_LINES = [
    "2405.05254|Rope scaling hurts long ctx|refuted|2024|0|",
    "1706.03762|Attention is all you need|supported|2017|1|",
    "2106.09685|LoRA low rank adaptation|supported|2021|1|",
    "2309.06180|Mistral 7B architecture|supported|2023|1|",
    "2401.04088|DeepSeekMoE mixture experts|supported|2024|1|",
    "2005.14165|Language models few-shot|supported|2020|1|",
    "2402.14905|StarCoder 2 training code|supported|2024|1|",
    "2310.06825|Mistral fine tuning rules|unverified|2023|0|",
    "2203.15556|Chinchilla scaling compute|supported|2022|1|",
    "2305.18290|Direct preference optim|supported|2023|1|",
    "2407.21783|Llama 3 herd models tech|supported|2024|1|",
    "2312.11805|Gemini models multimodal|supported|2023|1|",
    "2403.09611|Gemma open models Google|supported|2024|1|",
    "2307.09288|Llama 2 foundation chat|supported|2023|1|",
    "2210.03629|ReAct syner reasoning act|supported|2022|1|",
    "2404.14219|Phi 3 technical report|supported|2024|1|",
    "2302.13971|LLaMA efficient found LM|supported|2023|1|",
    "2408.03314|Qwen2 technical report|supported|2024|1|",
    "2308.12950|Code Llama open model|supported|2023|1|",
    "2406.11794|Nemotron 4 340B report|unverified|2024|0|",
]

# Typed schema (matching types explicitly)
FLAT_SCHEMA_TYPED = "schema:claim{id, title, verdict:enum(supported,refuted,unverified), year:int, verified:bool}\n"
# Untyped schema (default str, exactly matching M0's field names)
FLAT_SCHEMA_UNTYPED = "schema:claim{id, title, verdict:enum(supported,refuted,unverified), year, verified}\n"

# 2. Nested Fixed
NESTED_FIXED_DATA_LINES = [
    "Rope scaling hurts long ctx|refuted|2405.05254|2024|0|",
    "Attention is all you need|supported|1706.03762|2017|1|",
    "LoRA low rank adaptation|supported|2106.09685|2021|1|",
    "Mistral 7B architecture|supported|2309.06180|2023|1|",
    "DeepSeekMoE mixture experts|supported|2401.04088|2024|1|",
    "Language models few-shot|supported|2005.14165|2020|1|",
    "StarCoder 2 training code|supported|2402.14905|2024|1|",
    "Mistral fine tuning rules|unverified|2310.06825|2023|0|",
    "Chinchilla scaling compute|supported|2203.15556|2022|1|",
    "Direct preference optim|supported|2305.18290|2023|1|",
]

NESTED_FIXED_SCHEMA_TYPED = "schema:claim{text, verdict:enum(supported,refuted,unverified), source:record{id, year:int}, verified:bool}\n"
NESTED_FIXED_SCHEMA_UNTYPED = "schema:claim{text, verdict:enum(supported,refuted,unverified), source:record{id,year}, verified}\n"

# 3. Nested Variable
NESTED_VARIABLE_DATA_LINES = [
    "Rope scaling hurts long ctx|refuted|2405.05254|2024|",
    ">notes are from abstract only|",
    ">>abstract mentions 32k limit|",
    ">>appendix B clarifies|",
    ">table 3 contradicts|",
    ">>llama 7b tested only|",
    "LoRA matches full finetune|supported|2106.09685|2021|",
    ">tested on GLUE and Wikisql|",
    ">>rank 4 sufficient|",
    ">>higher ranks show diminishing returns|",
    ">memory footprint reduced 3x|",
    ">>optimizer state minimal|",
    ">checkpoint size under 50MB|",
    "KV cache quant loses 2x acc|refuted|2406.12345|2024|",
    ">outlier channels preserved|",
    ">>per-head scales help|",
    ">>perplexity intact|",
    "Chinchilla tokens beat params|supported|2203.15556|2022|",
    ">optimal ratio is 20 to 1|",
    ">>tested across 400 models|",
    ">>cosine schedule used|",
    ">kaplan tokens were suboptimal|",
    ">>fixed learning rate flawed|",
]

NESTED_VARIABLE_SCHEMA_TYPED = "schema:claim{text, verdict:enum(supported,refuted,unknown), source:record{id, year:int}, notes:list[record{text, subnotes:list[str]}]}\n"
NESTED_VARIABLE_SCHEMA_UNTYPED = "schema:claim{text, verdict:enum(supported,refuted,unknown), source:record{id,year}, notes:list[record{text, subnotes:list[str]}]}\n"

def main():
    os.makedirs(FIXTURES_DIR, exist_ok=True)

    # Flat
    data_str = "\n".join(FLAT_DATA_LINES) + "\n"
    with open(os.path.join(FIXTURES_DIR, "flat_tabular.v02.stc"), "w", encoding="utf-8") as f:
        f.write(FLAT_SCHEMA_TYPED + data_str)
    with open(os.path.join(FIXTURES_DIR, "flat_tabular.v02.untyped.stc"), "w", encoding="utf-8") as f:
        f.write(FLAT_SCHEMA_UNTYPED + data_str)
    with open(os.path.join(FIXTURES_DIR, "flat_tabular.v02.values_only.stc"), "w", encoding="utf-8") as f:
        f.write(data_str)
    with open(os.path.join(FIXTURES_DIR, "flat_tabular.v02.schema.stc"), "w", encoding="utf-8") as f:
        f.write(FLAT_SCHEMA_TYPED)

    # Nested Fixed
    data_str = "\n".join(NESTED_FIXED_DATA_LINES) + "\n"
    with open(os.path.join(FIXTURES_DIR, "nested_fixed.v02.stc"), "w", encoding="utf-8") as f:
        f.write(NESTED_FIXED_SCHEMA_TYPED + data_str)
    with open(os.path.join(FIXTURES_DIR, "nested_fixed.v02.untyped.stc"), "w", encoding="utf-8") as f:
        f.write(NESTED_FIXED_SCHEMA_UNTYPED + data_str)
    with open(os.path.join(FIXTURES_DIR, "nested_fixed.v02.values_only.stc"), "w", encoding="utf-8") as f:
        f.write(data_str)
    with open(os.path.join(FIXTURES_DIR, "nested_fixed.v02.schema.stc"), "w", encoding="utf-8") as f:
        f.write(NESTED_FIXED_SCHEMA_TYPED)

    # Nested Variable
    data_str = "\n".join(NESTED_VARIABLE_DATA_LINES) + "\n"
    with open(os.path.join(FIXTURES_DIR, "nested_variable.v02.stc"), "w", encoding="utf-8") as f:
        f.write(NESTED_VARIABLE_SCHEMA_TYPED + data_str)
    with open(os.path.join(FIXTURES_DIR, "nested_variable.v02.untyped.stc"), "w", encoding="utf-8") as f:
        f.write(NESTED_VARIABLE_SCHEMA_UNTYPED + data_str)
    with open(os.path.join(FIXTURES_DIR, "nested_variable.v02.values_only.stc"), "w", encoding="utf-8") as f:
        f.write(data_str)
    with open(os.path.join(FIXTURES_DIR, "nested_variable.v02.schema.stc"), "w", encoding="utf-8") as f:
        f.write(NESTED_VARIABLE_SCHEMA_TYPED)

    print("Successfully generated all v0.2 fixtures in", FIXTURES_DIR)

if __name__ == "__main__":
    main()
