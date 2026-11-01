#!/usr/bin/env python3
import hashlib
import json
import os
import subprocess
import sys
import tarfile
import urllib.error
import urllib.parse
import urllib.request
import zipfile


def sha256_file(filepath: str) -> str:
    h = hashlib.sha256()
    with open(filepath, "rb") as f:
        while chunk := f.read(65536):
            h.update(chunk)
    return h.hexdigest()


def find_file(search_dir: str, target_name: str) -> str | None:
    if not os.path.exists(search_dir):
        return None
    for root, _, files in os.walk(search_dir):
        if target_name in files:
            return os.path.join(root, target_name)
    return None


def main():
    token = os.environ.get("GITHUB_TOKEN")
    if not token:
        print("ERROR: GITHUB_TOKEN environment variable is not set.", file=sys.stderr)
        sys.exit(1)

    repository = os.environ.get("GITHUB_REPOSITORY", "moha/strata")
    tag_name = os.environ.get("GITHUB_REF_NAME")

    ref = os.environ.get("GITHUB_REF", "")
    if not tag_name or not tag_name.startswith("v"):
        if ref.startswith("refs/tags/"):
            tag_name = ref.replace("refs/tags/", "")

    if not tag_name:
        print("ERROR: Could not determine tag name from GITHUB_REF_NAME or GITHUB_REF.", file=sys.stderr)
        sys.exit(1)

    clean_tag = tag_name.replace("refs/tags/", "")

    api_url = os.environ.get("GITHUB_API_URL")
    if not api_url:
        server_url = os.environ.get("GITHUB_SERVER_URL", "https://git.soventa.tech")
        api_url = f"{server_url.rstrip('/')}/api/v1"
    else:
        api_url = api_url.rstrip("/")

    print(f"Repository: {repository}")
    print(f"Tag: {clean_tag}")
    print(f"API URL: {api_url}")

    assets_dir = os.path.abspath("dist-release")
    os.makedirs(assets_dir, exist_ok=True)

    # 1. Package Linux binary & archive
    linux_bin = find_file("artifacts/linux", "strata-linux")
    if linux_bin:
        os.chmod(linux_bin, 0o755)
        raw_dest = os.path.join(assets_dir, "strata-linux")
        with open(linux_bin, "rb") as src, open(raw_dest, "wb") as dst:
            dst.write(src.read())
        os.chmod(raw_dest, 0o755)

        tar_dest = os.path.join(assets_dir, "strata-linux-x86_64.tar.gz")
        with tarfile.open(tar_dest, "w:gz") as tar:
            tar.add(linux_bin, arcname="strata-linux")
        print(f"Packaged Linux: {raw_dest}, {tar_dest}")
    else:
        print("WARNING: strata-linux not found under artifacts/linux", file=sys.stderr)

    # 2. Package Windows installer & portable zip
    win_msi = find_file("artifacts/windows", "strata.msi")
    if win_msi:
        msi_dest = os.path.join(assets_dir, "strata-windows-x86_64.msi")
        with open(win_msi, "rb") as src, open(msi_dest, "wb") as dst:
            dst.write(src.read())
        print(f"Copied Windows MSI installer: {msi_dest}")
    else:
        print("WARNING: strata.msi not found under artifacts/windows", file=sys.stderr)

    win_exe = find_file("artifacts/windows", "strata.exe")
    if win_exe:
        zip_dest = os.path.join(assets_dir, "strata-windows-x86_64.zip")
        with zipfile.ZipFile(zip_dest, "w", zipfile.ZIP_DEFLATED) as zf:
            zf.write(win_exe, "strata.exe")
            for dll_name in ["libstdc++-6.dll", "libgcc_s_seh-1.dll", "libwinpthread-1.dll"]:
                dll_path = find_file("artifacts/windows", dll_name)
                if dll_path:
                    zf.write(dll_path, dll_name)
                else:
                    print(f"WARNING: DLL {dll_name} not found under artifacts/windows", file=sys.stderr)
        print(f"Packaged Windows portable zip: {zip_dest}")
    else:
        print("WARNING: strata.exe not found under artifacts/windows", file=sys.stderr)

    # 3. Generate SHA256SUMS
    asset_files = [f for f in sorted(os.listdir(assets_dir)) if os.path.isfile(os.path.join(assets_dir, f)) and f != "SHA256SUMS"]
    if not asset_files:
        print("ERROR: No assets found in dist-release to publish!", file=sys.stderr)
        sys.exit(1)

    sums_file = os.path.join(assets_dir, "SHA256SUMS")
    with open(sums_file, "w") as f:
        for fname in asset_files:
            fpath = os.path.join(assets_dir, fname)
            f.write(f"{sha256_file(fpath)}  {fname}\n")
    asset_files.append("SHA256SUMS")
    print(f"Generated SHA256SUMS for {len(asset_files) - 1} assets.")

    # 4. Forgejo Release lookup or creation
    headers = {
        "Authorization": f"token {token}",
        "Content-Type": "application/json",
        "Accept": "application/json",
    }

    rel_tag_url = f"{api_url}/repos/{repository}/releases/tags/{clean_tag}"
    rel_id = None
    try:
        req = urllib.request.Request(rel_tag_url, headers=headers)
        with urllib.request.urlopen(req) as resp:
            data = json.loads(resp.read().decode())
            rel_id = data["id"]
            print(f"Found existing release ID {rel_id} for tag {clean_tag}")
    except urllib.error.HTTPError as e:
        if e.code == 404:
            print(f"Release for {clean_tag} does not exist yet. Creating...")
        else:
            print(f"HTTPError checking release for {clean_tag}: {e.code} {e.read().decode()}", file=sys.stderr)
            sys.exit(1)

    if not rel_id:
        create_url = f"{api_url}/repos/{repository}/releases"
        body = json.dumps({
            "tag_name": clean_tag,
            "name": f"Strata {clean_tag}",
            "body": f"Release {clean_tag}",
            "draft": False,
            "prerelease": False,
        }).encode()
        req = urllib.request.Request(create_url, data=body, headers=headers, method="POST")
        try:
            with urllib.request.urlopen(req) as resp:
                data = json.loads(resp.read().decode())
                rel_id = data["id"]
                print(f"Created release ID {rel_id} for tag {clean_tag}")
        except urllib.error.HTTPError as e:
            if e.code in (409, 422):
                req = urllib.request.Request(rel_tag_url, headers=headers)
                with urllib.request.urlopen(req) as resp:
                    data = json.loads(resp.read().decode())
                    rel_id = data["id"]
                    print(f"Retrieved release ID {rel_id} after conflict.")
            else:
                print(f"Failed to create release: {e.code} {e.read().decode()}", file=sys.stderr)
                sys.exit(1)

    # 5. Retrieve existing assets for release
    assets_url = f"{api_url}/repos/{repository}/releases/{rel_id}/assets"
    existing_assets = {}
    try:
        req = urllib.request.Request(assets_url, headers=headers)
        with urllib.request.urlopen(req) as resp:
            for item in json.loads(resp.read().decode()):
                existing_assets[item["name"]] = item["id"]
    except urllib.error.HTTPError as e:
        print(f"Warning: Could not fetch existing assets: {e.code}", file=sys.stderr)

    # 6. Upload assets
    for fname in asset_files:
        fpath = os.path.join(assets_dir, fname)
        if fname in existing_assets:
            del_id = existing_assets[fname]
            del_url = f"{assets_url}/{del_id}"
            print(f"Deleting existing asset {fname} (ID: {del_id})...")
            try:
                del_req = urllib.request.Request(del_url, headers=headers, method="DELETE")
                with urllib.request.urlopen(del_req) as resp:
                    pass
            except urllib.error.HTTPError as e:
                print(f"Warning: failed to delete old asset {fname}: {e.code}", file=sys.stderr)

        print(f"Uploading {fname} ({os.path.getsize(fpath)} bytes)...")
        upload_endpoint = f"{assets_url}?name={urllib.parse.quote(fname)}"
        curl_cmd = [
            "curl", "-s", "-S", "-f",
            "-H", f"Authorization: token {token}",
            "-F", f"attachment=@{fpath};filename={fname}",
            upload_endpoint
        ]
        result = subprocess.run(curl_cmd, capture_output=True, text=True)
        if result.returncode != 0:
            print(f"ERROR uploading {fname}: {result.stderr}", file=sys.stderr)
            sys.exit(result.returncode)
        print(f"Successfully uploaded {fname}")

    print("All release assets uploaded successfully!")


if __name__ == "__main__":
    main()
