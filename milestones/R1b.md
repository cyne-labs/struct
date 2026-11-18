# R1b: fix-up after R1 review

Read `RULES.md` and `SPEC.md` first. SPEC.md was updated after R1. It now writes down several rules R1 had implemented without them being in the spec, adds one new error, and settles records that contain lists. Change only what this brief lists. Where SPEC.md doesn't answer something, stop and ask (rule 5).

## 1. Code changes

1. **`+` on an int is an error.** `+5` in an `int` field must fail with a line and column, not decode as 5.
2. **Records that contain a list are never null** (SPEC section 3). If a record has a list anywhere inside it, at any depth:
   - decoding a line where all of its columns are `~` gives a record with null values and its lists, which are empty when there are no child lines. It never gives null;
   - this must hold whether or not child lines follow.
   Records with no list inside still decode to null when all their columns are `~`, exactly as now.
3. **One meaning for a null record in the encoder.** A record with no list inside it can arrive as `Value::Null` or as a record whose fields are all null. The encoder accepts both and writes the same text, and `decode(encode(x))` returns `Value::Null` either way.
4. **Encoder errors.** Handing `encode` a `Value::Null` where a list belongs, or for a record that contains a list, is an error. Since there's no source line, set the line and column to 0 and say in the message which class and field it was.

## 2. Tests

1. **Negative tests, one per rule, each asserting the exact line, the exact column and the full exact message** (not a substring). Cover every error in SPEC.md, including those added since R1:
   - a data line before any schema line;
   - reusing an undefined class;
   - defining a class twice;
   - an invalid class name, field name and enum label;
   - a duplicate field name;
   - an unknown type name;
   - a schema line containing `|`;
   - a data line not ending in an unescaped `|`;
   - too few columns and too many columns, for a top-level line and for a child line;
   - one type mismatch each for `int`, `float`, `bool` and `enum`;
   - `+5` in an int;
   - an int out of `i64` range;
   - an empty field;
   - an unknown backslash sequence;
   - a depth rise of more than one, including a `>` line straight after the schema line;
   - a child line under a parent with no list;
   - a missing index when one is needed;
   - an index that names a non-list field, and one out of range;
   - each encoder error from 1.4.

   Update the existing 10 tests to the same standard.
2. **Positive tests for 1.2 and 1.3:**
   - an all-`~` list-holding record, with and without child lines;
   - an all-`~` record with no list, decoding to null;
   - both encoder input forms of a null record producing identical text.
3. **Round trips.** Change the random generator so it also produces records whose fields are all null (not only `Value::Null`) and all-`~` records that contain lists. Keep at least 5,000 cases, all passing, and report the new edge-case counts.

## Done when
`cargo test --offline` passes, every SPEC error has an exact-match negative test, and the round-trip report includes the two new edge cases.

## Final message
The test counts; the list of negative tests with the error each checks; any SPEC questions (rule 5); and the plain-English transparency overview (rule 10), covering only what changed.
