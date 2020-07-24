function echo_c {
    printf "\033[0;$1m$2\033[0m\n"
}

function log_info {
    echo_c 33 "$1"
}

function log_success {
    echo_c 32 "$1"
}

function log_err {
    echo_c 35 "$1"
}

