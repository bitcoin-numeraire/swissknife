# Preparing and publishing a release

`CHANGELOG.md` is the source of the GitHub release description. Keep release
entries focused on user-visible behavior, breaking changes, and security fixes.
Documentation, test, and internal refactor changes do not need individual changelog entries.

## Prepare the release PR

1. Review changes since the previous tag. Consolidate duplicate entries and
   describe the resulting behavior, rather than listing every commit.
2. Move the notes from `Unreleased` into a dated version section, keeping an
   empty `Unreleased` heading for subsequent work. Use absolute links inside
   the release section so the text also works outside this file.
3. Set the same version in the root `Cargo.toml`, both workspace crate manifests,
   and `dashboard/package.json`. Refresh `Cargo.lock`, then run `make openapi`
   so the checked-in API document reports that version.
4. Check the README and documentation against the current configuration and API.
   Run the relevant local checks and require green CI on the release commit.

## Extract the release description

From the repository root, this writes the current package version's changelog
section to a temporary file. It does not publish anything:

```bash
python3 - <<'PY'
import re
import tomllib
from pathlib import Path

version = tomllib.loads(Path("Cargo.toml").read_text())["package"]["version"]
changelog = Path("CHANGELOG.md").read_text()
pattern = rf"^## \[{re.escape(version)}\] - \d{{4}}-\d{{2}}-\d{{2}}\n(.*?)(?=^## |\Z)"
section = re.search(pattern, changelog, re.MULTILINE | re.DOTALL)
if section is None:
    raise SystemExit(f"No dated changelog entry for {version}")
Path("/tmp/swissknife-release-notes.md").write_text(section.group(1).strip() + "\n")
PY
```

Review that file and confirm the release date before tagging.

## Tag and publish

After merging the release PR, fetch `main` and choose its tested release commit.
The following example is for `0.3.0`:

```bash
git fetch origin main --tags
git tag -a v0.3.0 origin/main -m "SwissKnife 0.3.0"
git push origin v0.3.0
```

Pushing a `v*` tag triggers `.github/workflows/release.yml`. It builds the bundled
backend/dashboard, backend-only, and standalone-dashboard images on native
AMD64 and ARM64 runners, then publishes multi-architecture manifests and
provenance attestations to Docker Hub. The workflow requires the repository's
`DOCKERHUB_USERNAME` and `DOCKERHUB_TOKEN` secrets.

Wait for the Release workflow to succeed and inspect all three manifests:

```bash
docker buildx imagetools inspect bitcoinnumeraire/swissknife:0.3.0
docker buildx imagetools inspect bitcoinnumeraire/swissknife-server:0.3.0
docker buildx imagetools inspect bitcoinnumeraire/swissknife-dashboard:0.3.0
```

Confirm that each manifest includes `linux/amd64` and `linux/arm64` and matches
the intended source tag. Then publish the GitHub release with the extracted
changelog section:

```bash
gh release create v0.3.0 --verify-tag --title "0.3.0" \
  --notes-file /tmp/swissknife-release-notes.md
```

The tag workflow publishes images; it does not create the GitHub release or
deploy production. Update k8s-infra after the images are available.
