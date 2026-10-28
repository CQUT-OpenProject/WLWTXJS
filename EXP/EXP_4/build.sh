#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "$script_dir/../.." && pwd)"
sdk_dir="$repo_root/sdk"
build_dir="$script_dir/build"

if [[ -n "${SDCC:-}" ]]; then
    sdcc_exe="$SDCC"
elif command -v sdcc >/dev/null 2>&1; then
    sdcc_exe="$(command -v sdcc)"
elif [[ -x /opt/homebrew/bin/sdcc ]]; then
    sdcc_exe=/opt/homebrew/bin/sdcc
elif [[ -x /usr/local/bin/sdcc ]]; then
    sdcc_exe=/usr/local/bin/sdcc
else
    echo "error: SDCC not found. Install SDCC 4.x or set SDCC=/path/to/sdcc." >&2
    exit 1
fi

includes=(
    "-I$sdk_dir/cc2530/include"
    "-I$sdk_dir/cc2530/hal"
    "-I$sdk_dir/basicrf"
)

compile_flags=(
    -mmcs51
    --model-large
    --std-sdcc11
    --code-size 0x10000
    --xram-size 0x2000
    --opt-code-size
)

# halMcuWaitUs() relies on a fixed number of NOPs, so skip size optimization.
compile_flags_no_opt=(
    -mmcs51
    --model-large
    --std-sdcc11
    --code-size 0x10000
    --xram-size 0x2000
)

link_flags=(
    -mmcs51
    --model-large
    --std-sdcc11
    --code-size 0x10000
    --xram-size 0x2000
    --out-fmt-ihx
)

sources=(
    "main:$script_dir/main.c"
    "basic_rf:$sdk_dir/basicrf/basic_rf.c"
    "hal_rf:$sdk_dir/cc2530/hal/hal_rf.c"
    "hal_mcu:$sdk_dir/cc2530/hal/hal_mcu.c"
    "hal_int:$sdk_dir/cc2530/hal/hal_int.c"
    "hal_assert:$sdk_dir/cc2530/hal/hal_assert.c"
    "clock:$sdk_dir/cc2530/hal/clock.c"
)

roles=(
    "coordinator:ROLE_COORDINATOR"
    "enddevice:ROLE_ENDDEVICE"
)

for role_spec in "${roles[@]}"; do
    role="${role_spec%%:*}"
    define="${role_spec#*:}"
    role_dir="$build_dir/$role"
    mkdir -p "$role_dir"

    objects=()
    for source_spec in "${sources[@]}"; do
        name="${source_spec%%:*}"
        source_file="${source_spec#*:}"
        flags=("${compile_flags[@]}")
        if [[ "$name" == "hal_mcu" ]]; then
            flags=("${compile_flags_no_opt[@]}")
        fi

        echo "CC  $role/$name.rel"
        "$sdcc_exe" "${flags[@]}" "${includes[@]}" "-D$define" \
            -c "$source_file" -o "$role_dir/$name.rel"
        objects+=("$role_dir/$name.rel")
    done

    hex="$role_dir/$role.hex"
    echo "LD  $hex"
    "$sdcc_exe" "${link_flags[@]}" "${includes[@]}" "${objects[@]}" -o "$hex"
done

echo "Build complete:"
echo "  $build_dir/coordinator/coordinator.hex"
echo "  $build_dir/enddevice/enddevice.hex"
