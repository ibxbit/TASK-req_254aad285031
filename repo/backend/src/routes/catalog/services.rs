use rocket::form::FromForm;
use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::State;
use shared::{
    AvailabilityWindow, Category, CreateServiceRequest, Role, Service, ServiceComparison, SortMode,
    Tag, UpdateServiceRequest,
};
use sqlx::{MySql, MySqlPool, QueryBuilder};
use uuid::Uuid;

use crate::auth::guard::AuthUser;

const MANAGEMENT_ROLES: [Role; 2] = [Role::Administrator, Role::ServiceManager];

// ---------- Create / update / get ----------

#[post("/services", format = "json", data = "<req>")]
pub async fn create(
    pool: &State<MySqlPool>,
    user: AuthUser,
    req: Json<CreateServiceRequest>,
) -> Result<Json<Service>, Status> {
    user.require_any(&MANAGEMENT_ROLES)?;
    let id = Uuid::new_v4();
    let rating = req.rating.unwrap_or(0.0);
    sqlx::query(
        "INSERT INTO services \
         (id, name, description, price, rating, coverage_radius_miles, zip_code) \
         VALUES (?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&id.as_bytes()[..])
    .bind(&req.name)
    .bind(&req.description)
    .bind(req.price)
    .bind(rating)
    .bind(req.coverage_radius_miles)
    .bind(&req.zip_code)
    .execute(pool.inner())
    .await
    .map_err(|_| Status::BadRequest)?;

    Ok(Json(Service {
        id: id.to_string(),
        name: req.name.clone(),
        description: req.description.clone(),
        price: req.price,
        rating,
        coverage_radius_miles: req.coverage_radius_miles,
        zip_code: req.zip_code.clone(),
    }))
}

#[patch("/services/<id>", format = "json", data = "<req>")]
pub async fn update(
    pool: &State<MySqlPool>,
    user: AuthUser,
    id: &str,
    req: Json<UpdateServiceRequest>,
) -> Result<Status, Status> {
    user.require_any(&MANAGEMENT_ROLES)?;
    let sid = Uuid::parse_str(id).map_err(|_| Status::BadRequest)?;

    let mut qb: QueryBuilder<MySql> = QueryBuilder::new("UPDATE services SET ");
    let mut first = true;
    let push_set = |qb: &mut QueryBuilder<MySql>, col: &str, first: &mut bool| {
        if !*first {
            qb.push(", ");
        }
        *first = false;
        qb.push(col).push(" = ");
    };

    if let Some(v) = &req.name {
        push_set(&mut qb, "name", &mut first);
        qb.push_bind(v);
    }
    if let Some(v) = &req.description {
        push_set(&mut qb, "description", &mut first);
        qb.push_bind(v);
    }
    if let Some(v) = req.price {
        push_set(&mut qb, "price", &mut first);
        qb.push_bind(v);
    }
    if let Some(v) = req.rating {
        push_set(&mut qb, "rating", &mut first);
        qb.push_bind(v);
    }
    if let Some(v) = req.coverage_radius_miles {
        push_set(&mut qb, "coverage_radius_miles", &mut first);
        qb.push_bind(v);
    }
    if let Some(v) = &req.zip_code {
        push_set(&mut qb, "zip_code", &mut first);
        qb.push_bind(v);
    }
    if first {
        return Ok(Status::NoContent);
    }
    qb.push(" WHERE id = ").push_bind(sid.as_bytes().to_vec());
    let result = qb
        .build()
        .execute(pool.inner())
        .await
        .map_err(|_| Status::InternalServerError)?;
    if result.rows_affected() == 0 {
        return Err(Status::NotFound);
    }
    Ok(Status::NoContent)
}

#[get("/services/<id>")]
pub async fn get(
    pool: &State<MySqlPool>,
    _user: AuthUser,
    id: &str,
) -> Result<Json<Service>, Status> {
    let sid = Uuid::parse_str(id).map_err(|_| Status::BadRequest)?;
    let row: Option<(Vec<u8>, String, String, f64, f64, i32, String)> = sqlx::query_as(
        "SELECT id, name, description, price, rating, coverage_radius_miles, zip_code \
         FROM services WHERE id = ?",
    )
    .bind(&sid.as_bytes()[..])
    .fetch_optional(pool.inner())
    .await
    .map_err(|_| Status::InternalServerError)?;
    let Some((sid_b, name, desc, price, rating, radius, zip)) = row else {
        return Err(Status::NotFound);
    };
    Ok(Json(Service {
        id: Uuid::from_slice(&sid_b)
            .map_err(|_| Status::InternalServerError)?
            .to_string(),
        name,
        description: desc,
        price,
        rating,
        coverage_radius_miles: radius,
        zip_code: zip,
    }))
}

// ---------- Search ----------

#[derive(FromForm)]
pub struct SearchQuery {
    pub q: Option<String>,
    pub min_price: Option<f64>,
    pub max_price: Option<f64>,
    pub min_rating: Option<f64>,
    pub available_from: Option<String>,
    pub available_to: Option<String>,
    pub user_zip: Option<String>,
    pub sort: Option<String>,
    pub limit: Option<u32>,
    pub offset: Option<u32>,
    // Category filter: comma-separated list of category UUIDs.
    // Match semantics: service must be tagged with ALL listed categories.
    pub categories: Option<String>,
    // Tag filter: comma-separated list of tag UUIDs.
    // Match semantics: service must carry ANY listed tag.
    pub tags: Option<String>,
}

fn parse_uuid_csv(s: &str) -> Result<Vec<Uuid>, Status> {
    s.split(',')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|s| Uuid::parse_str(s).map_err(|_| Status::BadRequest))
        .collect()
}

fn parse_dt(s: &str) -> Result<chrono::NaiveDateTime, Status> {
    chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S")
        .or_else(|_| chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S"))
        .map_err(|_| Status::BadRequest)
}

#[get("/services/search?<query..>")]
pub async fn search(
    pool: &State<MySqlPool>,
    _user: AuthUser,
    query: SearchQuery,
) -> Result<Json<Vec<Service>>, Status> {
    // Resolve user ZIP -> coordinates up front (offline lookup).
    // Missing entry is a hard 400 so callers don't silently get empty results.
    let user_coords: Option<(f64, f64)> = match &query.user_zip {
        Some(z) if !z.trim().is_empty() => {
            let row: Option<(f64, f64)> = sqlx::query_as(
                "SELECT latitude, longitude FROM zip_coordinates WHERE zip_code = ?",
            )
            .bind(z.trim())
            .fetch_optional(pool.inner())
            .await
            .map_err(|_| Status::InternalServerError)?;
            match row {
                Some(c) => Some(c),
                None => return Err(Status::BadRequest),
            }
        }
        _ => None,
    };

    let sort = query
        .sort
        .as_deref()
        .and_then(SortMode::from_str)
        .unwrap_or(SortMode::BestRated);

    let mut qb: QueryBuilder<MySql> = QueryBuilder::new(
        "SELECT s.id, s.name, s.description, s.price, s.rating, \
                s.coverage_radius_miles, s.zip_code \
         FROM services s",
    );

    // Derived table giving each service its earliest availability.start_time.
    // Joined only when needed for sorting -> no cost on other sorts.
    if sort == SortMode::SoonestAvailable {
        qb.push(
            " LEFT JOIN ( \
                SELECT service_id, MIN(start_time) AS soonest \
                FROM availability GROUP BY service_id \
              ) sa ON sa.service_id = s.id",
        );
    }

    // Services without coordinate data never match a distance-filtered search.
    if user_coords.is_some() {
        qb.push(" INNER JOIN zip_coordinates szc ON szc.zip_code = s.zip_code");
    }

    qb.push(" WHERE 1=1");

    if let Some(v) = query.min_price {
        qb.push(" AND s.price >= ").push_bind(v);
    }
    if let Some(v) = query.max_price {
        qb.push(" AND s.price <= ").push_bind(v);
    }
    if let Some(v) = query.min_rating {
        qb.push(" AND s.rating >= ").push_bind(v);
    }
    if let Some(q) = &query.q {
        if !q.trim().is_empty() {
            qb.push(" AND MATCH(s.name, s.description) AGAINST (")
                .push_bind(q.clone())
                .push(" IN NATURAL LANGUAGE MODE)");
        }
    }
    let from_dt = match &query.available_from {
        Some(s) => Some(parse_dt(s)?),
        None => None,
    };
    let to_dt = match &query.available_to {
        Some(s) => Some(parse_dt(s)?),
        None => None,
    };
    if let (Some(from_dt), Some(to_dt)) = (from_dt, to_dt) {
        qb.push(
            " AND EXISTS (SELECT 1 FROM availability a \
               WHERE a.service_id = s.id AND a.start_time < ",
        )
        .push_bind(to_dt)
        .push(" AND a.end_time > ")
        .push_bind(from_dt)
        .push(")");
    }

    // Category filter (AND): every listed category must be attached.
    if let Some(cats) = &query.categories {
        let ids = parse_uuid_csv(cats)?;
        if !ids.is_empty() {
            qb.push(
                " AND (SELECT COUNT(DISTINCT sc.category_id) \
                   FROM service_categories sc \
                   WHERE sc.service_id = s.id AND sc.category_id IN (",
            );
            let mut sep = false;
            for id in &ids {
                if sep {
                    qb.push(", ");
                }
                qb.push_bind(id.as_bytes().to_vec());
                sep = true;
            }
            qb.push(")) = ").push_bind(ids.len() as i64);
        }
    }

    // Tag filter (OR): any listed tag qualifies.
    if let Some(tgs) = &query.tags {
        let ids = parse_uuid_csv(tgs)?;
        if !ids.is_empty() {
            qb.push(
                " AND EXISTS (SELECT 1 FROM service_tags st \
                   WHERE st.service_id = s.id AND st.tag_id IN (",
            );
            let mut sep = false;
            for id in &ids {
                if sep {
                    qb.push(", ");
                }
                qb.push_bind(id.as_bytes().to_vec());
                sep = true;
            }
            qb.push("))");
        }
    }

    // Haversine great-circle distance in miles (Earth mean radius 3959 mi).
    //   d = R * acos( cos(lat1) * cos(lat2) * cos(lon2 - lon1)
    //               + sin(lat1) * sin(lat2) )
    // LEAST(1.0, ...) guards against FP rounding pushing the acos input > 1.
    if let Some((lat, lon)) = user_coords {
        qb.push(" AND 3959 * ACOS(LEAST(1.0, ")
            .push("COS(RADIANS(")
            .push_bind(lat)
            .push("))")
            .push(" * COS(RADIANS(szc.latitude))")
            .push(" * COS(RADIANS(szc.longitude) - RADIANS(")
            .push_bind(lon)
            .push("))")
            .push(" + SIN(RADIANS(")
            .push_bind(lat)
            .push("))")
            .push(" * SIN(RADIANS(szc.latitude))")
            .push(")) <= s.coverage_radius_miles");
    }

    match sort {
        SortMode::LowestPrice => {
            qb.push(" ORDER BY s.price ASC, s.id ASC");
        }
        SortMode::SoonestAvailable => {
            // NULLs (no availability) last, then earliest start ascending.
            qb.push(" ORDER BY sa.soonest IS NULL ASC, sa.soonest ASC, s.id ASC");
        }
        SortMode::BestRated => {
            qb.push(" ORDER BY s.rating DESC, s.id ASC");
        }
    }

    let limit = query.limit.unwrap_or(50).min(200);
    let offset = query.offset.unwrap_or(0);
    qb.push(" LIMIT ")
        .push_bind(limit)
        .push(" OFFSET ")
        .push_bind(offset);

    let rows: Vec<(Vec<u8>, String, String, f64, f64, i32, String)> = qb
        .build_query_as()
        .fetch_all(pool.inner())
        .await
        .map_err(|_| Status::InternalServerError)?;

    let services = rows
        .into_iter()
        .filter_map(|(id, name, desc, price, rating, radius, zip)| {
            Some(Service {
                id: Uuid::from_slice(&id).ok()?.to_string(),
                name,
                description: desc,
                price,
                rating,
                coverage_radius_miles: radius,
                zip_code: zip,
            })
        })
        .collect();
    Ok(Json(services))
}

// ---------- Compare ----------

#[derive(FromForm)]
pub struct CompareQuery {
    pub ids: String,
}

#[get("/services/compare?<query..>")]
pub async fn compare(
    pool: &State<MySqlPool>,
    _user: AuthUser,
    query: CompareQuery,
) -> Result<Json<Vec<ServiceComparison>>, Status> {
    let ids: Vec<Uuid> = query
        .ids
        .split(',')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(Uuid::parse_str)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| Status::BadRequest)?;

    if ids.is_empty() || ids.len() > 3 {
        return Err(Status::BadRequest);
    }

    let mut out: Vec<ServiceComparison> = Vec::with_capacity(ids.len());
    for sid in ids {
        let svc_row: Option<(Vec<u8>, String, String, f64, f64, i32, String)> = sqlx::query_as(
            "SELECT id, name, description, price, rating, coverage_radius_miles, zip_code \
             FROM services WHERE id = ?",
        )
        .bind(&sid.as_bytes()[..])
        .fetch_optional(pool.inner())
        .await
        .map_err(|_| Status::InternalServerError)?;
        let Some((id_b, name, desc, price, rating, radius, zip)) = svc_row else {
            return Err(Status::NotFound);
        };

        let cats: Vec<(Vec<u8>, Option<Vec<u8>>, String)> = sqlx::query_as(
            "SELECT c.id, c.parent_id, c.name FROM categories c \
             JOIN service_categories sc ON sc.category_id = c.id \
             WHERE sc.service_id = ? ORDER BY c.name",
        )
        .bind(&sid.as_bytes()[..])
        .fetch_all(pool.inner())
        .await
        .map_err(|_| Status::InternalServerError)?;

        let tags: Vec<(Vec<u8>, String)> = sqlx::query_as(
            "SELECT t.id, t.name FROM tags t \
             JOIN service_tags st ON st.tag_id = t.id \
             WHERE st.service_id = ? ORDER BY t.name",
        )
        .bind(&sid.as_bytes()[..])
        .fetch_all(pool.inner())
        .await
        .map_err(|_| Status::InternalServerError)?;

        let avail: Vec<(Vec<u8>, chrono::NaiveDateTime, chrono::NaiveDateTime)> = sqlx::query_as(
            "SELECT id, start_time, end_time FROM availability \
             WHERE service_id = ? ORDER BY start_time",
        )
        .bind(&sid.as_bytes()[..])
        .fetch_all(pool.inner())
        .await
        .map_err(|_| Status::InternalServerError)?;

        out.push(ServiceComparison {
            service: Service {
                id: Uuid::from_slice(&id_b)
                    .map_err(|_| Status::InternalServerError)?
                    .to_string(),
                name,
                description: desc,
                price,
                rating,
                coverage_radius_miles: radius,
                zip_code: zip,
            },
            categories: cats
                .into_iter()
                .filter_map(|(cid, pid, cname)| {
                    Some(Category {
                        id: Uuid::from_slice(&cid).ok()?.to_string(),
                        parent_id: pid
                            .and_then(|b| Uuid::from_slice(&b).ok().map(|u| u.to_string())),
                        name: cname,
                    })
                })
                .collect(),
            tags: tags
                .into_iter()
                .filter_map(|(tid, tname)| {
                    Some(Tag {
                        id: Uuid::from_slice(&tid).ok()?.to_string(),
                        name: tname,
                    })
                })
                .collect(),
            availability: avail
                .into_iter()
                .filter_map(|(aid, st, et)| {
                    Some(AvailabilityWindow {
                        id: Uuid::from_slice(&aid).ok()?.to_string(),
                        service_id: sid.to_string(),
                        start_time: st,
                        end_time: et,
                    })
                })
                .collect(),
        });
    }

    Ok(Json(out))
}

// ---------- Favorite ----------

#[post("/services/<id>/favorite")]
pub async fn favorite(pool: &State<MySqlPool>, user: AuthUser, id: &str) -> Result<Status, Status> {
    let sid = Uuid::parse_str(id).map_err(|_| Status::BadRequest)?;
    sqlx::query(
        "INSERT INTO favorites (user_id, service_id) VALUES (?, ?) \
         ON DUPLICATE KEY UPDATE favorited_at = favorited_at",
    )
    .bind(&user.id.as_bytes()[..])
    .bind(&sid.as_bytes()[..])
    .execute(pool.inner())
    .await
    .map_err(|_| Status::BadRequest)?;
    Ok(Status::Created)
}
