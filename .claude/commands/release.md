# Release

Bump version, update changelog, commit, tag, push, and build.

1. Check current version in package.json
2. Ask for bump type (patch/minor/major)
3. Update version in: package.json, src-tauri/Cargo.toml, src-tauri/tauri.conf.json
4. Move CHANGELOG [Unreleased] to new version section
5. Commit: `git commit -m "chore: release vX.Y.Z"`
6. Tag: `git tag vX.Y.Z`
7. Push: `git push && git push --tags`
8. Build: `npm run tauri build`
