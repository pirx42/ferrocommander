# Skill: Proactively suggest better approaches to given instructions

**When.** The owner gives an instruction/direction — and there exists
(from your own knowledge or discoverable via research) an established
different approach that could reach the goal better: a different paradigm,
a different architecture, an industry standard for the problem class.

**Rule.**

1. **Always raise it, never silently go along.** The instruction is not
   blocked — the better approach is put NEXT to it as a suggestion
   (briefly: what, why better, what it would cost). The decision stays
   with the owner.
2. **Knowledge check at plan/direction starts:** before larger endeavors,
   explicitly ask "how do others solve this problem class?" (e.g. bots for
   this game type: scripted/utility AI, behavior trees, HTN planners,
   MCTS, BC+RL hybrids) — and name deviations from the chosen path.
3. **Early, not only at the plateau.** The suggestion belongs at the
   BEGINNING of the work or at the point where the evidence tips — not
   only when the owner himself raises the architecture question.

**Why.** Owner instruction 2026-08-24. Trigger: the v20 tuning ladder
(a–k) ran for several iterations against a structural data-to-space
mismatch; the role reversal (RuleBot backbone + arbiter instead of
end-to-end PPO — standard thinking for complex game bots) only came onto
the table through the owner's question "do we need to consider a different
approach?", even though the evidence (memorization, variant trade-offs)
was there earlier.

Related: [01](01-clarify-with-options.md) (offer options),
[41](41-more-correct-variant.md) (more correct variant), [65](65-verify-or-ask-never-assume.md).
