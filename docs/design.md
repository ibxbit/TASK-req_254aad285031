# System Design Overview

## Architecture
- **Frontend:** Dioxus (Rust) SPA, desktop & kiosk optimized
- **Backend:** Rocket (Rust), REST APIs, offline/local LAN
- **Database:** MySQL (local, no 3rd-party deps)
- **Storage:** Local for all media/attachments

## Key Modules
- **Authentication & RBAC:** Local auth, role-based permissions, lockout, password policy
- **Forum Management:** Multi-level (zone → board), scoped moderation, announcements, rules
- **Service Catalog:** Hierarchical, full-text search, multi-filter, offline distance, compare, favorite
- **Work Orders & Reviews:** Order lifecycle, review/anti-spam, image upload, follow-up, append-only logs
- **Internship Workflow:** Plan/report submission, mentor sign-off, late flagging, dashboards
- **Warehouse Management:** Hierarchies, bin attributes, change logs
- **Face Data Management:** Capture/import, validation, versioning, deduplication, audit, encryption

## Data Flow
- All user actions logged (audit/event logs)
- All sensitive data encrypted at rest
- All media stored locally, hashed

## Security
- No external network dependencies
- Field masking for sensitive data
- Tamper-proof logs (hash chaining)

## UI/UX
- English only, responsive for desktop/tablet
- Admin/Moderator dashboards
- Requester: catalog, orders, reviews
- Intern/Mentor: internship workflow

---

# Key Design Decisions
- **Offline-first:** All features work without internet
- **Auditability:** All changes traceable, append-only logs
- **Role Scoping:** Permissions strictly enforced per board/service
- **Data Privacy:** Encryption, masking, local storage only
- **Extensibility:** Modular backend, clear API boundaries

---

# Component Diagram

```
[User] <-> [Dioxus Frontend] <-> [Rocket REST API] <-> [MySQL DB]
                                            |
                                    [Local File Storage]
```

---

# Notable Constraints
- No cloud/external APIs
- All binaries and dependencies must be local
- All audit trails must be tamper-evident
