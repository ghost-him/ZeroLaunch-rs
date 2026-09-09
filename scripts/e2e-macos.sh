#!/usr/bin/env bash
set -euo pipefail

if [[ "$(uname -s)" != "Darwin" ]]; then
  echo "macOS E2E 仅支持 Darwin" >&2
  exit 2
fi

for tool in jq curl; do
  command -v "${tool}" >/dev/null || {
    echo "缺少 E2E 依赖工具: ${tool}" >&2
    exit 2
  }
done

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
token_file="${HOME}/Library/Application Support/ZeroLaunch-rs/cli-token.json"
cli_bin="${repo_root}/target/debug/zerolaunch-cli"

if [[ ! -x "${cli_bin}" ]]; then
  rust_toolchain="1.90.0-aarch64-apple-darwin"
  rustup run "${rust_toolchain}" cargo build -p zerolaunch-cli --manifest-path "${repo_root}/Cargo.toml"
fi

if [[ ! -f "${token_file}" ]]; then
  echo "未找到 ${token_file}，请先启动 ZeroLaunch 主程序" >&2
  exit 1
fi

host="$(jq -er '.host // "127.0.0.1"' "${token_file}")"
port="$(jq -er '.port | numbers' "${token_file}")"
token="$(jq -er '.token | strings | select(length > 0)' "${token_file}")"
base_url="http://${host}:${port}"

json_ping="$(${cli_bin} --json ping)"
[[ "$(jq -er '.pong' <<<"${json_ping}")" == "true" ]]

session="$(${cli_bin} --json session)"
jq -e '.mode | strings' <<<"${session}" >/dev/null

query="$(${cli_bin} --json query terminal)"
jq -e '(.list.results | type) == "array"' <<<"${query}" >/dev/null

components="$(${cli_bin} --json config list)"
[[ "$(jq -er 'length > 0' <<<"${components}")" == "true" ]]

plugins="$(${cli_bin} --json plugins list)"
[[ "$(jq -er 'type == "array"' <<<"${plugins}")" == "true" ]]

unauthorized_status="$(curl --noproxy '*' -sS -o /dev/null -w '%{http_code}' "${base_url}/v1/ping")"
[[ "${unauthorized_status}" == "401" ]]

response_file="$(mktemp "${TMPDIR:-/tmp}/zerolaunch-e2e.XXXXXX")"
trap 'rm -f "${response_file}"' EXIT
authorized_status="$(curl --noproxy '*' -sS -o "${response_file}" -w '%{http_code}' \
  -H "Authorization: Bearer ${token}" "${base_url}/v1/ping")"
[[ "${authorized_status}" == "200" ]]
curl --noproxy '*' -sS -o "${response_file}" \
  -H "Authorization: Bearer ${token}" "${base_url}/v1/ping" >/dev/null
[[ "$(jq -er '.pong' "${response_file}")" == "true" ]]

echo "macOS E2E passed: ping, session, query, config, plugins, bearer auth"
