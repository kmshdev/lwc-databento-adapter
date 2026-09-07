# Security Policy

## Supported versions

The project is in online beta (`0.1.x`, unpublished to npm). There is no long-term support branch yet; security fixes land on `main`.

## Reporting a vulnerability

Do not open a public issue for a security vulnerability. Report it through a [GitHub Security Advisory](https://github.com/kmshdev/lwc-databento-adapter/security/advisories/new) so the report stays private until a fix is available.

Include:
- The affected component (`services/databento-gateway`, `packages/databento-lightweight-charts`, or `examples/lightweight-charts-demo`).
- Steps to reproduce, or a minimal proof of concept.
- The potential impact (for example, credential exposure, unbounded resource use, or a protocol bypass).

## Scope

In scope:
- Exposure of `DATABENTO_API_KEY` or other credentials to the browser or logs.
- Gateway request handling that allows unbounded resource consumption (see `docs/technical-design.md` for the enforced limits).
- Wire-protocol handling that lets one client affect another client's session.

Out of scope:
- Vulnerabilities in `lightweight-charts` itself; report those to [TradingView](https://github.com/tradingview/lightweight-charts).
- Vulnerabilities in the Databento API or client libraries; report those to [Databento](https://databento.com).
