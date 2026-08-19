# SuperApp — local dev entrypoints (TR-02-006).
.PHONY: up down logs ps env-check test test-infra hooks test-deploy

hooks:         ## install the Conventional Commits commit-msg hook (TR-10-008)
	git config core.hooksPath scripts/hooks
	@echo "commit-msg hook installed (scripts/hooks) — core.hooksPath set"

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

test-deploy:   ## run P10 deploy/CI acceptance tests (static validation)
	./tests/test-deploy.sh

test: test-infra test-deploy ## run all repo-level tests
