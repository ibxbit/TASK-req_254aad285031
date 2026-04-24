# Documentation Checklist: Questions

Clarifications resolved during implementation of the Field Service
Operations Hub. Each entry documents a spec ambiguity and how we chose
to resolve it.

---

**Q:** How are forum moderator permissions scoped?
**A:** Moderators are assigned per board; their actions (pin, remove,
enforce rules) are limited to the boards they moderate.

**Q:** How is the offline "distance" filter implemented?
**A:** Uses locally stored site ZIP codes and per-service coverage radii
(miles) to compute distance without any external geocoding API.

**Q:** How are late internship reports flagged?
**A:** Weekly reports due Mondays 12:00 local, monthly reports due the
5th at 17:00 local. Submissions up to 72 h past the deadline are
accepted but auto-flagged and counted in progress dashboards.

**Q:** How is review anti-spam enforced?
**A:** Maximum 3 reviews per user per day and strictly 1 review per
order, enforced at the API layer.

**Q:** How is face data validated on capture?
**A:** Images must meet minimum 320x320 resolution, acceptable
brightness range, and low-blur thresholds; operator confirms a single
frontal face, with an optional turn-left/right liveness challenge.

**Q:** How are audit / review / internship logs tamper-proofed?
**A:** Append-only event logs with hash chaining; edits are permitted
only where policy allows and remain traceable via the chain.

**Q:** How is sensitive data protected at rest and in the UI?
**A:** Sensitive identifiers are encrypted at rest and masked in the
UI; face media and attachments are stored locally with content hashes.
