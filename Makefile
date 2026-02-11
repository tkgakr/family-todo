.PHONY: build-backend build-frontend build deploy clean

build-backend:
	cd backend && cargo lambda build --release --arm64

build-frontend:
	cd frontend && bun install --frozen-lockfile && bun run build

build: build-backend build-frontend

deploy:
	sam build && sam deploy

clean:
	cd backend && cargo clean
	rm -rf frontend/dist frontend/node_modules .aws-sam
