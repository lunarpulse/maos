import pathlib
import subprocess
import sys

root = pathlib.Path('/home/lunarpulse/dev_ws')
ladder = root / 'target/debug/t2-ladder'
extras = ['poll', 'ppoll', 'rt_sigaction', 'clone3', 'memfd_create', 'sendto', 'recvfrom']
kind = sys.argv[1]
base = [str(ladder), '--landlock', 'rights', '--seccomp', 'candidate', '--seccomp-order', 'kill-first']
if kind == 'hello-dynamic':
    target = [str(root / 'target/t2-ladder/hello-dynamic')]
    fixed = ['--execute-loader']
elif kind == 'hello-static':
    target = [str(root / 'target/t2-ladder/hello-static')]
    fixed = ['--static-target']
elif kind == 'head-runner':
    component = str(root / 'tests/fixtures/wasm/echo_spirit_component.wasm')
    target = [str(root / 'spikes/story-17-3a-wasm-recall/t2-ladder/target/maos-wasm-runner-head-92911f59'), '--component', component]
    fixed = ['--execute-loader', '--read', component, '--stdin', str(root / 'target/t2-ladder/runner.frame')]
else:
    raise SystemExit('unknown target')

def run(label, chosen):
    cmd = [*base, *fixed, '--extra', ','.join(chosen), '--', *target]
    print('+ ' + ' '.join(cmd), flush=True)
    completed = subprocess.run(cmd, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
    output = completed.stdout.decode('utf-8', 'replace')
    print(output, end='')
    result = next((line for line in reversed(output.splitlines()) if line.startswith('RESULT ')), 'RESULT missing')
    print(f'LOO target={kind} drop={label} harness_rc={completed.returncode} {result}', flush=True)

run('none', extras)
for drop in extras:
    run(drop, [entry for entry in extras if entry != drop])
