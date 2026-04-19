# Documentation Checklist: Questions

## Document your understanding of business gaps

**Question:** How to handle expired matches?

**Hypothesis:** Auto-cancel after 3 mins per prompt.

**Solution:** Implemented background cleanup logic.

---

## Additional Questions & Answers

**Q:** How are forum moderator permissions scoped?
**A:** Moderators are assigned per board; their actions (pin, remove, enforce) are limited to boards they moderate.

**Q:** How is offline distance filtering implemented?
**A:** Uses locally stored ZIP codes and service radii; computes distance without external APIs.

**Q:** How are late internship reports flagged?
**A:** Reports submitted >72h late are auto-flagged and included in dashboards.

**Q:** How is review anti-spam enforced?
**A:** Max 3 reviews/user/day, 1 review/order, enforced at API level.

**Q:** How is face data validated?
**A:** Images checked for resolution, brightness, blur, single face, and optional liveness challenge.

**Q:** How is audit log tamper-proofing achieved?
**A:** Append-only event logs with hash chaining; all edits are traceable.

**Q:** How is sensitive data protected?
**A:** All sensitive fields encrypted at rest and masked in the UI; media stored locally with hashes.
