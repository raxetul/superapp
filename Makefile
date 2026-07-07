# SuperApp — local dev entrypoints (TR-02-006).
.PHONY: up down logs ps env-check test test-infra

up:            ## start the internal infrastructure stack
	docker compose up -d

down:          ## stop and remove the stack
	docker compose down

logs:          ## tail logs for all services
	docker compose logs -f

ps:            ## list service status
	docker compose ps

env-check:     ## verify connectivity to all infrastructure dependencies
	./scripts/check-connectivity.sh

test-infra:    ## run P2 infra acceptance tests
	./tests/test-infra.sh

test: test-infra ## run all repo-level tests
