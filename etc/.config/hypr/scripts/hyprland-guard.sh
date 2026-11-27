#!/bin/bash
# Hyprland 现场保护脚本
# 1) 把 Hyprland 的临时日志($XDG_RUNTIME_DIR/hypr/.../hyprland.log)持续镜像到磁盘
#    因为 /run/user/1000 是 tmpfs，重启后就没了
# 2) 用 hyprctl 探活，一旦合成器主循环卡住(>2s 无响应)，抓线程现场
#
# 日志输出: ~/.local/state/hyprland-guard/

set -u

STATE="$HOME/.local/state/hyprland-guard"
mkdir -p "$STATE"
RT="${XDG_RUNTIME_DIR:-/run/user/$(id -u)}"

# 单实例
exec 9>"$STATE/.lock"
flock -n 9 || exit 0

log_path() {
    ls -t "$RT"/hypr/*/hyprland.log 2>/dev/null | head -1
}

fails=0
tick=0
last_capture=0

while :; do
    LOG="$(log_path)"
    if [ -n "$LOG" ] && [ -f "$LOG" ]; then
        # 每 3 秒镜像一次日志
        if [ $((tick % 3)) -eq 0 ]; then
            cp -f "$LOG" "$STATE/current.log" 2>/dev/null
        fi

        SIG="$(basename "$(dirname "$LOG")")"
        if HYPRLAND_INSTANCE_SIGNATURE="$SIG" timeout 2 hyprctl -j version >/dev/null 2>&1; then
            fails=0
        else
            fails=$((fails + 1))
        fi

        if [ "$fails" -ge 2 ] && [ $(( $(date +%s) - last_capture )) -gt 60 ]; then
            last_capture=$(date +%s)
            stamp="$(date +%Y%m%d-%H%M%S)"
            PID="$(pgrep -x Hyprland | head -1)"
            out="$STATE/hang-$stamp.txt"
            {
                echo "== 时间: $(date -Is)"
                echo "== Hyprland pid: ${PID:-?}   instance: $SIG"
                echo "== uptime/load: $(uptime)"
                echo
                if [ -n "${PID:-}" ]; then
                    echo "== 主线程 stat (state= D 不可中断 / R 运行 / S 睡眠):"
                    awk '{print "   state="$3"  utime="$14" stime="$15"  threads="$20}' "/proc/$PID/stat" 2>/dev/null
                    echo
                    echo "== 各线程: comm / state / wchan / syscall(号+参数)"
                    for t in /proc/"$PID"/task/*; do
                        tid="$(basename "$t")"
                        printf '   #%-7s %-18s state=%s  wchan=%s\n' "$tid" \
                            "$(cat "$t/comm" 2>/dev/null)" \
                            "$(awk '{print $3}' "$t/stat" 2>/dev/null)" \
                            "$(cat "$t/wchan" 2>/dev/null)"
                        printf '            syscall: %s\n' "$(cat "$t/syscall" 2>/dev/null)"
                    done
                    echo
                    echo "== 打开的关键 fd:"
                    # shellcheck disable=SC2010  # 纯诊断输出，文件名不会是恶意的
                    ls -l "/proc/$PID/fd" 2>/dev/null | grep -E "dri|wayland|socket" | head -30
                    echo
                    echo "== gdb 全线程 backtrace (ptrace_scope=1 时会失败):"
                    gdb -p "$PID" -batch -ex "thread apply all bt" 2>&1 | head -500
                fi
            } > "$out" 2>&1

            cp -f "$LOG" "$STATE/log-at-hang-$stamp.log" 2>/dev/null
            fails=0
        fi
    else
        fails=0
    fi

    tick=$((tick + 1))
    sleep 1
done
