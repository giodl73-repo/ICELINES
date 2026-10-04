"""Read a successful master browser workflow artifact; never rebuild its bytes."""
import json
import os
import re
import subprocess

def validate_selection(repository, run_id, commit):
    if not re.fullmatch(r"[A-Za-z0-9][A-Za-z0-9-]*/[A-Za-z0-9][A-Za-z0-9_.-]*",repository) or not re.fullmatch(r"[1-9][0-9]*",run_id) or not re.fullmatch(r"[a-f0-9]{40}",commit):
        raise ValueError("invalid rollback selection")


def validate_run(run, repository, commit):
    if (run["conclusion"] != "success" or run["head_sha"] != commit or run["head_branch"] != "master"
        or run["event"] not in ["push","workflow_dispatch"] or run["path"] != ".github/workflows/browser-pages.yml"
        or run["head_repository"]["full_name"] != repository):
        raise ValueError("rollback source must be a successful same-repository master browser build")


if __name__ == "__main__":
    repository = os.environ["GITHUB_REPOSITORY"]
    run_id = os.environ["ROLLBACK_RUN_ID"]
    commit = os.environ["ROLLBACK_COMMIT"]
    validate_selection(repository, run_id, commit)
    run = json.loads(subprocess.check_output(["gh","api",f"repos/{repository}/actions/runs/{run_id}"]))
    validate_run(run,repository,commit)
    subprocess.run(["gh","run","download",run_id,"--repo",repository,"--name",f"icelines-browser-{commit}","--dir","target/rollback-browser"],check=True)
