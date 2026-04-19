# API Specification

## Authentication
- Local username/password (min 12 chars)
- Lockout: 15 min after 5 failed attempts
- Roles: Administrator, Moderator, Service Manager, Warehouse Manager, Mentor, Intern, Requester

## Forums
- Multi-level: Zone → Board
- Board rules, announcements, visibility (public/restricted), moderators (scoped permissions)
- Moderator actions: pin post, remove comment, enforce rules

## Service Catalog
- Hierarchical categories/tags
- Full-text search (MySQL FTS)
- Filters: price range, availability, rating, offline distance (ZIP/radius), sort (rating, soonest, price)
- Favorite, compare (up to 3)

## Work Orders & Reviews
- Complete work order → review (within 14 days)
- Review: 1–5 stars, text, tags, up to 5 images (JPG/PNG, 2MB each), one follow-up
- Anti-spam: max 3 reviews/user/day, 1 review/order
- Review reputation: weighted average, 180-day decay, admin pin/collapse

## Internships
- Plans, daily/weekly/monthly reports (rich text, attachments)
- Mentor comments/sign-off
- Weekly due: Monday 12:00 PM, monthly: 5th 5:00 PM, late flag (72h)
- Progress dashboards

## Warehouse Management
- Warehouse–zone–bin hierarchy
- Bin: dimensions (in), load (lbs), temp zone, enable/disable
- Change log: timestamp, user

## Face Data
- Capture/import for badge/access
- Image: ≥320×320, brightness, blur, single frontal face, optional liveness
- Versioned, deduped, deactivatable, audit trail
- Media: local, hashed, encrypted, masked

## Audit/Event Logs
- Append-only, hash-chained, traceable edits

---

# API Endpoints (Sample)

- `POST /auth/login`
- `POST /auth/logout`
- `GET /forums`
- `POST /forums/board`
- `GET /catalog/services`
- `POST /workorders`
- `POST /reviews`
- `POST /internships/plan`
- `POST /internships/report`
- `GET /warehouse`
- `POST /face`
- `GET /audit`

---

# Data Models (Sample)

- User, Role, Forum, Board, Service, WorkOrder, Review, Internship, Warehouse, Bin, FaceRecord, AuditLog

---

# Security
- All sensitive data encrypted at rest
- Field masking in UI
- No third-party dependencies
