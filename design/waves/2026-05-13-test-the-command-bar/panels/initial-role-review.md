# Initial role review

## GLASS

The protocol must test whether the user can predict focus and screen changes in
under a few seconds. If `Tab`, `:`, `/`, or `Esc` need explanation every time,
the UI has a discoverability gap even if the parser works.

## CREST

The command bar should feel like a deliberate cockpit, not a hidden REPL. Review
the visible affordance, feedback flash, and help overlay before adding more
grammar.

## BENCH

Filtered cargo commands can pass with zero matches. Each pulse must record the
actual matching tests or add a focused test before checking a gate.

## EDGE

Treat tester input as adversarial grammar. Preserve common aliases when they
map cleanly, but reject ambiguous commands with specific correction text.

## WIRE

Write commands must remain POST-backed or mutation-intent backed. Do not make
watch/favorite/admin commands look like GET navigation just to improve flow.
