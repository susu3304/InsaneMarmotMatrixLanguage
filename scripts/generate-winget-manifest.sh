#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'USAGE'
Usage:
  scripts/generate-winget-manifest.sh --sha256 SHA256 [options]

Required:
  --sha256 SHA256           SHA256 of the Windows x64 zip installer.

Options:
  --version VERSION         Package version. Defaults to Cargo.toml package version.
  --release-tag TAG         GitHub release tag. Defaults to v<VERSION>.
  --release-date DATE       Release date in YYYY-MM-DD. Defaults to today in UTC.
  --output-dir DIR          Output root. Defaults to dist/winget.
  --installer-url URL       Installer URL. Defaults to the GitHub release asset URL.
  --package-id ID           Defaults to susu3304.InsaneMarmotMatrixLanguage.
  --owner OWNER             GitHub owner. Defaults to susu3304.
  --repo REPO               GitHub repo. Defaults to InsaneMarmotMatrixLanguage.
  --publisher NAME          winget publisher. Defaults to OWNER.
  --license NAME            Defaults to Proprietary.
  -h, --help                Show this help.
USAGE
}

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

read_cargo_version() {
  sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -n 1
}

version=""
sha256=""
release_tag=""
release_date="$(date -u +%F)"
output_dir="dist/winget"
installer_url=""
package_id="susu3304.InsaneMarmotMatrixLanguage"
owner="susu3304"
repo="InsaneMarmotMatrixLanguage"
publisher=""
license="Proprietary"
asset_name="imm-windows-x64.zip"

while [ "$#" -gt 0 ]; do
  case "$1" in
    --version)
      version="${2:?missing value for --version}"
      shift 2
      ;;
    --sha256)
      sha256="${2:?missing value for --sha256}"
      shift 2
      ;;
    --release-tag)
      release_tag="${2:?missing value for --release-tag}"
      shift 2
      ;;
    --release-date)
      release_date="${2:?missing value for --release-date}"
      shift 2
      ;;
    --output-dir)
      output_dir="${2:?missing value for --output-dir}"
      shift 2
      ;;
    --installer-url)
      installer_url="${2:?missing value for --installer-url}"
      shift 2
      ;;
    --package-id)
      package_id="${2:?missing value for --package-id}"
      shift 2
      ;;
    --owner)
      owner="${2:?missing value for --owner}"
      shift 2
      ;;
    --repo)
      repo="${2:?missing value for --repo}"
      shift 2
      ;;
    --publisher)
      publisher="${2:?missing value for --publisher}"
      shift 2
      ;;
    --license)
      license="${2:?missing value for --license}"
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "unknown option: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
done

if [ -z "$version" ]; then
  version="$(read_cargo_version)"
fi

if [ -z "$version" ]; then
  echo "version is required; pass --version or set Cargo.toml package.version" >&2
  exit 2
fi

if [ -z "$sha256" ]; then
  echo "--sha256 is required" >&2
  exit 2
fi

sha256="$(printf '%s' "$sha256" | tr '[:lower:]' '[:upper:]')"
case "$sha256" in
  *[!0-9A-F]*)
    echo "--sha256 must be hexadecimal" >&2
    exit 2
    ;;
esac

if [ "${#sha256}" -ne 64 ]; then
  echo "--sha256 must be 64 hex characters" >&2
  exit 2
fi

case "$release_date" in
  ????-??-??) ;;
  *)
    echo "--release-date must use YYYY-MM-DD" >&2
    exit 2
    ;;
esac

if [ -z "$release_tag" ]; then
  release_tag="v$version"
fi

if [ -z "$publisher" ]; then
  publisher="$owner"
fi

if [ -z "$installer_url" ]; then
  installer_url="https://github.com/$owner/$repo/releases/download/$release_tag/$asset_name"
fi

manifest_version="1.12.0"
package_path="$(printf '%s' "$package_id" | tr '.' '/')"
first_letter="$(printf '%.1s' "$package_id" | tr '[:upper:]' '[:lower:]')"
manifest_dir="$output_dir/manifests/$first_letter/$package_path/$version"
mkdir -p "$manifest_dir"

cat > "$manifest_dir/$package_id.yaml" <<EOF
# yaml-language-server: \$schema=https://aka.ms/winget-manifest.version.$manifest_version.schema.json
PackageIdentifier: $package_id
PackageVersion: $version
DefaultLocale: en-US
ManifestType: version
ManifestVersion: $manifest_version
EOF

cat > "$manifest_dir/$package_id.locale.en-US.yaml" <<EOF
# yaml-language-server: \$schema=https://aka.ms/winget-manifest.defaultLocale.$manifest_version.schema.json
PackageIdentifier: $package_id
PackageVersion: $version
PackageLocale: en-US
Publisher: $publisher
PublisherUrl: https://github.com/$owner
PackageName: insane marmot matrix
PackageUrl: https://github.com/$owner/$repo
License: $license
ShortDescription: Native CLI runtime for the IMM experimental programming language.
Tags:
- cli
- interpreter
- programming-language
- rust
ManifestType: defaultLocale
ManifestVersion: $manifest_version
EOF

cat > "$manifest_dir/$package_id.installer.yaml" <<EOF
# yaml-language-server: \$schema=https://aka.ms/winget-manifest.installer.$manifest_version.schema.json
PackageIdentifier: $package_id
PackageVersion: $version
InstallerType: zip
NestedInstallerType: portable
Commands:
- imm
ReleaseDate: $release_date
Installers:
- Architecture: x64
  NestedInstallerFiles:
  - RelativeFilePath: imm.exe
    PortableCommandAlias: imm
  InstallerUrl: $installer_url
  InstallerSha256: $sha256
ManifestType: installer
ManifestVersion: $manifest_version
EOF

echo "Wrote winget manifests to $manifest_dir"
echo "Validate on Windows with: winget validate \"$manifest_dir\""
