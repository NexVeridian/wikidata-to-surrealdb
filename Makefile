COMPOSE_FILES := surrealdb surrealdb.dev

.PHONY: $(addprefix up-,$(COMPOSE_FILES)) $(addprefix down-,$(COMPOSE_FILES))

$(addprefix up-,$(COMPOSE_FILES)):
	make down-$(subst up-,,$@)
	docker compose -f docker-compose-$(subst up-,,$@).yml up --build --pull always -d

$(addprefix down-,$(COMPOSE_FILES)):
	docker compose -f docker-compose-$(subst down-,,$@).yml down --volumes --remove-orphans
	docker network prune -f

view:
	docker attach wikidata-to-surrealdb

precommit:
	rustup update
	cargo update
	cargo check
	cargo fmt
	cargo t
	cargo clippy --fix --allow-dirty

check:
	rustup update
	cargo update
	nix flake update
	nix flake check
	cargo clippy --fix --allow-dirty
