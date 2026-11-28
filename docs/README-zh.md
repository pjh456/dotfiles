# Hyprland 配置（Dotfiles）

[English](../README.md) | 中文

我的个人 Arch dotfiles：带 systemd 用户服务管理的 Hyprland 配置，外加 shell 及部分编辑器周边配置。

![screenshot](screenshot.png)

## 架构

所有服务通过 `systemd --user` 管理：

- `hyprland-session.target` — 会话生命周期 target
- `hyprland.lua` 启动时执行 `~/.config/hypr/scripts/init.sh`（通过 `hyprland.start` 事件）
- `init.sh` 把 Wayland 环境变量导入 systemd，然后启动 `hyprland-session.target`
- 所有守护进程（`waybar`、`swaync`、`hyprpaper`、`hypridle`、`fcitx5-daemon`、`power-profile`、`hyprland-guard`）通过 `BindsTo=` / `PartOf=` 绑定到该 target；`mpv-profile-watch` 只需要 D-Bus，挂在 `default.target`

蓝牙是**按需启动**以省电的：空闲时 `bluetooth.service` 和无线电都是关闭的。waybar 的蓝牙模块（`dotctl status bt`）显示当前状态，点击它会先解除 rfkill、启动 `bluetooth.service`（经由 `install.sh` 配置的免密 sudoers 规则）、打开适配器并启动 `blueman-manager`。不使用托盘小程序（`blueman-applet`）。

因此 Hyprland 退出时，所有服务会被自动清理。

### Wayland 原生策略

能跑 Wayland 的都跑 Wayland，让弹窗尺寸/位置由合成器统一管理，而不是 XWayland：

- Qt：`QT_QPA_PLATFORM=wayland;xcb`
- GTK3：`GDK_BACKEND=wayland`（Thunar、pavucontrol、zathura、portal 文件选择窗）；
  `init.sh` 会把它导入 systemd/D-Bus 激活环境
- Firefox：`MOZ_ENABLE_WAYLAND=1`；Chromium：`chrome-flags.conf`；
  QQ：`qq-flags.conf`；其他 Electron：`ELECTRON_OZONE_PLATFORM_HINT=auto`
- Steam/Proton、Wine 和少数 SDL 游戏保持 XWayland（这是有意的）。

## 安装

已在 Arch / CachyOS 和 Debian（testing）上验证。

```bash
git clone https://github.com/pjh456/dotfiles ~/dotfiles
~/dotfiles/install.sh --packages
```

不带参数时 `install.sh` 会：

- 把已存在的 dotfiles 备份到 `~/.dotfiles-backup-<时间戳>/`，然后调用
  `dotctl install` 把 `etc/` 里的内容作为普通文件复制到 `$HOME`
  （备份、manifest 和 prune 都在 Rust 里）；
- 把固定版本的 `dotctl` release 装到 `~/.local/bin`（校验 checksum；
  `--no-cli` 只使用已有二进制，`--cli-from-source` 强制本地编译）；
- 启用主程序已安装的会话服务（`systemctl --user`）；
- 写入按需蓝牙的 NOPASSWD sudoers 规则（`systemctl` start/stop 和
  `rfkill unblock`），并记住已写入，重跑时不再要求输入密码；
- 写入背光 udev 规则，给 `video` 组 `/sys/class/backlight/*/brightness`
  的写权限，让 `brightnessctl` 免 sudo 可用（需要时把用户加入 `video`
  组；其他机器上可能要重新登录）；
- 写入 `/etc/tlp.d/90-dotfiles-profile.conf`，让 TLP 自己在 AC 上选
  `PRF`、电池上选 `SAV`；没有 TLP PPD 接口时仍由 dotctl watcher 兜底；
- 注释掉 `/etc/acpi/handler.sh` 里的音量/亮度命令（仅当装了 `acpid`），
  避免 acpid 和 Hyprland 的多媒体键绑定对同一次按键各执行一遍。

选项：

| 选项 | 作用 |
|---|---|
| `--packages` | 额外安装所需软件包——官方源走 `pacman`（Arch）或 `apt`（Debian），若检测到 `yay`/`paru`/`buttercup` 则 AUR 走它们；Debian 上改为打印需要手动编译的清单。清单位于 [`packages/`](../packages/)，由 CI 对着实时数据库校验。 |
| `--no-<pkg>` | 配合 `--packages` 使用：不安装软件包 `<pkg>`。包名必须存在于当前发行版的清单中，否则报错。 |
| `--restore [DIR]` | 回滚一次部署：删除已部署的文件（按 `~/.dotfiles-deployed` 清单），并从 `DIR` 还原备份的原始文件（默认：最新的 `~/.dotfiles-backup-*`）。systemd 启用、sudoers 规则和 acpid 改动不会回滚。 |
| `--with-cli` | 安装/更新 `dotctl`（默认）。下载固定版本的 `dotctl-v<version>` release 并校验 `SHA256SUMS`，必要时回退到 `cargo build`。 |
| `--cli-from-source` | 不走 release，本地编译 `dotctl`。 |
| `--no-cli` | 不安装/更新 `dotctl`；使用已安装的二进制或 `DOTCTL_BIN`（安装器本身需要它）。 |
| `--no-systemd` | 跳过 `daemon-reload` 和会话服务启用。 |
| `--no-sudo` | 跳过系统级调整：蓝牙 sudoers 规则、背光 udev 规则、TLP profile 策略、acpid 多媒体键去重。 |
| `-h`, `--help` | 显示用法。 |

伴生包会连带排除：`--no-tlp` 同时排除 `tlp-pd`。

脚本是幂等的——重复运行安全。

首次运行前，确认以下 `pass` 条目已存在（`.bashrc` 在 shell 启动时读取）：
`snyk/token`、`huggingface/token`。

### Debian 说明

在 Debian 上，`--packages` 通过 `apt` 安装
[`packages/debian.txt`](../packages/debian.txt)，并打印需要手动编译的软件包。
Debian 稳定版（trixie）的 main 里还没有 hyprland——请用 testing，或 stable
加上 trixie-backports。两个细节：

- `tlp` 与 `power-profiles-daemon` 冲突。waybar 的
  `power-profiles-daemon` 模块（以及 `dotctl watch power` 服务）需要
  `net.hadess.PowerProfiles` D-Bus 接口，tlp 1.10 已内置该功能——在
  `/etc/tlp.conf` 里设置 `TLP_PD_ENABLE=1` 启用。
- Arch 的 `fcitx5-gtk` 在 Debian 上叫 `fcitx5-frontend-gtk3` /
  `fcitx5-frontend-gtk4`。

#### 手动编译（Debian）

以下没有 Debian 软件包，需从源码构建：

| 包名                        | 源码                                                        |
| -------------------------- | ----------------------------------------------------------- |
| hypridle                   | https://github.com/hyprwm/hypridle                            |
| hyprlock                    | https://github.com/hyprwm/hyprlock                            |
| hyprpaper                   | https://github.com/hyprwm/hyprpaper                           |
| hyprswitch                  | https://github.com/hyprwm/hyprswitch                          |
| swaync                      | https://github.com/elkowar/swaync                             |
| fonts-jetbrains-mono-nerd   | https://github.com/nerd-fonts/nerd-fonts（安装脚本）          |
| rofi-emoji                  | https://github.com/Mange/rofi-emoji                          |

然后启动 Hyprland：

```bash
start-hyprland
```

每次启动都会重启 `hyprland-session.target`，让所有服务重新连上新合成器的 Wayland socket。崩溃重启后服务可能仍是 `active`，但连接早已断开——`fcitx5` 就会这样静默失去输入法，所以只拉起未运行的服务是不够的。若拉起 `init.sh` 的 Hyprland 实例已经消失（快速重启的竞态），脚本会直接退出。

## 日常使用

仓库的 `etc/` 是唯一事实来源，`$HOME` 里是普通文件拷贝。修改配置时编辑
仓库里的文件，然后重跑 `~/dotfiles/install.sh` 应用——未变化的文件不动，
变化过的重新拷贝。直接改 `$HOME` 里的文件不会进仓库，想保留的话先拷回
`etc/` 再提交。

## 回滚

```bash
~/dotfiles/install.sh --restore            # 用最新备份
~/dotfiles/install.sh --restore <dir>      # 或指定 ~/.dotfiles-backup-*/
```

删除已部署的文件（按 `~/.dotfiles-deployed` 清单），并把备份的原始文件
还原。不会回滚的部分：systemd 服务启用、sudoers 规则。

## 快捷键

| 按键                   | 功能                                           |
| ---------------------- | ---------------------------------------------- |
| `Super + Return`      | 打开 foot 终端                                 |
| `Super + Q`           | 关闭窗口                                       |
| `Super + Space`       | Rofi 应用启动器                                |
| `Super + .`           | 彩色 emoji 选择器（直接输入到焦点窗口；没装 `wtype` 时复制到剪贴板） |
| `Super + L`           | 电源菜单（关机/重启/锁屏/休眠/注销）           |
| `Alt + Tab`           | Hyprswitch 窗口切换                            |
| `Super + V`           | 剪贴板历史（cliphist + rofi）                  |
| `Super + W`           | 天气弹窗（与点击左上角 Waybar 天气模块相同的 7 天预报） |
| `Super + N`           | 切换通知中心（swaync）                         |
| `Super + F5`          | 重载 Hyprland + 重启 Waybar                    |
| `Ctrl + Alt + A`      | 区域截图（grim + slurp）                       |
| `Super + F`           | 全屏                                           |
| Waybar 蓝牙图标        | 点击：启动 `bluetooth.service` + 打开适配器，然后打开 `blueman-manager`（已开则无动作） |
| `Super + 1-5`         | 切换工作区                                     |
| `Super + Shift + 1-5` | 移动窗口到工作区                               |
| `Super + R`           | 缩放模式（方向键调整大小）                     |
| `XF86Audio*` / `XF86MonBrightness*` | 音量 / 亮度 OSD 弹窗（swaync） |

## dotctl

桌面辅助工具现在是 `~/.local/bin/dotctl` 一个 Rust 二进制。Waybar、Hyprland
快捷键、hypridle 和 systemd user unit 都调它，旧的 shell 脚本已删除。

`install.sh` 会下载固定版本的 `dotctl-v<version>` release（x86_64 musl 静态
二进制），校验 `SHA256SUMS`，下载不可用时回退到本地 `cargo build`。
`--no-cli` 只使用已有二进制（或 `DOTCTL_BIN`），安装器本身仍需要它；
`--cli-from-source` 强制本地编译。

热路径不再 fork 一堆进程：`status temp` 直接读 hwmon（约 0.5ms，旧的
`sensors | awk` 约 20ms）；`status bt` 不再调用 `bluetoothctl`（服务停止时
它可能卡几分钟）；mpv / power watcher 改成事件驱动而不是轮询。

### 手动命令

| 命令 | 作用 |
|---|---|
| `dotctl wallpaper set <图片>` | 从任意文件设壁纸（走 `hyprpaper`） |
| `dotctl wallpaper unset` | 切回纯黑壁纸（保留 hyprpaper 进程） |
| `dotctl menu power` | Rofi 电源菜单——`Super + L` 也是调它，再按一次关闭 |
| `dotctl menu clipboard` | Rofi 剪贴板历史，复制后带通知和 `--paste-once` |
| `dotctl menu emoji [--paste]` | 彩色 emoji 选择器；`--paste` 复制并发送 Ctrl+V（QQ 用） |
| `dotctl rofi -show drun` | 切换任意 rofi 菜单：没开就开，已开就关 |
| `dotctl mpv [profile]` | 立即把 `mpv.conf.{PRF,BAL,SAV}` 链接成 `mpv.conf` |
| `dotctl reload waybar` | 改完 waybar 配置后重启它 |
| `dotctl on-battery` | 仅在电池供电时返回 0（给 hypridle 用） |
| `dotctl doctor` | 检查 hwmon、D-Bus、Hyprland socket 和外部命令 |
| `dotctl guard --once` | 探测一次 Hyprland 是否活着；不带 `--once` 常驻镜像日志 |
| `dotctl install --repo-root ~/dotfiles` | 把 `etc/` 部署到 `$HOME`（通常由 `install.sh` 调用） |
| `dotctl restore [DIR]` | 回滚一次部署（通常由 `install.sh --restore` 调用） |

### 配置自动调用

| 命令 | 调用方 |
|---|---|
| `dotctl status temp` | Waybar 温度模块（5 秒轮询） |
| `dotctl status bt`、`dotctl bt toggle` | Waybar 蓝牙模块（5 秒轮询 + 点击） |
| `dotctl weather`、`dotctl weather --forecast` | Waybar 天气模块（30 分钟轮询）/ `Super + W` |
| `dotctl osd volume up\|down\|mute`、`dotctl osd brightness up\|down` | Hyprland 多媒体键 |
| `dotctl menu clipboard\|emoji\|power\|launcher` | `Super + V`、`Super + .`、`Super + L`、`Super + Space` |
| `dotctl watch mpv` | `mpv-profile-watch.service`（zbus PropertiesChanged） |
| `dotctl watch power` | `power-profile.service`（1 秒 sysfs 轮询；只在 AC↔电池转换时动作） |
| `dotctl guard` | `hyprland-guard.service`（日志镜像 + 卡死抓栈） |

电源 profile watcher 和 TLP 的自动切换一样，只在电源来源变化时动作。拔电后
如果你手动切到 `balanced` 或 `performance`，它会一直保持到你下次插拔电源。

### 开发

```bash
cargo test --manifest-path dotctl/Cargo.toml
cargo clippy --manifest-path dotctl/Cargo.toml --all-targets -- -D warnings
```

改完 `dotctl/Cargo.toml` 的版本后打 `dotctl-vX.Y.Z` tag 并推送。release
workflow 会构建 x86_64-musl 静态二进制、校验 tag 与版本一致、发布
`SHA256SUMS`，并在 Arch 容器里对产物做一次冒烟测试。

## 文件结构

```
├── install.sh                 # 部署 / 回滚入口
├── dotctl/                    # Rust CLI（单个静态 musl 二进制）
│   ├── Cargo.toml
│   └── src/
├── docs/
│   └── README-zh.md           # 中文文档（本文件）
├── packages/
│   ├── arch-official.txt      # pacman 清单（CI 校验）
│   ├── arch-aur.txt           # AUR 清单（yay/paru/buttercup）
│   ├── debian.txt             # apt 清单（CI 校验）
│   └── debian-manual.txt      # Debian 手动编译清单
├── etc/                       # $HOME 的 1:1 镜像（install.sh 拷贝就位）
│   ├── .bashrc                # 别名、starship、fzf、thefuck
│   ├── .bash_profile
│   ├── .gitconfig
│   ├── .config/
│   │   ├── chrome-flags.conf  # Chromium/Electron：Wayland 原生 + Intel VA-API
│   │   ├── qq-flags.conf      # QQ：原生 Wayland，避开 XWayland 弹窗尺寸问题
│   │   ├── starship.toml      # Catppuccin 双行提示符
│   │   ├── fastfetch/         # 分组系统信息卡片
│   │   ├── btop/              # Catppuccin 主题、透明背景
│   │   ├── hypr/              # hyprland.lua（+anim/keys/notify）、hypridle、hyprlock、hyprpaper、scripts/init.sh
│   │   ├── waybar/            # 状态栏配置与样式
│   │   ├── rofi/              # 启动器配置
│   │   ├── swaync/            # 通知中心配置与样式
│   │   ├── fcitx5/            # 输入法配置
│   │   ├── foot/              # 终端配置
│   │   └── systemd/user/      # 会话服务（*.wants/ 不入库）
│   └── .local/share/
│       └── fcitx5/themes/     # 自包含的 fcitx5 主题（svg + png）
└── .github/workflows/         # shellcheck/JSON/安装测试 + dotctl CI/release
```
