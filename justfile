set shell := ["bash", "-c"]
set dotenv-path := "./config/dev.env"
engine := `if command -v docker >/dev/null 2>&1; then echo "docker"; else echo "podman"; fi`

check:
    just precommit-shared
    nix flake update
    nix flake check -v

precommit:
    just precommit-shared
    cargo check
    just test

# surrealdb, surrealdb.dev
default := "surrealdb"
up file = default:
    just down {{file}}
    {{engine}} compose -f docker-compose-{{file}}.yml up --build --pull always -d

down file = default:
    {{engine}} compose -f docker-compose-{{file}}.yml down --volumes --remove-orphans
    {{engine}} network prune -f

view:
    {{engine}} attach wikidata-to-surrealdb

alias t := test
test:
    cargo t --no-fail-fast

precommit-shared:
    cargo upgrade -v
    cargo update
    cargo fmt --all
    just clippy

alias fmt := clippy
clippy:
    cargo fmt --all
    tombi fmt
    cargo clippy --all-targets --workspace --fix --allow-dirty -- -W clippy::nursery -W rust-2018-idioms \
        -A clippy::future_not_send -A clippy::option_if_let_else -A clippy::or_fun_call
    cargo machete --fix
