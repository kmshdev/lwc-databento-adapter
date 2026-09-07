# Dedicated connectivity RFQ packet

## Recommended pilot

Request a 10G single-port Databento Live Raw TCP cross-connect at CyrusOne Aurora I (DC3/CHI1) with a small managed bare-metal or 1U footprint.

This recommendation matches the currently qualified `GLBX.MDP3` / ES workflow. CME states that its Aurora colocation facility houses the Globex matching engines, and Databento states that its CME live gateways are in CyrusOne Aurora I. Request 25G only as an alternate because no measured project throughput requires it.

## Provider shortlist

| Priority | Provider | Current evidence | Qualification gap |
| --- | --- | --- | --- |
| 1 | Beeks Financial Cloud | Beeks advertises rack space, dedicated bare metal, 1U through full cabinets, and cross-connect services in CME Aurora DC3. CME's official vendor directory lists Beeks at Aurora and identifies hosting and managed infrastructure services. | Must confirm the server is physically in Aurora I, not only elsewhere on the Aurora campus, and that a direct cross-connect to Databento is available. |
| 2 | Options Technology | CME's official directory lists managed colocation, hardware, networking, low-latency market data, and shared or dedicated environments at Aurora CME Co-Location. | Must confirm small-footprint availability, exact Aurora I demarc, Databento cross-connect support, and commercial terms. |
| 3 | Avelacom | CME's official directory lists Aurora CME Co-Location plus ultra-low-latency connectivity, colocation, managed infrastructure, fiber, and microwave services. | Best treated as a network/managed-infrastructure alternative until compute footprint and direct Databento cross-connect are confirmed. |

Presence in CME's directory is not Databento certification. No provider passes until Databento and the provider both confirm the exact circuit.

Sources verified 2026-08-31:

- [CME colocation](https://www.cmegroup.com/solutions/co-location.html)
- [CME vendor: Beeks](https://www.cmegroup.com/solutions/market-tech-and-data-services/technology-vendor-services/beeks-financial-cloud.html)
- [CME vendor: Options Technology](https://www.cmegroup.com/solutions/market-tech-and-data-services/technology-vendor-services/options-it.html)
- [CME vendor: Avelacom](https://www.cmegroup.com/solutions/market-tech-and-data-services/technology-vendor-services/avelacom.html)
- [Beeks CME Connect](https://beeksgroup.com/services/connectivity/colocation-hosting/cme-connect/)
- [Databento CME colocation overview](https://databento.com/blog/cme-colocation)

## Official outreach routes

Use these routes for the RFQ; they were verified on 2026-08-31:

| Recipient | Official route |
| --- | --- |
| Databento | [Contact sales](https://databento.com/contact) |
| Beeks Financial Cloud | [Product or service enquiry](https://beeksgroup.com/contact/) |
| Options Technology | [sales@options-it.com](mailto:sales@options-it.com) |
| Avelacom | [sales@avelacom.com](mailto:sales@avelacom.com) |

Sending the RFQ is an external action and requires explicit approval. Use an approved private channel if the message includes account, entitlement, commercial, or strategy details.

## Send to Databento and each provider

**Subject:** RFQ: Databento Live Raw TCP cross-connect at CyrusOne Aurora I

We are qualifying a Databento Live Raw API deployment for a 90th-percentile physical handoff below 50 microseconds. Please quote and confirm the following without placing an order:

1. The compute and Databento demarc are in CyrusOne Aurora I/DC3/CHI1 at 2905 E. Diehl Road, not Aurora II/III or a Chicago proximity site.
2. A direct 10G cross-connect from the proposed footprint to Databento Live is available. Include 25G as an alternate quote.
3. The service uses Databento Live Raw API over TCP and is compatible with the intended `GLBX.MDP3` entitlement.
4. Handoff details: media, optics, connector, VLAN/routing model, addressing, gateway hostname or route changes, MTU, and any customer equipment requirements.
5. Compute options: dedicated bare metal or 1U, CPU model, NIC model, PCIe topology, memory, local storage, remote hands, and replacement service level.
6. Timing options: PTP or other clock service, supported hardware timestamping, and timestamp provenance.
7. Commercials: non-recurring charge, monthly recurring charge, minimum term, setup lead time, support coverage, and all Databento/provider cross-connect fees.
8. Measurement: who will produce a dated p90 receipt for first byte entering Databento's boundary switch through last byte leaving onto this customer cross-connect, and how the receipt will be tied to the installed circuit.
9. Resilience: single-port failure handling and separately priced redundant-port or diverse-path options. The initial qualification remains single-port unless explicitly approved.

Please identify every assumption and distinguish installed-circuit measurements from published estimates or application latency.

## Decision gate

Do not place an order based on marketing latency, an Aurora-campus location, or a generic CME connection. Select a provider only after written confirmation of:

- exact Aurora I location;
- direct Databento cross-connect;
- 10G or 25G Live Raw TCP handoff;
- complete price and term;
- circuit-specific p90 measurement ownership.

The RFQ contains no account identifier, API key, entitlement identifier, or trading strategy. Add sensitive commercial or account data only in an approved private channel.

## Response ledger

Update this table only from written responses. A blank or marketing claim is not confirmation.

| Party | Contact status | Aurora I confirmed | Direct Databento cross-connect confirmed | Price and term | Measurement owner |
| --- | --- | --- | --- | --- | --- |
| Databento | Not contacted | Pending | Pending | Pending | Pending |
| Beeks Financial Cloud | Not contacted | Pending | Pending | Pending | Pending |
| Options Technology | Not contacted | Pending | Pending | Pending | Pending |
| Avelacom | Not contacted | Pending | Pending | Pending | Pending |
