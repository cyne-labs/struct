# Fix: Flash Lite harness stops on a per-minute limit

Read `milestones/RULES.md`, `milestones/R4-gemini-lite.md` (decision 4 especially), `bench/r4/collect_gemini_lite.py` and `bench/r4/test_gemini_lite_offline.py`.

The first real run stopped after 16 calls with "Quota exhausted". It wasn't: it was the free tier's **per-minute** request limit (15 requests per minute for this model). The run stop is recorded in `bench/r4/real_gemini_lite/run_info.json` under `stopped_at`: read the real error there. The harness took the words "You exceeded your current quota" in the message as a daily quota and stopped, instead of waiting and retrying as decision 4 says.

**Don't touch** `bench/r4/real_gemini_lite/` (its 16 finished calls must be kept and resumed from), any other `real*` folder, `grades/`, or any other harness file. Don't call the API or any model. Never print or write the API key.

## Fixes (mechanical, from decision 4)

1. **Classify a 429 by its structured details, not its message text.** Google's error body carries `details` entries:
   - a `QuotaFailure` whose `violations[].quotaId` names the limit, for example `GenerateRequestsPerMinutePerProjectPerModel-FreeTier`. A quota ID containing `PerMinute` is a per-minute limit: wait and retry. A quota ID containing `PerDay` is a daily limit: stop cleanly as now;
   - a `RetryInfo` with `retryDelay`, for example `"51s"`. Wait that long (plus 1 second) before retrying. Fall back to `Retry-After` or the 10 / 20 / 40 s backoff only if neither is present.
   - When the details say neither per-minute nor per-day, keep the existing behaviour: back off, and stop cleanly only if 429s keep coming after the backoff.
   - Per-minute waits don't count toward the 5-try limit for other errors; a call may wait several times. Cap the total wait for one call at 10 minutes, then treat it as a failure (recorded, retried on resume).
2. **Pace requests so the limit isn't hit.** Add a client-side limiter shared by all worker threads: at most 14 requests started in any rolling 60-second window. Keep up to 4 calls in flight and the seed-by-seed order.
3. **Resume.** The earlier run left a `stopped_at` in `run_info.json`. A new run resumes from the 16 recorded calls (skipping the ones that succeeded, retrying the failed one), and records its own start and stop.

## Tests

Add to `test_gemini_lite_offline.py`: a per-minute 429 with `RetryInfo` is waited out and retried, not treated as a stop; a per-day 429 stops cleanly; the limiter never lets more than 14 calls start in 60 seconds (use a fake clock); resume after a `stopped_at` continues from the right call. All four offline test files must pass.

## Final message

What changed, test counts, and a plain-English summary (RULES rule 10). Don't commit or push.
