#!/usr/bin/env python3
"""Select a release push by exact tag+SHA and reject stale run attempts."""
import json
import sys


def select(runs, sha, tag):
    matching = [run for run in runs if run.get('event') == 'push'
                and run.get('head_sha') == sha and run.get('head_branch') == tag]
    if not matching:
        return None
    run = max(matching, key=lambda item: item['id'])
    return run['id'], run['run_attempt']


def successful(run, run_id, attempt, sha, tag):
    if (run.get('id'), run.get('run_attempt'), run.get('head_sha'), run.get('head_branch'), run.get('event')) != (run_id, attempt, sha, tag, 'push'):
        raise ValueError('release run identity/attempt changed; refusing stale success')
    if run.get('status') != 'completed':
        return False
    if run.get('conclusion') != 'success':
        raise ValueError('bound release attempt failed: ' + str(run.get('conclusion')))
    return True


if __name__ == '__main__':
    mode, sha, tag, *binding = sys.argv[1:]
    data = json.load(sys.stdin)
    if mode == 'select':
        selected = select(data['workflow_runs'], sha, tag)
        if selected is None:
            sys.exit(3)
        print(*selected)
    elif mode == 'check':
        run_id, attempt = map(int, binding)
        sys.exit(0 if successful(data, run_id, attempt, sha, tag) else 3)
    else:
        raise SystemExit('unknown release-run mode')
