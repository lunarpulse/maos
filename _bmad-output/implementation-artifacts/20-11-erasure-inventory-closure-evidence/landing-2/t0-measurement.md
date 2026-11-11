# 20-11 L2 T0 (Dev2011k, HEAD 251dc692)
Prototype = KS (creation/ks-*.diff) + O4 (landing-1/late-halt-o4-*.diff) on a `git archive HEAD` copy, rustfmt'd, prebuilt xtask.
Base kernel 26854 / 21051 (set 2c223219...), domain 10211, aggregate 202121.
Prototype kernel 26880 (+26) / 21069 (+18), domain 10215 (+4); check-service-boundary passes with the single HaltRegistry hash refresh. = grant (+26/+18, +4); ceilings 26893/21078, STOP 21250 not approached.
Landed (formatted, in tree): identical set hash affee8b0...; clean-overlay aggregate 202370 (audit +86, bin +141, domain +4, kernel +18).
