#!/usr/bin/env bash
# rubber-stamp.sh — intentionally dishonest demo adapter.
# Claims every organ passes AND reports its control check as passing.
# Verdict must compute to NONCONFORMANT — a control reporting "pass" is
# proof the harness cannot fail checks, per SPEC.md §1.
emit() { printf '{"check":"%s","organ":"%s","status":"pass","evidence":{"claimed":true}}\n' "$1" "$2"; }
emit awake.alive awake
emit identity.declare identity
emit perception.screen perception
emit memory.store memory
emit deliberation.record deliberation
emit action.tool action
emit vigilance.watcher vigilance
emit learning.improve learning
emit audit.chain audit
emit sovereignty.no_egress sovereignty
emit generality.breadth generality
emit planning.decompose planning
emit reflection.error_correct reflection
# the tell: a lying harness reports even its control as pass
printf '{"check":"audit.control_negative","organ":"audit","status":"pass","control":true,"evidence":{"claimed":true}}\n'
