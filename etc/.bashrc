#
# ~/.bashrc
#

# If not running interactively, don't do anything
[[ $- != *i* ]] && return

eval "$(starship init bash)"

alias ls='eza --icons=auto'
alias cat='bat --paging=never'
alias pacman='pacman --color always'
alias find='fd'
alias df='duf'

alias grep='rg --color=auto'
alias zed='zeditor'

alias pon='export http_proxy="http://127.0.0.1:7890"; export https_proxy="http://127.0.0.1:7890"; export no_proxy="localhost,127.0.0.1,.edu.cn,.aliyun.com,.ustc.edu.cn,.hit.edu.cn,.bfsu.edu.cn"'
alias poff='unset http_proxy; unset https_proxy; unset no_proxy'
alias baidupcs='http_proxy="" https_proxy="" baidupcs'

pon

alias ssh='TERM=xterm-256color ssh'

eval "$(fzf --bash)"
eval "$(thefuck --alias)"

alias qq='/opt/QQ/qq'
alias rm='trash-put'
tldr() {
  if [[ "$1" == "--update" || "$1" == "-u" ]]; then
    LANG=zh_CN.UTF-8 LC_ALL= command tldr --update
  else
    command tldr "$@"
  fi
}

if [[ "$TERM" == "linux" ]] && [[ -z "$FBTERM" ]]; then
  export LANG=en_US.UTF-8
  export LC_ALL=en_US.UTF-8
fi

alias senrenbanka='steam steam://rungameid/1144400'

export CARGO_BUILD_JOBS=8
export RUSTC_WRAPPER=sccache

export PATH=~/.bun/bin:$PATH
export PATH=~/.npm-global/bin:$PATH

# uv
export TERMINAL=foot
export PATH="$HOME/.local/bin:$PATH"

# API KEY
export SNYK_TOKEN=$(pass snyk/token)
export HF_TOKEN=$(pass huggingface/token)
