#!/usr/bin/env bash
set -euo pipefail

readonly VERSION="v0.1.0"
readonly MAX_ENTRIES=80

print_kv() {
  printf '%s=%s\n' "$1" "$2"
}

read_first_line() {
  local path="$1"
  local value=""
  if [[ -r "$path" ]]; then
    IFS= read -r value < "$path" || true
    value="${value//$'\r'/}"
    value="${value//$'\n'/}"
    printf '%s' "$value"
  fi
}

probe_directory() {
  local name="$1"
  local path="$2"
  local listing=""
  local count=0

  printf '\n== %s ==\n' "$name"
  print_kv path "$path"

  if [[ ! -e "$path" ]]; then
    print_kv present no
    print_kv readable no
    print_kv entries 0
    return 0
  fi

  print_kv present yes
  if [[ ! -d "$path" ]]; then
    print_kv readable no
    print_kv error not_a_directory
    return 0
  fi

  if ! listing="$(LC_ALL=C ls -1A "$path" 2>/dev/null)"; then
    print_kv readable no
    print_kv error permission_denied_or_unreadable
    return 0
  fi

  print_kv readable yes
  if [[ -n "$listing" ]]; then
    count="$(printf '%s\n' "$listing" | awk 'END { print NR }')"
  fi
  print_kv entries "$count"
  if (( count > 0 )); then
    printf '%s\n' "$listing" | awk -v max="$MAX_ENTRIES" 'NR <= max { print "entry=" $0 } NR == max + 1 { print "entry=...truncated..." }'
  fi
}

print_selected_attributes() {
  local path="$1"
  shift
  local attribute=""
  local value=""

  for attribute in "$@"; do
    value="$(read_first_line "$path/$attribute")"
    if [[ -n "$value" ]]; then
      print_kv "$attribute" "$value"
    fi
  done
}

scan_typec() {
  local base="/sys/class/typec"
  local node=""
  local name=""

  probe_directory typec "$base"
  [[ -d "$base" ]] || return 0

  shopt -s nullglob
  for node in "$base"/port[0-9]*; do
    [[ -d "$node" ]] || continue
    name="${node##*/}"
    [[ "$name" =~ ^port[0-9]+$ ]] || continue
    printf '\n-- typec_port=%s --\n' "$name"
    print_selected_attributes "$node" \
      data_role power_role port_type power_operation_mode orientation \
      usb_power_delivery_revision usb_typec_revision

    local partner="${node}-partner"
    if [[ -d "$partner" ]]; then
      print_kv partner_present yes
      print_selected_attributes "$partner" type
      if [[ -d "$partner/identity" ]]; then
        print_kv partner_identity yes
        print_selected_attributes "$partner/identity" \
          id_header cert_stat product product_type_vdo1 product_type_vdo2 product_type_vdo3
      fi
    else
      print_kv partner_present no
    fi

    local cable="${node}-cable"
    if [[ -d "$cable" ]]; then
      print_kv cable_present yes
      print_selected_attributes "$cable" type plug_type
      if [[ -d "$cable/identity" ]]; then
        print_kv cable_identity yes
        print_selected_attributes "$cable/identity" \
          id_header cert_stat product product_type_vdo1 product_type_vdo2 product_type_vdo3
      fi
    else
      print_kv cable_present no
    fi
  done
  shopt -u nullglob
}

scan_power_delivery() {
  local base="/sys/class/usb_power_delivery"
  local node=""

  probe_directory usb_power_delivery "$base"
  [[ -d "$base" ]] || return 0

  shopt -s nullglob
  for node in "$base"/*; do
    [[ -d "$node" ]] || continue
    printf '\n-- pd_node=%s --\n' "${node##*/}"
    print_selected_attributes "$node" \
      revision version power_role data_role status contract operating_power maximum_power
  done
  shopt -u nullglob
}

scan_power_supply() {
  local base="/sys/class/power_supply"
  local node=""

  probe_directory power_supply "$base"
  [[ -d "$base" ]] || return 0

  shopt -s nullglob
  for node in "$base"/*; do
    [[ -d "$node" ]] || continue
    printf '\n-- power_supply=%s --\n' "${node##*/}"
    print_selected_attributes "$node" \
      type usb_type online present status charge_type \
      voltage_now current_now power_now voltage_max current_max input_current_limit
  done
  shopt -u nullglob
}

scan_usb_devices() {
  local base="/sys/bus/usb/devices"
  local node=""
  local shown=0

  probe_directory usb_devices "$base"
  [[ -d "$base" ]] || return 0

  shopt -s nullglob
  for node in "$base"/*; do
    [[ -d "$node" ]] || continue
    [[ -r "$node/idVendor" || -r "$node/idProduct" ]] || continue
    printf '\n-- usb_device=%s --\n' "${node##*/}"
    print_selected_attributes "$node" \
      idVendor idProduct manufacturer product speed version bMaxPower bDeviceClass
    shown=$((shown + 1))
    if (( shown >= MAX_ENTRIES )); then
      print_kv usb_device_output truncated
      break
    fi
  done
  shopt -u nullglob
}

main() {
  printf '== usbeehive Android Pixel capability scan ==\n'
  print_kv version "$VERSION"
  print_kv generated_at "$(date -Is 2>/dev/null || date)"
  print_kv uid "$(id -u)"
  print_kv user "$(id -un 2>/dev/null || printf unknown)"
  print_kv kernel "$(uname -srmo 2>/dev/null || uname -a)"
  if command -v getprop >/dev/null 2>&1; then
    print_kv android_device "$(getprop ro.product.device)"
    print_kv android_release "$(getprop ro.build.version.release)"
    print_kv android_sdk "$(getprop ro.build.version.sdk)"
    print_kv security_patch "$(getprop ro.build.version.security_patch)"
  fi

  scan_usb_devices
  scan_typec
  scan_power_delivery
  scan_power_supply

  printf '\nRESULT: USBEEHIVE_ANDROID_CAPABILITY_SCAN_DONE rc=0\n'
}

main "$@"
