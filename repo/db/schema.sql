-- Field Service Operations Hub - MySQL schema
-- Target: MySQL 8.0+

CREATE DATABASE IF NOT EXISTS field_service_hub
    CHARACTER SET utf8mb4
    COLLATE utf8mb4_unicode_ci;

USE field_service_hub;

-- ============================================================
-- Module: Auth + RBAC
-- ============================================================

CREATE TABLE IF NOT EXISTS users (
    id                   BINARY(16)    NOT NULL,
    username             VARCHAR(64)   NOT NULL,
    password_hash        VARCHAR(255)  NOT NULL,
    role                 VARCHAR(32)   NOT NULL,
    is_active            TINYINT(1)    NOT NULL DEFAULT 1,
    failed_login_count   INT           NOT NULL DEFAULT 0,
    locked_until         DATETIME      NULL,
    -- Encrypted sensitive identifier (AES-GCM, nonce||ciphertext||tag).
    -- NULL when never assigned. Never sent in plaintext to clients.
    sensitive_id_enc     VARBINARY(256) NULL,
    -- Masked display form (e.g. "XXX-XX-1234"). Safe for API/UI output.
    sensitive_id_mask    VARCHAR(64)   NULL,
    created_at           DATETIME      NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at           DATETIME      NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
    PRIMARY KEY (id),
    UNIQUE KEY uq_users_username (username),
    CONSTRAINT chk_users_role CHECK (role IN (
        'administrator','moderator','service_manager','warehouse_manager',
        'mentor','intern','requester'
    ))
) ENGINE=InnoDB;

CREATE TABLE IF NOT EXISTS sessions (
    token_hash   BINARY(32)  NOT NULL,
    user_id      BINARY(16)  NOT NULL,
    created_at   DATETIME    NOT NULL DEFAULT CURRENT_TIMESTAMP,
    expires_at   DATETIME    NOT NULL,
    PRIMARY KEY (token_hash),
    KEY idx_sessions_user (user_id),
    CONSTRAINT fk_sessions_user FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
) ENGINE=InnoDB;

-- ============================================================
-- Module: Teams (minimal infrastructure for forum RESTRICTED visibility)
-- ============================================================

CREATE TABLE IF NOT EXISTS teams (
    id          BINARY(16)   NOT NULL,
    name        VARCHAR(100) NOT NULL,
    created_at  DATETIME     NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (id),
    UNIQUE KEY uq_teams_name (name)
) ENGINE=InnoDB;

CREATE TABLE IF NOT EXISTS user_teams (
    user_id    BINARY(16) NOT NULL,
    team_id    BINARY(16) NOT NULL,
    joined_at  DATETIME   NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (user_id, team_id),
    KEY idx_user_teams_team (team_id),
    CONSTRAINT fk_ut_user FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
    CONSTRAINT fk_ut_team FOREIGN KEY (team_id) REFERENCES teams(id) ON DELETE CASCADE
) ENGINE=InnoDB;

-- ============================================================
-- Module: Forum (Zone -> Board -> Post -> Comment)
-- ============================================================

CREATE TABLE IF NOT EXISTS zones (
    id          BINARY(16)   NOT NULL,
    name        VARCHAR(100) NOT NULL,
    created_at  DATETIME     NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at  DATETIME     NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
    PRIMARY KEY (id),
    UNIQUE KEY uq_zones_name (name)
) ENGINE=InnoDB;

CREATE TABLE IF NOT EXISTS boards (
    id               BINARY(16)   NOT NULL,
    zone_id          BINARY(16)   NOT NULL,
    name             VARCHAR(100) NOT NULL,
    visibility_type  VARCHAR(20)  NOT NULL,
    created_by       BINARY(16)   NOT NULL,
    created_at       DATETIME     NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at       DATETIME     NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
    PRIMARY KEY (id),
    UNIQUE KEY uq_boards_zone_name (zone_id, name),
    KEY idx_boards_zone (zone_id),
    CONSTRAINT fk_boards_zone FOREIGN KEY (zone_id) REFERENCES zones(id) ON DELETE CASCADE,
    CONSTRAINT fk_boards_creator FOREIGN KEY (created_by) REFERENCES users(id),
    CONSTRAINT chk_boards_visibility CHECK (visibility_type IN ('public','restricted'))
) ENGINE=InnoDB;

CREATE TABLE IF NOT EXISTS board_allowed_teams (
    board_id BINARY(16) NOT NULL,
    team_id  BINARY(16) NOT NULL,
    PRIMARY KEY (board_id, team_id),
    KEY idx_bat_team (team_id),
    CONSTRAINT fk_bat_board FOREIGN KEY (board_id) REFERENCES boards(id) ON DELETE CASCADE,
    CONSTRAINT fk_bat_team FOREIGN KEY (team_id) REFERENCES teams(id) ON DELETE CASCADE
) ENGINE=InnoDB;

CREATE TABLE IF NOT EXISTS board_rules (
    id         BINARY(16) NOT NULL,
    board_id   BINARY(16) NOT NULL,
    content    TEXT       NOT NULL,
    created_at DATETIME   NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (id),
    KEY idx_board_rules_board (board_id),
    CONSTRAINT fk_br_board FOREIGN KEY (board_id) REFERENCES boards(id) ON DELETE CASCADE
) ENGINE=InnoDB;

CREATE TABLE IF NOT EXISTS board_moderators (
    id          BINARY(16) NOT NULL,
    board_id    BINARY(16) NOT NULL,
    user_id     BINARY(16) NOT NULL,
    assigned_at DATETIME   NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (id),
    UNIQUE KEY uq_bm_board_user (board_id, user_id),
    KEY idx_bm_user (user_id),
    CONSTRAINT fk_bm_board FOREIGN KEY (board_id) REFERENCES boards(id) ON DELETE CASCADE,
    CONSTRAINT fk_bm_user FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
) ENGINE=InnoDB;

CREATE TABLE IF NOT EXISTS posts (
    id         BINARY(16)   NOT NULL,
    board_id   BINARY(16)   NOT NULL,
    author_id  BINARY(16)   NOT NULL,
    title      VARCHAR(255) NOT NULL,
    content    TEXT         NOT NULL,
    is_pinned  TINYINT(1)   NOT NULL DEFAULT 0,
    created_at DATETIME     NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (id),
    KEY idx_posts_board_pinned (board_id, is_pinned, created_at),
    CONSTRAINT fk_posts_board FOREIGN KEY (board_id) REFERENCES boards(id) ON DELETE CASCADE,
    CONSTRAINT fk_posts_author FOREIGN KEY (author_id) REFERENCES users(id)
) ENGINE=InnoDB;

CREATE TABLE IF NOT EXISTS comments (
    id         BINARY(16) NOT NULL,
    post_id    BINARY(16) NOT NULL,
    author_id  BINARY(16) NOT NULL,
    content    TEXT       NOT NULL,
    created_at DATETIME   NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (id),
    KEY idx_comments_post (post_id, created_at),
    CONSTRAINT fk_comments_post FOREIGN KEY (post_id) REFERENCES posts(id) ON DELETE CASCADE,
    CONSTRAINT fk_comments_author FOREIGN KEY (author_id) REFERENCES users(id)
) ENGINE=InnoDB;

-- ============================================================
-- Module: Service Catalog (services, categories, tags, availability, favorites)
-- ============================================================

CREATE TABLE IF NOT EXISTS services (
    id                     BINARY(16)     NOT NULL,
    name                   VARCHAR(255)   NOT NULL,
    description            TEXT           NOT NULL,
    price                  DOUBLE         NOT NULL DEFAULT 0.0,
    rating                 DOUBLE         NOT NULL DEFAULT 0.0,
    coverage_radius_miles  INT            NOT NULL DEFAULT 0,
    zip_code               VARCHAR(10)    NOT NULL,
    created_at             DATETIME       NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at             DATETIME       NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
    PRIMARY KEY (id),
    KEY idx_services_price (price),
    KEY idx_services_rating (rating),
    KEY idx_services_zip (zip_code),
    FULLTEXT KEY ft_services_name_desc (name, description),
    CONSTRAINT chk_services_price  CHECK (price >= 0),
    CONSTRAINT chk_services_rating CHECK (rating >= 0 AND rating <= 5),
    CONSTRAINT chk_services_radius CHECK (coverage_radius_miles >= 0)
) ENGINE=InnoDB;

CREATE TABLE IF NOT EXISTS categories (
    id         BINARY(16)   NOT NULL,
    parent_id  BINARY(16)   NULL,
    name       VARCHAR(100) NOT NULL,
    PRIMARY KEY (id),
    KEY idx_categories_parent (parent_id),
    CONSTRAINT fk_categories_parent FOREIGN KEY (parent_id) REFERENCES categories(id) ON DELETE SET NULL
) ENGINE=InnoDB;

CREATE TABLE IF NOT EXISTS service_categories (
    service_id  BINARY(16) NOT NULL,
    category_id BINARY(16) NOT NULL,
    PRIMARY KEY (service_id, category_id),
    KEY idx_sc_category (category_id),
    CONSTRAINT fk_sc_service  FOREIGN KEY (service_id)  REFERENCES services(id)   ON DELETE CASCADE,
    CONSTRAINT fk_sc_category FOREIGN KEY (category_id) REFERENCES categories(id) ON DELETE CASCADE
) ENGINE=InnoDB;

CREATE TABLE IF NOT EXISTS tags (
    id   BINARY(16)  NOT NULL,
    name VARCHAR(50) NOT NULL,
    PRIMARY KEY (id),
    UNIQUE KEY uq_tags_name (name)
) ENGINE=InnoDB;

CREATE TABLE IF NOT EXISTS service_tags (
    service_id BINARY(16) NOT NULL,
    tag_id     BINARY(16) NOT NULL,
    PRIMARY KEY (service_id, tag_id),
    KEY idx_st_tag (tag_id),
    CONSTRAINT fk_st_service FOREIGN KEY (service_id) REFERENCES services(id) ON DELETE CASCADE,
    CONSTRAINT fk_st_tag     FOREIGN KEY (tag_id)     REFERENCES tags(id)     ON DELETE CASCADE
) ENGINE=InnoDB;

CREATE TABLE IF NOT EXISTS availability (
    id         BINARY(16) NOT NULL,
    service_id BINARY(16) NOT NULL,
    start_time DATETIME   NOT NULL,
    end_time   DATETIME   NOT NULL,
    PRIMARY KEY (id),
    KEY idx_availability_service_start (service_id, start_time),
    CONSTRAINT fk_availability_service FOREIGN KEY (service_id) REFERENCES services(id) ON DELETE CASCADE,
    CONSTRAINT chk_availability_window CHECK (end_time > start_time)
) ENGINE=InnoDB;

CREATE TABLE IF NOT EXISTS favorites (
    user_id      BINARY(16) NOT NULL,
    service_id   BINARY(16) NOT NULL,
    favorited_at DATETIME   NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (user_id, service_id),
    KEY idx_favorites_service (service_id),
    CONSTRAINT fk_fav_user    FOREIGN KEY (user_id)    REFERENCES users(id)    ON DELETE CASCADE,
    CONSTRAINT fk_fav_service FOREIGN KEY (service_id) REFERENCES services(id) ON DELETE CASCADE
) ENGINE=InnoDB;

-- ZIP -> coordinates lookup table. Populated locally (offline); Haversine
-- distance filtering in /services/search joins through this table.
CREATE TABLE IF NOT EXISTS zip_coordinates (
    zip_code  VARCHAR(10) NOT NULL,
    latitude  DOUBLE      NOT NULL,
    longitude DOUBLE      NOT NULL,
    PRIMARY KEY (zip_code),
    CONSTRAINT chk_zc_lat CHECK (latitude  BETWEEN -90  AND 90),
    CONSTRAINT chk_zc_lon CHECK (longitude BETWEEN -180 AND 180)
) ENGINE=InnoDB;

-- ============================================================
-- Module: Work Orders & Reviews
-- ============================================================

CREATE TABLE IF NOT EXISTS work_orders (
    id            BINARY(16)  NOT NULL,
    requester_id  BINARY(16)  NOT NULL,
    service_id    BINARY(16)  NOT NULL,
    status        VARCHAR(20) NOT NULL DEFAULT 'pending',
    completed_at  DATETIME    NULL,
    created_at    DATETIME    NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at    DATETIME    NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
    PRIMARY KEY (id),
    KEY idx_wo_requester (requester_id),
    KEY idx_wo_service (service_id),
    KEY idx_wo_status (status),
    CONSTRAINT fk_wo_req FOREIGN KEY (requester_id) REFERENCES users(id),
    CONSTRAINT fk_wo_svc FOREIGN KEY (service_id)  REFERENCES services(id),
    CONSTRAINT chk_wo_status CHECK (status IN ('pending','in_progress','completed','cancelled'))
) ENGINE=InnoDB;

-- Review lifecycle:
--   kind='initial'   -> exactly one per work order (enforced by partial
--                       UNIQUE below via a generated column).
--   kind='follow_up' -> at most one per work order, links back via
--                       parent_review_id. Allowed only after an initial
--                       review exists for the same order.
CREATE TABLE IF NOT EXISTS reviews (
    id                BINARY(16)         NOT NULL,
    work_order_id     BINARY(16)         NOT NULL,
    user_id           BINARY(16)         NOT NULL,
    rating            TINYINT UNSIGNED   NOT NULL,
    text              TEXT               NOT NULL,
    is_pinned         TINYINT(1)         NOT NULL DEFAULT 0,
    is_collapsed      TINYINT(1)         NOT NULL DEFAULT 0,
    kind              VARCHAR(16)        NOT NULL DEFAULT 'initial',
    parent_review_id  BINARY(16)         NULL,
    created_at        DATETIME           NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at        DATETIME           NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
    -- Generated columns allow two partial-style unique constraints while
    -- still representing "one initial per order" and "one follow-up per
    -- order" in pure MySQL 8.
    wo_initial_key    BINARY(16)         GENERATED ALWAYS AS
        (CASE WHEN kind = 'initial'   THEN work_order_id END) VIRTUAL,
    wo_followup_key   BINARY(16)         GENERATED ALWAYS AS
        (CASE WHEN kind = 'follow_up' THEN work_order_id END) VIRTUAL,
    PRIMARY KEY (id),
    UNIQUE KEY uq_review_wo_initial   (wo_initial_key),
    UNIQUE KEY uq_review_wo_followup  (wo_followup_key),
    KEY idx_reviews_user_created (user_id, created_at),
    KEY idx_reviews_wo_kind      (work_order_id, kind),
    CONSTRAINT fk_review_wo     FOREIGN KEY (work_order_id)    REFERENCES work_orders(id) ON DELETE CASCADE,
    CONSTRAINT fk_review_user   FOREIGN KEY (user_id)          REFERENCES users(id),
    CONSTRAINT fk_review_parent FOREIGN KEY (parent_review_id) REFERENCES reviews(id)     ON DELETE SET NULL,
    CONSTRAINT chk_review_rating CHECK (rating BETWEEN 1 AND 5),
    CONSTRAINT chk_review_kind   CHECK (kind IN ('initial','follow_up'))
) ENGINE=InnoDB;

CREATE TABLE IF NOT EXISTS review_images (
    id           BINARY(16)   NOT NULL,
    review_id    BINARY(16)   NOT NULL,
    file_path    VARCHAR(500) NOT NULL,
    size         INT          NOT NULL,
    content_type VARCHAR(50)  NOT NULL,
    -- SHA-256 content hash (hex) for integrity verification.
    content_hash VARCHAR(64)  NOT NULL,
    created_at   DATETIME     NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (id),
    KEY idx_ri_review (review_id),
    KEY idx_ri_hash (content_hash),
    CONSTRAINT fk_ri_review FOREIGN KEY (review_id) REFERENCES reviews(id) ON DELETE CASCADE,
    CONSTRAINT chk_ri_size CHECK (size > 0 AND size <= 2097152),
    CONSTRAINT chk_ri_type CHECK (content_type IN ('image/jpeg','image/png'))
) ENGINE=InnoDB;

CREATE TABLE IF NOT EXISTS review_tags (
    id   BINARY(16)  NOT NULL,
    name VARCHAR(50) NOT NULL,
    PRIMARY KEY (id),
    UNIQUE KEY uq_review_tags_name (name)
) ENGINE=InnoDB;

CREATE TABLE IF NOT EXISTS review_tag_map (
    review_id BINARY(16) NOT NULL,
    tag_id    BINARY(16) NOT NULL,
    PRIMARY KEY (review_id, tag_id),
    KEY idx_rtm_tag (tag_id),
    CONSTRAINT fk_rtm_review FOREIGN KEY (review_id) REFERENCES reviews(id)      ON DELETE CASCADE,
    CONSTRAINT fk_rtm_tag    FOREIGN KEY (tag_id)    REFERENCES review_tags(id) ON DELETE CASCADE
) ENGINE=InnoDB;

-- ============================================================
-- Module: Internship Management
-- ============================================================

CREATE TABLE IF NOT EXISTS internship_plans (
    id         BINARY(16) NOT NULL,
    intern_id  BINARY(16) NOT NULL,
    content    TEXT       NOT NULL,
    created_at DATETIME   NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (id),
    KEY idx_ip_intern (intern_id),
    CONSTRAINT fk_ip_intern FOREIGN KEY (intern_id) REFERENCES users(id)
) ENGINE=InnoDB;

CREATE TABLE IF NOT EXISTS reports (
    id           BINARY(16)  NOT NULL,
    intern_id    BINARY(16)  NOT NULL,
    report_type  VARCHAR(20) NOT NULL,
    content      TEXT        NOT NULL,
    status       VARCHAR(20) NOT NULL,
    submitted_at DATETIME    NOT NULL DEFAULT CURRENT_TIMESTAMP,
    due_at       DATETIME    NOT NULL,
    is_late      TINYINT(1)  NOT NULL DEFAULT 0,
    PRIMARY KEY (id),
    KEY idx_reports_intern (intern_id),
    KEY idx_reports_intern_submitted (intern_id, submitted_at),
    KEY idx_reports_status_late (is_late, status),
    CONSTRAINT fk_reports_intern FOREIGN KEY (intern_id) REFERENCES users(id),
    CONSTRAINT chk_reports_type   CHECK (report_type IN ('daily','weekly','monthly')),
    CONSTRAINT chk_reports_status CHECK (status IN ('on_time','late'))
) ENGINE=InnoDB;

CREATE TABLE IF NOT EXISTS report_attachments (
    id           BINARY(16)   NOT NULL,
    report_id    BINARY(16)   NOT NULL,
    file_path    VARCHAR(500) NOT NULL,
    -- SHA-256 content hash (hex) for integrity verification.
    content_hash VARCHAR(64)  NOT NULL,
    size_bytes   BIGINT       NOT NULL,
    created_at   DATETIME     NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (id),
    KEY idx_ra_report (report_id),
    KEY idx_ra_hash (content_hash),
    CONSTRAINT fk_ra_report FOREIGN KEY (report_id) REFERENCES reports(id) ON DELETE CASCADE,
    CONSTRAINT chk_ra_size  CHECK (size_bytes > 0)
) ENGINE=InnoDB;

CREATE TABLE IF NOT EXISTS mentor_comments (
    id         BINARY(16) NOT NULL,
    report_id  BINARY(16) NOT NULL,
    mentor_id  BINARY(16) NOT NULL,
    content    TEXT       NOT NULL,
    created_at DATETIME   NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (id),
    KEY idx_mc_report (report_id),
    CONSTRAINT fk_mc_report FOREIGN KEY (report_id) REFERENCES reports(id) ON DELETE CASCADE,
    CONSTRAINT fk_mc_mentor FOREIGN KEY (mentor_id) REFERENCES users(id)
) ENGINE=InnoDB;

CREATE TABLE IF NOT EXISTS report_approvals (
    id          BINARY(16) NOT NULL,
    report_id   BINARY(16) NOT NULL,
    mentor_id   BINARY(16) NOT NULL,
    approved_at DATETIME   NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (id),
    UNIQUE KEY uq_report_approvals_report (report_id),
    KEY idx_report_approvals_mentor (mentor_id),
    CONSTRAINT fk_app_report FOREIGN KEY (report_id) REFERENCES reports(id) ON DELETE CASCADE,
    CONSTRAINT fk_app_mentor FOREIGN KEY (mentor_id) REFERENCES users(id)
) ENGINE=InnoDB;

-- ============================================================
-- Module: Warehouse Management
-- ============================================================

CREATE TABLE IF NOT EXISTS warehouses (
    id         BINARY(16)   NOT NULL,
    name       VARCHAR(100) NOT NULL,
    created_at DATETIME     NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME     NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
    PRIMARY KEY (id),
    UNIQUE KEY uq_warehouses_name (name)
) ENGINE=InnoDB;

CREATE TABLE IF NOT EXISTS warehouse_zones (
    id           BINARY(16)   NOT NULL,
    warehouse_id BINARY(16)   NOT NULL,
    name         VARCHAR(100) NOT NULL,
    created_at   DATETIME     NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at   DATETIME     NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
    PRIMARY KEY (id),
    UNIQUE KEY uq_wz_warehouse_name (warehouse_id, name),
    KEY idx_wz_warehouse (warehouse_id),
    CONSTRAINT fk_wz_warehouse FOREIGN KEY (warehouse_id) REFERENCES warehouses(id) ON DELETE CASCADE
) ENGINE=InnoDB;

CREATE TABLE IF NOT EXISTS bins (
    id            BINARY(16)   NOT NULL,
    zone_id       BINARY(16)   NOT NULL,
    name          VARCHAR(100) NOT NULL,
    width_in      DOUBLE       NOT NULL,
    height_in     DOUBLE       NOT NULL,
    depth_in      DOUBLE       NOT NULL,
    max_load_lbs  DOUBLE       NOT NULL,
    temp_zone     VARCHAR(50)  NOT NULL,
    is_enabled    TINYINT(1)   NOT NULL DEFAULT 1,
    created_at    DATETIME     NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at    DATETIME     NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
    PRIMARY KEY (id),
    UNIQUE KEY uq_bins_zone_name (zone_id, name),
    KEY idx_bins_zone (zone_id),
    CONSTRAINT fk_bins_zone FOREIGN KEY (zone_id) REFERENCES warehouse_zones(id) ON DELETE CASCADE,
    CONSTRAINT chk_bins_dim  CHECK (width_in > 0 AND height_in > 0 AND depth_in > 0),
    CONSTRAINT chk_bins_load CHECK (max_load_lbs >= 0)
) ENGINE=InnoDB;

-- Traceability for structural changes at the warehouse level (rename,
-- create, delete). Captures actor + timestamp + before/after values.
CREATE TABLE IF NOT EXISTS warehouse_change_log (
    id           BINARY(16)  NOT NULL,
    warehouse_id BINARY(16)  NOT NULL,
    changed_by   BINARY(16)  NOT NULL,
    change_type  VARCHAR(50) NOT NULL,
    old_value    TEXT        NULL,
    new_value    TEXT        NULL,
    created_at   DATETIME    NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (id),
    KEY idx_wcl_wh_created (warehouse_id, created_at),
    CONSTRAINT fk_wcl_user FOREIGN KEY (changed_by) REFERENCES users(id)
) ENGINE=InnoDB;

-- Traceability for structural changes at the warehouse-zone level.
CREATE TABLE IF NOT EXISTS warehouse_zone_change_log (
    id           BINARY(16)  NOT NULL,
    zone_id      BINARY(16)  NOT NULL,
    changed_by   BINARY(16)  NOT NULL,
    change_type  VARCHAR(50) NOT NULL,
    old_value    TEXT        NULL,
    new_value    TEXT        NULL,
    created_at   DATETIME    NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (id),
    KEY idx_wzcl_zone_created (zone_id, created_at),
    CONSTRAINT fk_wzcl_user FOREIGN KEY (changed_by) REFERENCES users(id)
) ENGINE=InnoDB;

CREATE TABLE IF NOT EXISTS bin_change_log (
    id          BINARY(16)  NOT NULL,
    bin_id      BINARY(16)  NOT NULL,
    changed_by  BINARY(16)  NOT NULL,
    change_type VARCHAR(50) NOT NULL,
    old_value   TEXT        NULL,
    new_value   TEXT        NULL,
    created_at  DATETIME    NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (id),
    KEY idx_bcl_bin_created (bin_id, created_at),
    CONSTRAINT fk_bcl_bin  FOREIGN KEY (bin_id)     REFERENCES bins(id)  ON DELETE CASCADE,
    CONSTRAINT fk_bcl_user FOREIGN KEY (changed_by) REFERENCES users(id)
) ENGINE=InnoDB;

-- ============================================================
-- Module: Face Data Management
-- Records and images are immutable; deactivation flag only.
-- ============================================================

CREATE TABLE IF NOT EXISTS face_records (
    id         BINARY(16) NOT NULL,
    user_id    BINARY(16) NOT NULL,
    version    INT        NOT NULL,
    is_active  TINYINT(1) NOT NULL DEFAULT 1,
    created_at DATETIME   NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (id),
    UNIQUE KEY uq_face_records_user_version (user_id, version),
    KEY idx_face_records_user_active (user_id, is_active),
    CONSTRAINT fk_face_records_user FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
    CONSTRAINT chk_face_records_version CHECK (version > 0)
) ENGINE=InnoDB;

CREATE TABLE IF NOT EXISTS face_images (
    id                BINARY(16)   NOT NULL,
    face_record_id    BINARY(16)   NOT NULL,
    file_path         VARCHAR(500) NOT NULL,
    hash              VARCHAR(64)  NOT NULL,        -- SHA-256 hex (content integrity)
    perceptual_hash   VARCHAR(16)  NOT NULL,        -- 64-bit aHash hex (dedup)
    resolution        VARCHAR(20)  NOT NULL,        -- e.g. "1280x720"
    brightness_score  DOUBLE       NOT NULL,
    blur_score        DOUBLE       NOT NULL,
    created_at        DATETIME     NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (id),
    KEY idx_face_images_record (face_record_id),
    KEY idx_face_images_hash (hash),
    KEY idx_face_images_phash (perceptual_hash),
    CONSTRAINT fk_face_images_record FOREIGN KEY (face_record_id) REFERENCES face_records(id) ON DELETE CASCADE
) ENGINE=InnoDB;

-- Optional liveness challenge recording. One row per submitted challenge.
-- action records the challenge type (e.g. 'turn_left','turn_right','blink')
-- and `passed` captures the local operator / automated result. Persistent
-- trace tied to the face_record so review can reconstruct the event chain.
CREATE TABLE IF NOT EXISTS face_liveness_challenges (
    id              BINARY(16)   NOT NULL,
    face_record_id  BINARY(16)   NOT NULL,
    challenge       VARCHAR(40)  NOT NULL,
    passed          TINYINT(1)   NOT NULL,
    notes           VARCHAR(255) NULL,
    performed_by    BINARY(16)   NOT NULL,
    created_at      DATETIME     NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (id),
    KEY idx_flc_record (face_record_id, created_at),
    CONSTRAINT fk_flc_record FOREIGN KEY (face_record_id) REFERENCES face_records(id) ON DELETE CASCADE,
    CONSTRAINT fk_flc_user   FOREIGN KEY (performed_by)   REFERENCES users(id),
    CONSTRAINT chk_flc_challenge CHECK (challenge IN (
        'turn_left','turn_right','blink','nod','smile','tilt'
    ))
) ENGINE=InnoDB;

CREATE TABLE IF NOT EXISTS face_audits (
    id              BINARY(16)  NOT NULL,
    face_record_id  BINARY(16)  NOT NULL,
    action          VARCHAR(20) NOT NULL,
    performed_by    BINARY(16)  NOT NULL,
    created_at      DATETIME    NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (id),
    KEY idx_face_audits_record (face_record_id),
    CONSTRAINT fk_fa_record FOREIGN KEY (face_record_id) REFERENCES face_records(id) ON DELETE CASCADE,
    CONSTRAINT fk_fa_user   FOREIGN KEY (performed_by)   REFERENCES users(id),
    CONSTRAINT chk_face_audits_action CHECK (action IN (
        'created','validated','rejected','deactivated'
    ))
) ENGINE=InnoDB;

-- ============================================================
-- Module: Audit Log (append-only, tamper-evident hash chain)
-- `sequence` gives stable ordering even across same-second inserts.
-- `prev_hash` links each event to its predecessor for the same
-- (entity_type, entity_id) pair; `hash = SHA256(payload || prev_hash)`.
-- ============================================================

CREATE TABLE IF NOT EXISTS event_log (
    id          BINARY(16)  NOT NULL,
    sequence    BIGINT      NOT NULL AUTO_INCREMENT,
    entity_type VARCHAR(50) NOT NULL,
    entity_id   BINARY(16)  NOT NULL,
    action      VARCHAR(50) NOT NULL,
    payload     LONGTEXT    NOT NULL,
    prev_hash   VARCHAR(64) NOT NULL,
    hash        VARCHAR(64) NOT NULL,
    created_at  DATETIME    NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (id),
    KEY idx_event_log_sequence (sequence),
    KEY idx_event_log_entity   (entity_type, entity_id, sequence),
    KEY idx_event_log_hash     (hash),
    KEY idx_event_log_created  (created_at)
) ENGINE=InnoDB;

-- ============================================================
-- Idempotent migrations for existing deployments.
--
-- MySQL 8 has no `ALTER TABLE ... ADD COLUMN IF NOT EXISTS`, so we wrap
-- each structural change in a stored procedure that checks
-- INFORMATION_SCHEMA first. Running this block repeatedly is safe.
-- ============================================================

DROP PROCEDURE IF EXISTS apply_review_migrations;
DELIMITER $$
CREATE PROCEDURE apply_review_migrations()
BEGIN
    -- reviews.kind
    IF NOT EXISTS (SELECT 1 FROM INFORMATION_SCHEMA.COLUMNS
                    WHERE TABLE_SCHEMA = DATABASE()
                      AND TABLE_NAME   = 'reviews'
                      AND COLUMN_NAME  = 'kind') THEN
        ALTER TABLE reviews
            ADD COLUMN kind VARCHAR(16) NOT NULL DEFAULT 'initial';
    END IF;

    -- reviews.parent_review_id
    IF NOT EXISTS (SELECT 1 FROM INFORMATION_SCHEMA.COLUMNS
                    WHERE TABLE_SCHEMA = DATABASE()
                      AND TABLE_NAME   = 'reviews'
                      AND COLUMN_NAME  = 'parent_review_id') THEN
        ALTER TABLE reviews
            ADD COLUMN parent_review_id BINARY(16) NULL,
            ADD CONSTRAINT fk_review_parent
                FOREIGN KEY (parent_review_id) REFERENCES reviews(id) ON DELETE SET NULL;
    END IF;

    -- Replace old UNIQUE(work_order_id) with partial-style uniques via
    -- generated columns. Only drop the legacy index if present.
    IF EXISTS (SELECT 1 FROM INFORMATION_SCHEMA.STATISTICS
                WHERE TABLE_SCHEMA = DATABASE()
                  AND TABLE_NAME   = 'reviews'
                  AND INDEX_NAME   = 'uq_review_wo') THEN
        ALTER TABLE reviews DROP INDEX uq_review_wo;
    END IF;

    IF NOT EXISTS (SELECT 1 FROM INFORMATION_SCHEMA.COLUMNS
                    WHERE TABLE_SCHEMA = DATABASE()
                      AND TABLE_NAME   = 'reviews'
                      AND COLUMN_NAME  = 'wo_initial_key') THEN
        ALTER TABLE reviews
            ADD COLUMN wo_initial_key  BINARY(16) GENERATED ALWAYS AS
                (CASE WHEN kind = 'initial'   THEN work_order_id END) VIRTUAL,
            ADD COLUMN wo_followup_key BINARY(16) GENERATED ALWAYS AS
                (CASE WHEN kind = 'follow_up' THEN work_order_id END) VIRTUAL,
            ADD UNIQUE KEY uq_review_wo_initial  (wo_initial_key),
            ADD UNIQUE KEY uq_review_wo_followup (wo_followup_key);
    END IF;
END$$
DELIMITER ;

CALL apply_review_migrations();
DROP PROCEDURE apply_review_migrations;

-- Join table: requester-selectable tags supplied at review creation time.
-- Existing admin-only `review_tag_map` (back-fill tagging) remains in
-- place; the selectable-at-create flow inserts into the same map.
