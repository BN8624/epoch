// 시드 기반 세계 골격 생성·검증

use crate::error::CoreError;
use crate::rng::DeterministicRng;
use crate::world::{
    GenerationMeta, Realm, Ruler, Territory, WORLD_HEIGHT, WORLD_REALM_COUNT, WORLD_RULER_COUNT,
    WORLD_SCHEMA_VERSION, WORLD_TERRITORY_COUNT, WORLD_WIDTH, WorldSkeleton, coords_from_index,
    territory_id_at,
};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

/// 영역 분할 template ID.
pub const TEMPLATE_HORIZONTAL: &str = "horizontal";
pub const TEMPLATE_VERTICAL: &str = "vertical";
pub const TEMPLATE_BLOCKS_2X3: &str = "blocks_2x3";
pub const TEMPLATE_BLOCKS_3X2: &str = "blocks_3x2";

const TEMPLATE_IDS: [&str; 4] = [
    TEMPLATE_HORIZONTAL,
    TEMPLATE_VERTICAL,
    TEMPLATE_BLOCKS_2X3,
    TEMPLATE_BLOCKS_3X2,
];

/// 국가 이름 fixture (12개 이상, 중복 없음).
const REALM_NAME_POOL: [&str; 12] = [
    "Ashenmarch",
    "Blackmere",
    "Cairnwood",
    "Duskvale",
    "Emberhold",
    "Frostfen",
    "Goldreach",
    "Hearthglen",
    "Ironspan",
    "Jadeport",
    "Kestrel",
    "Larkfield",
];

/// 통치자 이름 fixture (12개 이상, 중복 없음).
const RULER_NAME_POOL: [&str; 12] = [
    "Aldric", "Beren", "Cedric", "Dorian", "Elias", "Faramund", "Gareth", "Hadrian", "Ivor",
    "Jorin", "Kael", "Lucan",
];

/// `seed`에서 항상 유효한 세계 골격을 생성한다. 불변식 실패 시 오류.
pub fn generate_world(seed: u64) -> Result<WorldSkeleton, CoreError> {
    let mut rng = DeterministicRng::new(seed);

    // 1. template 선택 (첫 draw % 4)
    let template_draw = rng.next_u64();
    let template_index = (template_draw % 4) as usize;
    let template_id = TEMPLATE_IDS[template_index].to_string();

    // 2. 국가·통치자 이름 deterministic shuffle 후 앞 6개
    let mut realm_names = REALM_NAME_POOL.map(str::to_string);
    fisher_yates_shuffle(&mut realm_names, &mut rng);
    let mut ruler_names = RULER_NAME_POOL.map(str::to_string);
    fisher_yates_shuffle(&mut ruler_names, &mut rng);

    let selected_realm_names: Vec<String> =
        realm_names.into_iter().take(WORLD_REALM_COUNT).collect();
    let selected_ruler_names: Vec<String> =
        ruler_names.into_iter().take(WORLD_RULER_COUNT).collect();

    // 3. template → 좌표별 region index (0..5)
    let region_of = region_map_for_template(template_index);

    // 4. realm ID / 이름 배정 (region i → realm-(i+1))
    let realm_ids: Vec<String> = (1..=WORLD_REALM_COUNT)
        .map(|i| format!("realm-{i:02}"))
        .collect();

    // region → territory ids
    let mut region_territories: Vec<Vec<String>> = vec![Vec::new(); WORLD_REALM_COUNT];
    for y in 0..WORLD_HEIGHT {
        for x in 0..WORLD_WIDTH {
            let region = region_of[y as usize][x as usize];
            region_territories[region].push(territory_id_at(x, y));
        }
    }
    for tids in &mut region_territories {
        tids.sort();
    }

    // 5. 수도 선택 (RNG 없음): 영역 내 Manhattan 거리 합 최소, 동점 시 ID 최소
    let capitals: Vec<String> = region_territories
        .iter()
        .map(|tids| select_capital(tids))
        .collect();

    // 6. rulers
    let rulers: Vec<Ruler> = (0..WORLD_RULER_COUNT)
        .map(|i| Ruler {
            id: format!("ruler-{:02}", i + 1),
            name: selected_ruler_names[i].clone(),
            realm_id: realm_ids[i].clone(),
            seat_territory_id: capitals[i].clone(),
        })
        .collect();

    // 7. realms
    let realms: Vec<Realm> = (0..WORLD_REALM_COUNT)
        .map(|i| Realm {
            id: realm_ids[i].clone(),
            name: selected_realm_names[i].clone(),
            capital_territory_id: capitals[i].clone(),
            ruler_id: rulers[i].id.clone(),
            territory_ids: region_territories[i].clone(),
        })
        .collect();

    // 8. territories (index 0..35 순)
    let mut coord_to_realm: BTreeMap<(u8, u8), String> = BTreeMap::new();
    for (region, tids) in region_territories.iter().enumerate() {
        for tid in tids {
            let index = parse_territory_index(tid)?;
            let (x, y) = coords_from_index(index);
            coord_to_realm.insert((x, y), realm_ids[region].clone());
        }
    }

    let mut territories = Vec::with_capacity(WORLD_TERRITORY_COUNT);
    for index in 0..WORLD_TERRITORY_COUNT {
        let (x, y) = coords_from_index(index);
        let id = territory_id_at(x, y);
        let realm_id = coord_to_realm
            .get(&(x, y))
            .cloned()
            .ok_or_else(|| CoreError::InvalidWorld(format!("missing realm for {id}")))?;
        let neighbors = orthogonal_neighbors(x, y);
        territories.push(Territory {
            id,
            x,
            y,
            realm_id,
            neighbors,
        });
    }

    let world = WorldSkeleton {
        schema_version: WORLD_SCHEMA_VERSION,
        seed,
        width: WORLD_WIDTH,
        height: WORLD_HEIGHT,
        generation: GenerationMeta {
            template_id,
            rng_draws: rng.draws(),
        },
        territories,
        realms,
        rulers,
    };

    validate_world(&world)?;
    Ok(world)
}

/// 세계 골격 불변식을 검사한다. 실패 시 fail closed.
pub fn validate_world(world: &WorldSkeleton) -> Result<(), CoreError> {
    // 전체 크기
    if world.schema_version != WORLD_SCHEMA_VERSION {
        return Err(CoreError::InvalidWorld(format!(
            "schema_version {} != {WORLD_SCHEMA_VERSION}",
            world.schema_version
        )));
    }
    if world.width != WORLD_WIDTH || world.height != WORLD_HEIGHT {
        return Err(CoreError::InvalidWorld(format!(
            "grid size {}x{} != {WORLD_WIDTH}x{WORLD_HEIGHT}",
            world.width, world.height
        )));
    }
    if world.territories.len() != WORLD_TERRITORY_COUNT {
        return Err(CoreError::InvalidWorld(format!(
            "territory count {} != {WORLD_TERRITORY_COUNT}",
            world.territories.len()
        )));
    }
    if world.realms.len() != WORLD_REALM_COUNT {
        return Err(CoreError::InvalidWorld(format!(
            "realm count {} != {WORLD_REALM_COUNT}",
            world.realms.len()
        )));
    }
    if world.rulers.len() != WORLD_RULER_COUNT {
        return Err(CoreError::InvalidWorld(format!(
            "ruler count {} != {WORLD_RULER_COUNT}",
            world.rulers.len()
        )));
    }

    // Territory 유일성·범위
    let mut territory_ids = BTreeSet::new();
    let mut coords = BTreeSet::new();
    let mut territory_by_id: BTreeMap<String, &Territory> = BTreeMap::new();
    for t in &world.territories {
        if !territory_ids.insert(t.id.clone()) {
            return Err(CoreError::InvalidWorld(format!(
                "duplicate territory id {}",
                t.id
            )));
        }
        if t.x >= WORLD_WIDTH || t.y >= WORLD_HEIGHT {
            return Err(CoreError::InvalidWorld(format!(
                "territory {} out of range ({},{})",
                t.id, t.x, t.y
            )));
        }
        if !coords.insert((t.x, t.y)) {
            return Err(CoreError::InvalidWorld(format!(
                "duplicate coordinates ({},{})",
                t.x, t.y
            )));
        }
        territory_by_id.insert(t.id.clone(), t);
    }

    let realm_ids: BTreeSet<String> = world.realms.iter().map(|r| r.id.clone()).collect();
    if realm_ids.len() != WORLD_REALM_COUNT {
        return Err(CoreError::InvalidWorld("duplicate realm ids".to_string()));
    }
    let realm_names: BTreeSet<String> = world.realms.iter().map(|r| r.name.clone()).collect();
    if realm_names.len() != WORLD_REALM_COUNT {
        return Err(CoreError::InvalidWorld("duplicate realm names".to_string()));
    }

    // realm 참조·영지 배분
    let mut claimed: BTreeMap<String, String> = BTreeMap::new();
    for realm in &world.realms {
        if realm.territory_ids.len() != 6 {
            return Err(CoreError::InvalidWorld(format!(
                "realm {} has {} territories (expected 6)",
                realm.id,
                realm.territory_ids.len()
            )));
        }
        let mut sorted = realm.territory_ids.clone();
        sorted.sort();
        if sorted != realm.territory_ids {
            return Err(CoreError::InvalidWorld(format!(
                "realm {} territory_ids not sorted",
                realm.id
            )));
        }
        let mut uniq = BTreeSet::new();
        for tid in &realm.territory_ids {
            if !territory_by_id.contains_key(tid) {
                return Err(CoreError::InvalidWorld(format!(
                    "realm {} references unknown territory {tid}",
                    realm.id
                )));
            }
            if !uniq.insert(tid.clone()) {
                return Err(CoreError::InvalidWorld(format!(
                    "realm {} has duplicate territory {tid}",
                    realm.id
                )));
            }
            if let Some(prev) = claimed.insert(tid.clone(), realm.id.clone()) {
                return Err(CoreError::InvalidWorld(format!(
                    "territory {tid} claimed by both {prev} and {}",
                    realm.id
                )));
            }
        }
        if !realm.territory_ids.contains(&realm.capital_territory_id) {
            return Err(CoreError::InvalidWorld(format!(
                "realm {} capital {} not in own territories",
                realm.id, realm.capital_territory_id
            )));
        }
        if !world.rulers.iter().any(|r| r.id == realm.ruler_id) {
            return Err(CoreError::InvalidWorld(format!(
                "realm {} ruler {} unknown",
                realm.id, realm.ruler_id
            )));
        }
    }
    if claimed.len() != WORLD_TERRITORY_COUNT {
        return Err(CoreError::InvalidWorld(format!(
            "claimed territories {} != {WORLD_TERRITORY_COUNT}",
            claimed.len()
        )));
    }

    // territory.realm_id 일치
    for t in &world.territories {
        let expected = claimed.get(&t.id).ok_or_else(|| {
            CoreError::InvalidWorld(format!("territory {} not claimed by any realm", t.id))
        })?;
        if &t.realm_id != expected {
            return Err(CoreError::InvalidWorld(format!(
                "territory {} realm_id {} != realm claim {expected}",
                t.id, t.realm_id
            )));
        }
        if !realm_ids.contains(&t.realm_id) {
            return Err(CoreError::InvalidWorld(format!(
                "territory {} unknown realm {}",
                t.id, t.realm_id
            )));
        }
    }

    // neighbors
    for t in &world.territories {
        let mut seen = BTreeSet::new();
        let mut sorted_check = t.neighbors.clone();
        sorted_check.sort();
        if sorted_check != t.neighbors {
            return Err(CoreError::InvalidWorld(format!(
                "territory {} neighbors not sorted",
                t.id
            )));
        }
        for n in &t.neighbors {
            if n == &t.id {
                return Err(CoreError::InvalidWorld(format!(
                    "territory {} self-neighbor",
                    t.id
                )));
            }
            if !seen.insert(n.clone()) {
                return Err(CoreError::InvalidWorld(format!(
                    "territory {} duplicate neighbor {n}",
                    t.id
                )));
            }
            let other = territory_by_id.get(n).ok_or_else(|| {
                CoreError::InvalidWorld(format!("territory {} neighbor {n} missing", t.id))
            })?;
            let dx = i16::from(t.x).abs_diff(i16::from(other.x));
            let dy = i16::from(t.y).abs_diff(i16::from(other.y));
            if dx + dy != 1 {
                return Err(CoreError::InvalidWorld(format!(
                    "territory {} neighbor {n} not orthogonal manhattan-1",
                    t.id
                )));
            }
            if !other.neighbors.contains(&t.id) {
                return Err(CoreError::InvalidWorld(format!(
                    "neighbor not bidirectional: {} -> {n}",
                    t.id
                )));
            }
        }
        // 좌표상 실제 직교 이웃과 일치
        let expected = orthogonal_neighbors(t.x, t.y);
        if t.neighbors != expected {
            return Err(CoreError::InvalidWorld(format!(
                "territory {} neighbors {:?} != expected {:?}",
                t.id, t.neighbors, expected
            )));
        }
    }

    // 전체 territory graph 연결
    if !is_connected_graph(
        &world
            .territories
            .iter()
            .map(|t| t.id.clone())
            .collect::<Vec<_>>(),
        &territory_by_id,
        |_| true,
    ) {
        return Err(CoreError::InvalidWorld(
            "full territory graph is not connected".to_string(),
        ));
    }

    // realm 영역 연결성
    for realm in &world.realms {
        let realm_set: BTreeSet<String> = realm.territory_ids.iter().cloned().collect();
        if !is_connected_graph(&realm.territory_ids, &territory_by_id, |tid| {
            realm_set.contains(tid)
        }) {
            return Err(CoreError::InvalidWorld(format!(
                "realm {} territories are not connected",
                realm.id
            )));
        }
        // capital 결정론 재검증
        let expected_cap = select_capital(&realm.territory_ids);
        if realm.capital_territory_id != expected_cap {
            return Err(CoreError::InvalidWorld(format!(
                "realm {} capital {} != expected {expected_cap}",
                realm.id, realm.capital_territory_id
            )));
        }
    }

    // Rulers
    let mut ruler_ids = BTreeSet::new();
    let mut ruler_names = BTreeSet::new();
    let mut realm_ruler_count: BTreeMap<String, u32> = BTreeMap::new();
    for ruler in &world.rulers {
        if !ruler_ids.insert(ruler.id.clone()) {
            return Err(CoreError::InvalidWorld(format!(
                "duplicate ruler id {}",
                ruler.id
            )));
        }
        if !ruler_names.insert(ruler.name.clone()) {
            return Err(CoreError::InvalidWorld(format!(
                "duplicate ruler name {}",
                ruler.name
            )));
        }
        if !realm_ids.contains(&ruler.realm_id) {
            return Err(CoreError::InvalidWorld(format!(
                "ruler {} unknown realm {}",
                ruler.id, ruler.realm_id
            )));
        }
        *realm_ruler_count.entry(ruler.realm_id.clone()).or_insert(0) += 1;
        let realm = world
            .realms
            .iter()
            .find(|r| r.id == ruler.realm_id)
            .ok_or_else(|| CoreError::InvalidWorld(format!("ruler {} realm missing", ruler.id)))?;
        if ruler.seat_territory_id != realm.capital_territory_id {
            return Err(CoreError::InvalidWorld(format!(
                "ruler {} seat {} != capital {}",
                ruler.id, ruler.seat_territory_id, realm.capital_territory_id
            )));
        }
        if realm.ruler_id != ruler.id {
            return Err(CoreError::InvalidWorld(format!(
                "realm {} ruler_id {} != ruler {}",
                realm.id, realm.ruler_id, ruler.id
            )));
        }
    }
    for realm in &world.realms {
        let count = realm_ruler_count.get(&realm.id).copied().unwrap_or(0);
        if count != 1 {
            return Err(CoreError::InvalidWorld(format!(
                "realm {} has {count} rulers (expected 1)",
                realm.id
            )));
        }
    }

    // template id 허용 목록
    if !TEMPLATE_IDS.contains(&world.generation.template_id.as_str()) {
        return Err(CoreError::InvalidWorld(format!(
            "unknown template_id {}",
            world.generation.template_id
        )));
    }

    Ok(())
}

fn fisher_yates_shuffle(items: &mut [String], rng: &mut DeterministicRng) {
    let n = items.len();
    if n < 2 {
        return;
    }
    for i in (1..n).rev() {
        let j = (rng.next_u64() % (i as u64 + 1)) as usize;
        items.swap(i, j);
    }
}

/// template_index 0..3 → 각 좌표의 region 0..5
fn region_map_for_template(template_index: usize) -> [[usize; 6]; 6] {
    let mut map = [[0usize; 6]; 6];
    for y in 0..6u8 {
        for x in 0..6u8 {
            let region = match template_index {
                0 => usize::from(y), // horizontal stripes
                1 => usize::from(x), // vertical stripes
                2 => {
                    // 2×3 blocks
                    let bx = usize::from(x) / 2;
                    let by = usize::from(y) / 3;
                    by * 3 + bx
                }
                3 => {
                    // 3×2 blocks
                    let bx = usize::from(x) / 3;
                    let by = usize::from(y) / 2;
                    by * 2 + bx
                }
                _ => unreachable!("template_index must be 0..3"),
            };
            map[y as usize][x as usize] = region;
        }
    }
    map
}

fn orthogonal_neighbors(x: u8, y: u8) -> Vec<String> {
    let mut out = Vec::new();
    let dirs: [(i16, i16); 4] = [(0, -1), (0, 1), (-1, 0), (1, 0)];
    for (dx, dy) in dirs {
        let nx = i16::from(x) + dx;
        let ny = i16::from(y) + dy;
        if nx >= 0 && ny >= 0 && nx < i16::from(WORLD_WIDTH) && ny < i16::from(WORLD_HEIGHT) {
            out.push(territory_id_at(nx as u8, ny as u8));
        }
    }
    out.sort();
    out
}

/// 영역 내 다른 영지까지 Manhattan 거리 합이 최소인 영지를 수도로 고른다.
/// 동점이면 territory ID 사전순 최소.
fn select_capital(territory_ids: &[String]) -> String {
    let coords: Vec<(String, u8, u8)> = territory_ids
        .iter()
        .map(|id| {
            let index = parse_territory_index_unchecked(id);
            let (x, y) = coords_from_index(index);
            (id.clone(), x, y)
        })
        .collect();

    let mut best_id = coords[0].0.clone();
    let mut best_sum = u32::MAX;
    for (id, x, y) in &coords {
        let mut sum = 0u32;
        for (oid, ox, oy) in &coords {
            if oid == id {
                continue;
            }
            let dx = u32::from(x.abs_diff(*ox));
            let dy = u32::from(y.abs_diff(*oy));
            sum = sum.saturating_add(dx + dy);
        }
        if sum < best_sum || (sum == best_sum && id < &best_id) {
            best_sum = sum;
            best_id = id.clone();
        }
    }
    best_id
}

fn parse_territory_index(id: &str) -> Result<usize, CoreError> {
    let suffix = id
        .strip_prefix("territory-")
        .ok_or_else(|| CoreError::InvalidWorld(format!("invalid territory id format: {id}")))?;
    suffix
        .parse::<usize>()
        .map_err(|_| CoreError::InvalidWorld(format!("invalid territory index in id: {id}")))
}

fn parse_territory_index_unchecked(id: &str) -> usize {
    id.strip_prefix("territory-")
        .and_then(|s| s.parse().ok())
        .unwrap_or(0)
}

/// `nodes`에서 시작해 `allow`를 통과하는 neighbor만 따라 연결되는지 BFS로 검사.
fn is_connected_graph(
    nodes: &[String],
    territory_by_id: &BTreeMap<String, &Territory>,
    allow: impl Fn(&str) -> bool,
) -> bool {
    if nodes.is_empty() {
        return true;
    }
    let node_set: BTreeSet<&str> = nodes.iter().map(String::as_str).collect();
    let start = nodes[0].as_str();
    let mut visited = BTreeSet::new();
    let mut queue = VecDeque::new();
    queue.push_back(start);
    visited.insert(start);
    while let Some(cur) = queue.pop_front() {
        let Some(t) = territory_by_id.get(cur) else {
            return false;
        };
        for n in &t.neighbors {
            if !allow(n) || !node_set.contains(n.as_str()) {
                continue;
            }
            if visited.insert(n.as_str()) {
                queue.push_back(n.as_str());
            }
        }
    }
    visited.len() == nodes.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_fixed_size_for_seed_1() {
        let w = generate_world(1).expect("world");
        assert_eq!(w.schema_version, WORLD_SCHEMA_VERSION);
        assert_eq!(w.width, 6);
        assert_eq!(w.height, 6);
        assert_eq!(w.territories.len(), 36);
        assert_eq!(w.realms.len(), 6);
        assert_eq!(w.rulers.len(), 6);
        for r in &w.realms {
            assert_eq!(r.territory_ids.len(), 6);
        }
    }

    #[test]
    fn all_four_templates_reachable_and_valid() {
        // 충분히 많은 seed에서 4 template을 모두 관측하고 검증한다.
        let mut seen = BTreeSet::new();
        for seed in 0u64..64 {
            let w = generate_world(seed).expect("world");
            seen.insert(w.generation.template_id.clone());
            validate_world(&w).expect("valid");
            for r in &w.realms {
                assert_eq!(r.territory_ids.len(), 6);
            }
        }
        for id in TEMPLATE_IDS {
            assert!(seen.contains(id), "template {id} never selected in 0..64");
        }
    }

    #[test]
    fn neighbor_bidirectional_and_orthogonal() {
        let w = generate_world(42).expect("world");
        let by_id: BTreeMap<_, _> = w.territories.iter().map(|t| (t.id.as_str(), t)).collect();
        for t in &w.territories {
            for n in &t.neighbors {
                let other = by_id[n.as_str()];
                assert!(other.neighbors.contains(&t.id));
                let dist = t.x.abs_diff(other.x) + t.y.abs_diff(other.y);
                assert_eq!(dist, 1);
            }
        }
    }

    #[test]
    fn capital_selection_deterministic_no_extra_rng() {
        let a = generate_world(7).expect("a");
        let b = generate_world(7).expect("b");
        for (ra, rb) in a.realms.iter().zip(b.realms.iter()) {
            assert_eq!(ra.capital_territory_id, rb.capital_territory_id);
        }
        // 재계산 일치
        for r in &a.realms {
            assert_eq!(r.capital_territory_id, select_capital(&r.territory_ids));
        }
    }

    #[test]
    fn names_unique_within_world() {
        for seed in [0u64, 1, 2, 42, u64::MAX] {
            let w = generate_world(seed).expect("world");
            let mut rn = BTreeSet::new();
            let mut un = BTreeSet::new();
            for r in &w.realms {
                assert!(rn.insert(r.name.clone()), "dup realm name {}", r.name);
            }
            for u in &w.rulers {
                assert!(un.insert(u.name.clone()), "dup ruler name {}", u.name);
            }
        }
    }

    #[test]
    fn same_seed_structure_and_bytes_equal() {
        let a = generate_world(1).expect("a");
        let b = generate_world(1).expect("b");
        assert_eq!(a, b);
        let ba = a.to_compact_json_bytes().expect("ba");
        let bb = b.to_compact_json_bytes().expect("bb");
        assert_eq!(ba, bb);
    }

    #[test]
    fn seed_1_and_2_differ() {
        let a = generate_world(1).expect("1");
        let b = generate_world(2).expect("2");
        let ba = a.to_compact_json_bytes().expect("ba");
        let bb = b.to_compact_json_bytes().expect("bb");
        assert_ne!(ba, bb);
        assert_ne!(a, b);
    }

    #[test]
    fn full_invariants_on_representative_seeds() {
        for seed in [0u64, 1, 2, 42, u64::MAX] {
            let w = generate_world(seed).expect("world");
            validate_world(&w).expect("invariants");
        }
    }

    #[test]
    fn capital_is_geometric_median_of_realm() {
        // horizontal: 각 행 y, 영지 x=0..5 → 중앙 후보 x=2 또는 x=3 동점 시 ID 최소
        // territory index = y*6+x → 행 y의 ID territory-(y*6).. 중 거리합 최소
        let w = generate_world(0).expect("world");
        // 어떤 template이든 select_capital과 일치만 확인 (위 테스트와 중복 방지 차원의 단위)
        for r in &w.realms {
            let cap = select_capital(&r.territory_ids);
            assert_eq!(r.capital_territory_id, cap);
            assert!(r.territory_ids.contains(&cap));
        }
    }
}
