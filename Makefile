common-start:
	$(MAKE) docker-build
	$(MAKE) docker-run
	$(MAKE) migrate-run
	@echo "Container is running and migrations applied."

docker-build:
	docker build -t zerox-postgres .

docker-run:
	docker run -d --env-file .env --name zerox-postgres -p 5434:5432 zerox-postgres

migrate-run:
	sqlx migrate run

migrate-info:
	sqlx migrate info

migrate-revert:
	sqlx migrate revert

clean:
	docker stop zerox-postgres || true
	docker rm zerox-postgres || true