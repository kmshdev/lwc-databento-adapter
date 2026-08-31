# Dedicated connectivity qualification

## Target

Qualify Databento Live Raw API delivery below 50 microseconds at the 90th percentile through the cross-connect option documented by Databento. This target is a physical network-handoff service level, not browser page load, gateway processing, or public-internet end-to-end latency.

Databento's published estimate is 42.4 microseconds at the 90th percentile. Its measurement boundary is first byte entering Databento's boundary switch to last byte leaving onto the customer cross-connect.

Authoritative source: [Databento dedicated connectivity guide](https://databento.com/docs/architecture/dedicated-connectivity-guide#cross-connect-with-any-colocation-or-managed-services-provider-msp?historical=http&live=raw&reference=http), verified 2026-08-31.

## Required physical state

All of the following must be true before qualification can begin:

- Service is Databento Live using the Raw API over TCP. Historical service and browser WebSockets are outside this latency boundary.
- Customer infrastructure is cross-connected to Databento at CyrusOne Aurora I (DC3) or Equinix NY4/5 through a colocation provider or managed services provider.
- The handoff uses a Databento-supported 10G or 25G port. Databento states that a single port is sufficient for reliable Raw API transmission in this setup.
- Databento has confirmed the service order and the cross-connect is installed, active, and associated with the intended Live entitlement.
- The gateway workload used for application qualification runs behind that handoff; a laptop, ordinary cloud instance, or public-internet route is not equivalent.

The repository contains no evidence that a site, provider, port, service order, or installed cross-connect has been selected. Creating any of those has commercial and external infrastructure effects and requires explicit user authorization.

## Acceptance evidence

The target is proved only by a dated receipt tied to the installed circuit that includes:

1. Site: DC3, NY4, or NY5.
2. Port speed: 10G or 25G.
3. Service and transport: Databento Live Raw API over TCP.
4. Measurement boundary: first byte into Databento's boundary switch through last byte out onto the customer cross-connect.
5. Percentile: 90th.
6. Measured latency: strictly below 50 microseconds.
7. Measurement owner and method, with enough provenance to associate the result with the installed circuit.

Databento's published 42.4-microsecond estimate demonstrates that the option is designed for the target; it is not proof that this project's unprovisioned connection achieves it. A local socket timer, ICMP ping, browser navigation timing, or gateway log cannot substitute for the specified boundary measurement.

## Application qualification after the circuit passes

Provider-boundary acceptance is followed by a separate application measurement. Use hardware or kernel timestamps appropriate to the selected host and NIC to characterize:

- cross-connect egress to gateway socket receive;
- socket receive to normalized base bar;
- normalized base bar to outbound adapter frame;
- end-to-end tails and loss under the selected feed load.

These application measurements must label their own boundaries and percentiles. They must not be combined with or reported as Databento's 42.4-microsecond handoff figure.

## Current status

`blocked_external`: the repository-side contract is defined, but no physical site, provider, port speed, commercial approval, installed circuit, or circuit-specific latency receipt is available. The existing real-provider tests use ordinary connectivity and prove API behavior only.
