"""Synthetic repositories for the latency bench: `small` (3 files) and
`large` (N files of about 2 KB each), both with src/client.rs to edit."""
import os
import subprocess
import sys

work, files = sys.argv[1], int(sys.argv[2])
CLIENT = "fn connect(url: &str) -> Result<Conn> {\n    let conn = open(url)?;\n    Ok(conn)\n}\n"
env = dict(os.environ, GIT_AUTHOR_NAME="bench", GIT_AUTHOR_EMAIL="bench@example.invalid",
           GIT_COMMITTER_NAME="bench", GIT_COMMITTER_EMAIL="bench@example.invalid")


def repo(name: str, count: int) -> None:
    root = os.path.join(work, name)
    os.makedirs(os.path.join(root, "src"))
    with open(os.path.join(root, "src", "client.rs"), "w") as f:
        f.write(CLIENT)
    for i in range(count):
        d = os.path.join(root, f"pkg{i // 1000:04d}")
        os.makedirs(d, exist_ok=True)
        with open(os.path.join(d, f"f{i}.rs"), "w") as f:
            f.write(f"// file {i}\nfn f{i}() {{}}\n" + "x" * 2000 + "\n")
    for args in (["init", "-q"], ["add", "-A"], ["commit", "-qm", "bench"]):
        subprocess.run(["git", *args], cwd=root, env=env, check=True)


repo("small", 2)
repo("large", files)
with open(os.path.join(work, "edit.spl"), "w") as f:
    f.write('edit "src/client.rs" {\n  @c := one text "open(url)"\n'
            '  replace @c with ["with_retry(3, || ", keep(@c), ")"]\n}\n')
