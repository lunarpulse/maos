import re,sys,os
keep_cov = len(sys.argv)>1 and sys.argv[1]=='nogo'
mods = ['check_epic_6_bridge','check_cross_form_equiv','check_adr_040_accepted','check_literal_reappearance','check_governance_categories','check_coverage_matrix_completeness','check_deprecations_declared','check_bare_review_findings','check_dev_model_used_populated','check_epic_close_green']
variants = {'check_epic_6_bridge':'CheckEpic6Bridge','check_cross_form_equiv':'CheckCrossFormEquiv','check_adr_040_accepted':'CheckAdr040Accepted','check_literal_reappearance':'CheckLiteralReappearance','check_governance_categories':'CheckGovernanceCategories','check_coverage_matrix_completeness':'CheckCoverageMatrixCompleteness','check_deprecations_declared':'CheckDeprecationsDeclared','check_bare_review_findings':'CheckBareReviewFindings','check_dev_model_used_populated':'CheckDevModelUsedPopulated','check_epic_close_green':'CheckEpicCloseGreen'}
if keep_cov: mods.remove('check_coverage_matrix_completeness')
# move dev-model helpers
src=open('xtask/src/check_dev_model_used_populated.rs').read().split('\n')
def span(a,b): return '\n'.join(src[a-1:b])
helpers = span(17,108)+'\n\n'+span(301,367)
rc='xtask/src/check_dev_record_completeness.rs'
s=open(rc).read()
cut = s.index('#[cfg(test)]')
s = s[:cut] + '// Moved from the retired check_dev_model_used_populated (20-3b).\n' + helpers + '\n\n' + s[cut:]
s = s.replace('crate::check_dev_model_used_populated::','')
open(rc,'w').write(s)
t='xtask/src/check_dev_model_tier.rs'
s=open(t).read(); s=s.replace('use crate::check_dev_model_used_populated::{','use crate::check_dev_record_completeness::{'); open(t,'w').write(s)
m=open('xtask/src/main.rs').read().split('\n')
out=[]; i=0
vset={variants[x] for x in mods}
while i < len(m):
    line=m[i]
    mm=re.match(r'^mod (\w+);$',line)
    if mm and mm.group(1) in mods: i+=1; continue
    # command block: variant line "    Name {"
    vm=re.match(r'^    (\w+) \{$',line)
    if vm and vm.group(1) in vset:
        # drop preceding doc/attr lines already in out
        while out and (out[-1].startswith('    ///') or out[-1].startswith('    #[command')): out.pop()
        j=i
        while m[j] != '    },': j+=1
        i=j+1; continue
    am=re.match(r'^        Commands::(\w+) \{',line)
    if am and am.group(1) in vset:
        if line.rstrip().endswith(','):
            i+=1; continue
        j=i
        while m[j] != '        }': j+=1
        i=j+1; continue
    out.append(line); i+=1
open('xtask/src/main.rs','w').write('\n'.join(out))
for x in mods: os.remove(f'xtask/src/{x}.rs')
p='xtask/src/check_ship_gate_completeness.rs'; s=open(p).read(); s=s.replace('    "check-cross-form-equiv",\n','',1); open(p,'w').write(s)
p='xtask/src/prepush.rs'; s=open(p).read()
s=s.replace('    ("check-deprecations-declared", "deprecation declarations"),\n','',1)
if not keep_cov:
    s=s.replace('    (\n        "check-coverage-matrix-completeness",\n        "xtask/coverage-matrix.toml",\n    ),\n','',1)
open(p,'w').write(s)
p='crates/maos-bin/src/main.rs'; s=open(p).read()
s=s.replace('        "check-bare-review-findings",\n        "check-dev-model-used-populated",\n    ];','    ];',1)
s=s.replace(',"check-bare-review-findings","check-dev-model-used-populated"]}}"#',']}}"#',1)
open(p,'w').write(s)
