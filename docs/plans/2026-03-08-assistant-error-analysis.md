# Assistant Error Analysis & Interaction Improvement

> **Origin prompt**: `/brainstorming 请用log-query skill 查看一下所有的log，通过user和assistant，尤其是user纠正ASSISTANT的内容，总结一下assistant犯错的本质。另外在总结一下user和assistant的交互方式是否还有改进的空间`

**Date**: 2026-03-08
**Data sources**: Session logs from cc-companion (247 sessions), MRI-agent-benchmark (33 sessions), blade-agent (3 sessions), sam (146 sessions), and other projects
**Method**: Full-text search across all JSONL logs for user correction patterns including: tool rejections, explicit corrections (不对/错了/不是), scope limiters (只需要/不用/先不要), and user clarification signals (我的意思是/我改变主意)

---

## Part 1: The Nature of Assistant Mistakes

### 1. Scope Creep (过度行动) — Most Frequent

The single most common correction pattern. Assistant expands beyond what was asked.

**Evidence:**
- User says "不需要改code，只需要创个 issue" → assistant still attempts code changes
- User says "只需要一个 markdown 文件在 ./reference folder" → assistant tries to modify the main document (`benchmark_redesign_v2.md`)
- User says "比较两个文件的得分，不用修改或者添加任何code" → assistant considers adding code
- User says "不需要改code，只需要描述明白问题和测试建议，然后用 gh cli 创立一个 issue" → assistant needs explicit reminder

**Root cause**: Assistant treats "understanding a requirement" and "executing an action" as the same step. Upon receiving a requirement, it immediately begins implementation rather than confirming scope first.

### 2. Unverified Assumptions (未验证假设) — Causes Real Bugs

Assistant trusts documentation/inference over verified runtime behavior.

**Evidence:**
- Keychain service name: used `"claude-ai-oauth"` (from skill docs), actual value is `"Claude Code-credentials"` (from real code)
- Keychain data format: assumed bare token string, actual format is JSON requiring `json.loads()` to extract `claudeAiOauth.accessToken`
- API rate limit model: assumed linear relationship, actual behavior is sliding window / token bucket (discovered only after running experiment)

**Root cause**: Trusting "looks reasonable" information without verifying against actual code or runtime. The Keychain bug is the clearest example — the assistant copied from skill documentation without checking `src/app/mod.rs` where the real service name was defined.

### 3. Over-Engineering (过度工程化) — Persistent Despite Warnings

CLAUDE.md contains 4 notes, 3 of which warn against this. It keeps happening.

**Evidence:**
- blade-agent analysis: ~70% of code was premature abstraction and dead code. Core value needed only ~30% of total code
- User explicitly asked "这个项目是否太复杂了？结构是否过于冗余" — confirming this is a recurring concern
- Assistant proposes 3 approaches with trade-off analysis when user asked a simple question
- Unnecessary error handling, configuration options, abstraction layers added without request

**Root cause**: AI training rewards thoroughness and comprehensiveness. "Simple" feels like "incomplete" to the model.

### 4. Unnecessary Questions (多余提问) — Wastes User Time

Assistant asks questions whose answers are obvious from context or irrelevant to the task.

**Evidence:**
- Assistant asked "Scope — Run all 3 phases in one go, or make them separate subcommands?"
- User responded: "我不明白问题" and "这有什么关系吗？"
- Assistant later admitted: "我问得不好，忽略它"
- Another session: user had already given clear instructions but assistant asked for confirmation on obvious details

**Root cause**: The model defaults to "explore all dimensions" rather than "execute what's clearly stated." Sometimes a question is the assistant's way of appearing thoughtful when it should just act.

### 5. Momentum Bias (惯性执行) — Doesn't Stop on Reject

Tool rejections don't immediately halt the action chain.

**Evidence:**
- Session `3dc3b4a2`: rejected **6 times** in a single session
- Session `4137c88a`: rejected **4 times** consecutively
- Session `d3ddae23`: rejected 3 times, user had to say "我改变主意了" explicitly
- After rejection, assistant sometimes attempts a "workaround" of the rejected action instead of asking why it was rejected

**Root cause**: The model treats rejection as "this specific edit was wrong" rather than "stop and reconsider your entire approach."

---

## Part 2: Interaction Improvement Opportunities

### 1. Enforce "Confirm Before Act" for Limiting Words

When user uses scope limiters (只需要, 不用, 先不要, 不需要改code), assistant should:
- Restate understood scope before acting
- NOT "helpfully" expand scope
- Treat these words as hard constraints, not suggestions

**Current pattern**: `User gives limited request → Assistant executes expanded version → User rejects → Assistant adjusts`
**Target pattern**: `User gives limited request → Assistant confirms scope → Executes within scope → Done`

### 2. Separate Research/Plan/Implement Phases

User has a clear workflow hierarchy that assistant frequently violates:
- **Issue creation phase**: describe problems and proposals only, no code
- **Planning phase**: design but don't implement
- **Implementation phase**: write code following approved plan

Assistant often jumps from issue-creation to implementation, skipping the user's approval gates.

### 3. Better Chinese Language Comprehension for Intent Limiters

Key Chinese patterns that signal constraints:
- "只需要" (zhǐ xūyào) = ONLY this, nothing else
- "先不要" (xiān bùyào) = NOT NOW, maybe later
- "不用改code" (bùyòng gǎi code) = DO NOT touch code
- "帮我看看" (bāng wǒ kànkan) = READ and EXPLAIN, don't modify
- "不用修改或者添加任何code" = zero code changes, period

Recommendation: When user includes such limiters, assistant should echo the constraint back before proceeding.

### 4. First Reject = Full Stop Protocol

Current problem: 1 reject is not enough; often takes 2-5 rejects to stop.

Recommendation: On first reject, assistant should:
1. Immediately stop the current action chain
2. Acknowledge the rejection
3. Wait for user to provide new direction
4. NOT attempt alternative approaches without asking

### 5. Long Session Checkpoint Summaries

In long sessions (e.g., session `1a3ff8e1` running overnight API experiments), assistant sometimes:
- Repeats already-completed operations
- Forgets earlier agreements (e.g., experiment parameters)
- Requires user to re-provide already-given information

Recommendation: Periodically produce a short checkpoint summary of decisions made and current state.

---

## Summary

| Mistake Type | Frequency | Severity | Root Cause |
|---|---|---|---|
| Scope Creep | Very High | Medium | Treating understanding as execution |
| Unverified Assumptions | Medium | High (causes bugs) | Trust docs > verify code |
| Over-Engineering | High | Medium | Training bias toward thoroughness |
| Unnecessary Questions | Medium | Low | Exploring all dimensions vs acting |
| Momentum Bias | Medium | High (wastes time) | Rejection = "fix edit" not "stop" |

**Core insight**: The fundamental problem is not capability but **behavioral control** — the assistant is too "proactively helpful." The CLAUDE.md constraints ("SIMPLEST", "don't over-engineer", "DO NOT ADD UNNECESSARY FEATURES") are all fighting the same tendency. The metric to optimize is **"number of rejections per session"**, not "features completed."
