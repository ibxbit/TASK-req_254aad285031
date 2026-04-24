# Prebuilt image for the Field Service Operations Hub Playwright E2E suite.
# Dependencies and specs are baked in at build time so the runtime container
# needs no network access and no `npm install` step.
FROM mcr.microsoft.com/playwright:v1.44.0-jammy

WORKDIR /e2e

COPY package.json package-lock.json ./
RUN npm ci

COPY playwright.config.ts ./
COPY tests ./tests

CMD ["npx", "playwright", "test"]
