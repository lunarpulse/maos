import os
import pathlib
import subprocess

root = pathlib.Path('/home/lunarpulse/dev_ws')
extras = ['poll', 'ppoll', 'rt_sigaction', 'clone3', 'memfd_create', 'sendto', 'recvfrom']
for drop in [None, *extras]:
    selected = [value for value in extras if value != drop]
    env = os.environ.copy()
    env.update(T2_SECCOMP='candidate', T2_SECCOMP_ORDER='kill-first', T2_RUNNER_EXTRAS=','.join(selected))
    cmd = ['python3', 'spikes/story-17-3a-wasm-recall/t2-ladder/relay_fd.py']
    print('+ ' + ' '.join(f'{key}={env[key]}' for key in ['T2_SECCOMP','T2_SECCOMP_ORDER','T2_RUNNER_EXTRAS']) + ' ' + ' '.join(cmd), flush=True)
    result = subprocess.run(cmd, cwd=root, env=env, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
    output = result.stdout.decode('utf-8', 'replace')
    print(output, end='')
    print(f'LOO target=spike-runner-inherited-fd drop={drop or "none"} harness_rc={result.returncode} recall_ok={"recall\\\":\\\"ok" in output} service_clean={"LADDER_RC=0 SERVICE_RC=0" in output}', flush=True)
