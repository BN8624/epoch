// M1.2 인구·가계 골격 생성·검증

use crate::error::CoreError;
use crate::population::{
    CURRENTS_PER_HOUSE, DYNASTIC_WORLD_SCHEMA_VERSION, DynasticWorld, ELDERS_PER_HOUSE,
    GenerationBand, HOUSE_COUNT, HOUSES_PER_REALM, House, PERSON_COUNT, PERSONS_PER_HOUSE, Person,
    PopulationGenerationMeta, PopulationSkeleton, RulerPersonLink, YOUNGS_PER_HOUSE,
    generation_for_member, house_id_at, person_id_at,
};
use crate::rng::DeterministicRng;
use crate::world::WorldSkeleton;
use crate::worldgen::{generate_world, validate_world};
use std::collections::{BTreeMap, BTreeSet};

/// population RNG stream domain separation (M1.1 world RNG와 분리).
const POPULATION_SEED_DOMAIN: u64 = 0xE90C_0001_4D12_0002;

/// 고정 House 이름 fixture (24개 이상, 중복 없음).
const HOUSE_NAME_POOL: [&str; 24] = [
    "Ashcroft",
    "Blackthorn",
    "Coldwater",
    "Dunmoor",
    "Eastmarch",
    "Fairwind",
    "Greyvale",
    "Highcliff",
    "Ironwood",
    "Jadeford",
    "Kingsley",
    "Longridge",
    "Mossbrook",
    "Northfield",
    "Oakenshield",
    "Pinecrest",
    "Quarrygate",
    "Ravenhill",
    "Silverbrook",
    "Thornwall",
    "Underwood",
    "Valehart",
    "Westbridge",
    "Yarrow",
];

/// 고정 given-name fixture (24개 이상, 중복 없음).
const GIVEN_NAME_POOL: [&str; 32] = [
    "Aric", "Bram", "Corin", "Davos", "Edric", "Fenric", "Gwil", "Harun", "Idra", "Jessa", "Kira",
    "Liora", "Marek", "Nessa", "Orin", "Pella", "Quen", "Rhea", "Soren", "Talia", "Ulric", "Vera",
    "Wynn", "Xara", "Ysolde", "Zane", "Alma", "Bren", "Cleo", "Dune", "Eira", "Falk",
];

/// 유효한 세계 골격 위에 인구·가계 골격을 생성한다. 불변식 실패 시 오류.
pub fn generate_population(world: &WorldSkeleton) -> Result<PopulationSkeleton, CoreError> {
    validate_world(world)?;

    let population_seed = world.seed ^ POPULATION_SEED_DOMAIN;
    let mut rng = DeterministicRng::new(population_seed);

    // House names: population RNG로 Fisher-Yates 후 앞 18개
    let mut house_names = HOUSE_NAME_POOL.map(str::to_string);
    fisher_yates_shuffle(&mut house_names, &mut rng);
    let selected_house_names: Vec<String> = house_names.into_iter().take(HOUSE_COUNT).collect();

    let mut houses = Vec::with_capacity(HOUSE_COUNT);
    let mut persons = Vec::with_capacity(PERSON_COUNT);
    let mut ruler_links = Vec::with_capacity(world.rulers.len());

    // house ID 배정은 world.realms 벡터 순서가 아니라 realm ID 정규 순서 기준
    // (realm-01 → house-01..03, realm-02 → house-04..06, …).
    let mut realms_by_id: Vec<&crate::world::Realm> = world.realms.iter().collect();
    realms_by_id.sort_by(|a, b| a.id.cmp(&b.id));

    for (realm_index, realm) in realms_by_id.iter().enumerate() {
        let seats = select_house_seats(realm)?;
        let ruler = world
            .rulers
            .iter()
            .find(|r| r.id == realm.ruler_id)
            .ok_or_else(|| {
                CoreError::InvalidPopulation(format!(
                    "realm {} ruler {} missing",
                    realm.id, realm.ruler_id
                ))
            })?;

        for (local_house, seat) in seats.iter().enumerate() {
            let house_index = realm_index * HOUSES_PER_REALM + local_house;
            let house_id = house_id_at(house_index);
            let house_name = selected_house_names[house_index].clone();

            let base_person = house_index * PERSONS_PER_HOUSE;
            let mut member_ids = Vec::with_capacity(PERSONS_PER_HOUSE);
            for m in 0..PERSONS_PER_HOUSE {
                member_ids.push(person_id_at(base_person + m));
            }

            // head = current[0] = member 2
            let head_person_id = member_ids[2].clone();

            // 가문 내 이름 배정 (중복 금지). ruling house head는 ruler name 우선 예약.
            let is_ruling_house = local_house == 0;
            let names =
                assign_house_names(house_index, population_seed, is_ruling_house, &ruler.name)?;

            for (m, person_id) in member_ids.iter().enumerate() {
                let generation = generation_for_member(m);
                let known_parent_ids = parent_ids_for_member(m, &member_ids);
                persons.push(Person {
                    id: person_id.clone(),
                    name: names[m].clone(),
                    house_id: house_id.clone(),
                    realm_id: realm.id.clone(),
                    home_territory_id: seat.clone(),
                    generation,
                    known_parent_ids,
                });
            }

            houses.push(House {
                id: house_id,
                name: house_name,
                realm_id: realm.id.clone(),
                seat_territory_id: seat.clone(),
                head_person_id: head_person_id.clone(),
                member_ids,
            });

            if is_ruling_house {
                ruler_links.push(RulerPersonLink {
                    ruler_id: ruler.id.clone(),
                    person_id: head_person_id,
                });
            }
        }
    }

    // 안정 순서: house-01..18 (realm ID 정규 순), person-001..144
    // member_ids는 이미 person ID 순

    let population = PopulationSkeleton {
        houses,
        persons,
        ruler_links,
        generation: PopulationGenerationMeta {
            rng_draws: rng.draws(),
        },
    };

    validate_population(world, &population)?;
    Ok(population)
}

/// 시드에서 세계 골격과 인구 계층을 함께 생성한다.
pub fn generate_dynastic_world(seed: u64) -> Result<DynasticWorld, CoreError> {
    let world = generate_world(seed)?;
    let population = generate_population(&world)?;
    let dynastic = DynasticWorld {
        schema_version: DYNASTIC_WORLD_SCHEMA_VERSION,
        seed,
        world,
        population,
    };
    Ok(dynastic)
}

/// 인구·가계 골격 불변식을 검사한다. 실패 시 fail closed.
pub fn validate_population(
    world: &WorldSkeleton,
    population: &PopulationSkeleton,
) -> Result<(), CoreError> {
    validate_world(world).map_err(|e| match e {
        CoreError::InvalidWorld(msg) => CoreError::InvalidPopulation(format!("world: {msg}")),
        other => other,
    })?;

    // Counts
    if population.houses.len() != HOUSE_COUNT {
        return Err(CoreError::InvalidPopulation(format!(
            "house count {} != {HOUSE_COUNT}",
            population.houses.len()
        )));
    }
    if population.persons.len() != PERSON_COUNT {
        return Err(CoreError::InvalidPopulation(format!(
            "person count {} != {PERSON_COUNT}",
            population.persons.len()
        )));
    }
    if population.ruler_links.len() != world.rulers.len() {
        return Err(CoreError::InvalidPopulation(format!(
            "ruler_links {} != rulers {}",
            population.ruler_links.len(),
            world.rulers.len()
        )));
    }

    let mut elder_n = 0usize;
    let mut current_n = 0usize;
    let mut young_n = 0usize;
    for p in &population.persons {
        match p.generation {
            GenerationBand::Elder => elder_n += 1,
            GenerationBand::Current => current_n += 1,
            GenerationBand::Young => young_n += 1,
        }
    }
    if elder_n != 36 || current_n != 54 || young_n != 54 {
        return Err(CoreError::InvalidPopulation(format!(
            "generation totals elder={elder_n} current={current_n} young={young_n} (expected 36/54/54)"
        )));
    }

    let realm_ids: BTreeSet<String> = world.realms.iter().map(|r| r.id.clone()).collect();
    let territory_by_id: BTreeMap<String, &crate::world::Territory> = world
        .territories
        .iter()
        .map(|t| (t.id.clone(), t))
        .collect();
    let realm_by_id: BTreeMap<String, &crate::world::Realm> =
        world.realms.iter().map(|r| (r.id.clone(), r)).collect();
    let ruler_by_id: BTreeMap<String, &crate::world::Ruler> =
        world.rulers.iter().map(|r| (r.id.clone(), r)).collect();

    // House uniqueness + exact ID set house-01..18
    let expected_house_ids: BTreeSet<String> = (0..HOUSE_COUNT).map(house_id_at).collect();
    let mut house_ids = BTreeSet::new();
    let mut house_names = BTreeSet::new();
    let mut house_by_id: BTreeMap<String, &House> = BTreeMap::new();
    let mut person_house_claim: BTreeMap<String, String> = BTreeMap::new();
    let mut houses_per_realm: BTreeMap<String, usize> = BTreeMap::new();

    for house in &population.houses {
        if !house_ids.insert(house.id.clone()) {
            return Err(CoreError::InvalidPopulation(format!(
                "duplicate house id {}",
                house.id
            )));
        }
        if !house_names.insert(house.name.clone()) {
            return Err(CoreError::InvalidPopulation(format!(
                "duplicate house name {}",
                house.name
            )));
        }
        if !realm_ids.contains(&house.realm_id) {
            return Err(CoreError::InvalidPopulation(format!(
                "house {} unknown realm {}",
                house.id, house.realm_id
            )));
        }
        *houses_per_realm.entry(house.realm_id.clone()).or_insert(0) += 1;

        let realm = realm_by_id.get(&house.realm_id).ok_or_else(|| {
            CoreError::InvalidPopulation(format!("house {} realm missing", house.id))
        })?;
        if !realm.territory_ids.contains(&house.seat_territory_id) {
            return Err(CoreError::InvalidPopulation(format!(
                "house {} seat {} not in realm {}",
                house.id, house.seat_territory_id, house.realm_id
            )));
        }

        if house.member_ids.len() != PERSONS_PER_HOUSE {
            return Err(CoreError::InvalidPopulation(format!(
                "house {} members {} != {PERSONS_PER_HOUSE}",
                house.id,
                house.member_ids.len()
            )));
        }
        let mut sorted_members = house.member_ids.clone();
        sorted_members.sort();
        if sorted_members != house.member_ids {
            return Err(CoreError::InvalidPopulation(format!(
                "house {} member_ids not sorted",
                house.id
            )));
        }
        let mut uniq_members = BTreeSet::new();
        for mid in &house.member_ids {
            if !uniq_members.insert(mid.clone()) {
                return Err(CoreError::InvalidPopulation(format!(
                    "house {} duplicate member {mid}",
                    house.id
                )));
            }
            if let Some(prev) = person_house_claim.insert(mid.clone(), house.id.clone()) {
                return Err(CoreError::InvalidPopulation(format!(
                    "person {mid} claimed by both {prev} and {}",
                    house.id
                )));
            }
        }
        if !house.member_ids.contains(&house.head_person_id) {
            return Err(CoreError::InvalidPopulation(format!(
                "house {} head {} not a member",
                house.id, house.head_person_id
            )));
        }
        // head = current[0] = member_ids[2]
        if house.head_person_id != house.member_ids[2] {
            return Err(CoreError::InvalidPopulation(format!(
                "house {} head {} != member_ids[2] {}",
                house.id, house.head_person_id, house.member_ids[2]
            )));
        }

        house_by_id.insert(house.id.clone(), house);
    }

    if house_ids != expected_house_ids {
        return Err(CoreError::InvalidPopulation(format!(
            "house id set mismatch: got {house_ids:?} expected house-01..house-18"
        )));
    }

    // 국가별 연속 house ID 배정: realm ID 정규 순서 기준
    // realm-01 → house-01..03, realm-02 → house-04..06, …
    let mut sorted_realm_ids: Vec<String> = world.realms.iter().map(|r| r.id.clone()).collect();
    sorted_realm_ids.sort();
    for (realm_index, realm_id) in sorted_realm_ids.iter().enumerate() {
        let mut realm_houses: Vec<&House> = population
            .houses
            .iter()
            .filter(|h| h.realm_id == *realm_id)
            .collect();
        realm_houses.sort_by(|a, b| a.id.cmp(&b.id));
        if realm_houses.len() != HOUSES_PER_REALM {
            return Err(CoreError::InvalidPopulation(format!(
                "realm {realm_id} has {} houses (expected {HOUSES_PER_REALM})",
                realm_houses.len()
            )));
        }
        for (local, h) in realm_houses.iter().enumerate() {
            let expected_id = house_id_at(realm_index * HOUSES_PER_REALM + local);
            if h.id != expected_id {
                return Err(CoreError::InvalidPopulation(format!(
                    "realm {realm_id} local house {local} id {} != expected {expected_id}",
                    h.id
                )));
            }
        }
    }

    for realm in &world.realms {
        let count = houses_per_realm.get(&realm.id).copied().unwrap_or(0);
        if count != HOUSES_PER_REALM {
            return Err(CoreError::InvalidPopulation(format!(
                "realm {} has {count} houses (expected {HOUSES_PER_REALM})",
                realm.id
            )));
        }
        // seat uniqueness within realm
        let seats: Vec<&str> = population
            .houses
            .iter()
            .filter(|h| h.realm_id == realm.id)
            .map(|h| h.seat_territory_id.as_str())
            .collect();
        let seat_set: BTreeSet<&str> = seats.iter().copied().collect();
        if seat_set.len() != seats.len() {
            return Err(CoreError::InvalidPopulation(format!(
                "realm {} house seats not unique",
                realm.id
            )));
        }
    }

    // Persons
    let mut person_ids = BTreeSet::new();
    let mut person_by_id: BTreeMap<String, &Person> = BTreeMap::new();
    let mut persons_per_realm: BTreeMap<String, usize> = BTreeMap::new();

    for person in &population.persons {
        if !person_ids.insert(person.id.clone()) {
            return Err(CoreError::InvalidPopulation(format!(
                "duplicate person id {}",
                person.id
            )));
        }
        let house = house_by_id.get(&person.house_id).ok_or_else(|| {
            CoreError::InvalidPopulation(format!(
                "person {} unknown house {}",
                person.id, person.house_id
            ))
        })?;
        if person.realm_id != house.realm_id {
            return Err(CoreError::InvalidPopulation(format!(
                "person {} realm {} != house realm {}",
                person.id, person.realm_id, house.realm_id
            )));
        }
        if !realm_ids.contains(&person.realm_id) {
            return Err(CoreError::InvalidPopulation(format!(
                "person {} unknown realm {}",
                person.id, person.realm_id
            )));
        }
        if !territory_by_id.contains_key(&person.home_territory_id) {
            return Err(CoreError::InvalidPopulation(format!(
                "person {} unknown home {}",
                person.id, person.home_territory_id
            )));
        }
        if person.home_territory_id != house.seat_territory_id {
            return Err(CoreError::InvalidPopulation(format!(
                "person {} home {} != house seat {}",
                person.id, person.home_territory_id, house.seat_territory_id
            )));
        }
        let claimed_house = person_house_claim.get(&person.id).ok_or_else(|| {
            CoreError::InvalidPopulation(format!(
                "person {} not listed in any house.member_ids",
                person.id
            ))
        })?;
        if claimed_house != &person.house_id {
            return Err(CoreError::InvalidPopulation(format!(
                "person {} house_id {} != member claim {claimed_house}",
                person.id, person.house_id
            )));
        }
        *persons_per_realm
            .entry(person.realm_id.clone())
            .or_insert(0) += 1;
        person_by_id.insert(person.id.clone(), person);
    }

    if person_house_claim.len() != PERSON_COUNT {
        return Err(CoreError::InvalidPopulation(format!(
            "member claims {} != {PERSON_COUNT}",
            person_house_claim.len()
        )));
    }

    for realm in &world.realms {
        let count = persons_per_realm.get(&realm.id).copied().unwrap_or(0);
        if count != HOUSES_PER_REALM * PERSONS_PER_HOUSE {
            return Err(CoreError::InvalidPopulation(format!(
                "realm {} has {count} persons (expected {})",
                realm.id,
                HOUSES_PER_REALM * PERSONS_PER_HOUSE
            )));
        }
    }

    // House generation composition + fixed member-slot placement + head Current
    // member 0,1 → Elder; 2,3,4 → Current; 5,6,7 → Young; head == member_ids[2]
    for house in &population.houses {
        let mut e = 0usize;
        let mut c = 0usize;
        let mut y = 0usize;
        let mut names = BTreeSet::new();
        for (m, mid) in house.member_ids.iter().enumerate() {
            let p = person_by_id.get(mid).ok_or_else(|| {
                CoreError::InvalidPopulation(format!(
                    "house {} member {mid} missing person record",
                    house.id
                ))
            })?;
            let expected_gen = generation_for_member(m);
            if p.generation != expected_gen {
                return Err(CoreError::InvalidPopulation(format!(
                    "house {} member_ids[{m}] {} generation {:?} != expected {:?}",
                    house.id, mid, p.generation, expected_gen
                )));
            }
            match p.generation {
                GenerationBand::Elder => e += 1,
                GenerationBand::Current => c += 1,
                GenerationBand::Young => y += 1,
            }
            if !names.insert(p.name.as_str()) {
                return Err(CoreError::InvalidPopulation(format!(
                    "house {} duplicate person name {}",
                    house.id, p.name
                )));
            }
        }
        if e != ELDERS_PER_HOUSE || c != CURRENTS_PER_HOUSE || y != YOUNGS_PER_HOUSE {
            return Err(CoreError::InvalidPopulation(format!(
                "house {} generation counts {e}/{c}/{y} != {ELDERS_PER_HOUSE}/{CURRENTS_PER_HOUSE}/{YOUNGS_PER_HOUSE}",
                house.id
            )));
        }
        let head = person_by_id.get(&house.head_person_id).ok_or_else(|| {
            CoreError::InvalidPopulation(format!(
                "house {} head {} missing",
                house.id, house.head_person_id
            ))
        })?;
        if head.generation != GenerationBand::Current {
            return Err(CoreError::InvalidPopulation(format!(
                "house {} head {} is not Current",
                house.id, house.head_person_id
            )));
        }
        if head.realm_id != house.realm_id {
            return Err(CoreError::InvalidPopulation(format!(
                "house {} head realm mismatch",
                house.id
            )));
        }
        if head.home_territory_id != house.seat_territory_id {
            return Err(CoreError::InvalidPopulation(format!(
                "house {} head home != seat",
                house.id
            )));
        }
    }

    // Parent graph
    for person in &population.persons {
        // no self, no dupes
        let mut seen_parents = BTreeSet::new();
        for pid in &person.known_parent_ids {
            if pid == &person.id {
                return Err(CoreError::InvalidPopulation(format!(
                    "person {} self-parent",
                    person.id
                )));
            }
            if !seen_parents.insert(pid.clone()) {
                return Err(CoreError::InvalidPopulation(format!(
                    "person {} duplicate parent {pid}",
                    person.id
                )));
            }
            let parent = person_by_id.get(pid).ok_or_else(|| {
                CoreError::InvalidPopulation(format!("person {} parent {pid} missing", person.id))
            })?;
            if parent.house_id != person.house_id {
                return Err(CoreError::InvalidPopulation(format!(
                    "person {} parent {pid} different house",
                    person.id
                )));
            }
            let expected_parent_gen = match person.generation {
                GenerationBand::Elder => {
                    return Err(CoreError::InvalidPopulation(format!(
                        "elder {} has parent {pid}",
                        person.id
                    )));
                }
                GenerationBand::Current => GenerationBand::Elder,
                GenerationBand::Young => GenerationBand::Current,
            };
            if parent.generation != expected_parent_gen {
                return Err(CoreError::InvalidPopulation(format!(
                    "person {} parent {pid} generation {:?} != expected {:?}",
                    person.id, parent.generation, expected_parent_gen
                )));
            }
        }
        match person.generation {
            GenerationBand::Elder => {
                if !person.known_parent_ids.is_empty() {
                    return Err(CoreError::InvalidPopulation(format!(
                        "elder {} must have no parents",
                        person.id
                    )));
                }
            }
            GenerationBand::Current | GenerationBand::Young => {
                if person.known_parent_ids.is_empty() {
                    return Err(CoreError::InvalidPopulation(format!(
                        "person {} ({:?}) needs at least one parent",
                        person.id, person.generation
                    )));
                }
            }
        }
    }

    // Ruler links
    let mut linked_rulers = BTreeSet::new();
    let mut linked_persons = BTreeSet::new();
    for link in &population.ruler_links {
        if !linked_rulers.insert(link.ruler_id.clone()) {
            return Err(CoreError::InvalidPopulation(format!(
                "duplicate ruler link for {}",
                link.ruler_id
            )));
        }
        if !linked_persons.insert(link.person_id.clone()) {
            return Err(CoreError::InvalidPopulation(format!(
                "person {} linked to multiple rulers",
                link.person_id
            )));
        }
        let ruler = ruler_by_id.get(&link.ruler_id).ok_or_else(|| {
            CoreError::InvalidPopulation(format!("ruler link unknown ruler {}", link.ruler_id))
        })?;
        let person = person_by_id.get(&link.person_id).ok_or_else(|| {
            CoreError::InvalidPopulation(format!("ruler link unknown person {}", link.person_id))
        })?;
        if person.generation != GenerationBand::Current {
            return Err(CoreError::InvalidPopulation(format!(
                "linked person {} is not Current",
                person.id
            )));
        }
        if person.realm_id != ruler.realm_id {
            return Err(CoreError::InvalidPopulation(format!(
                "ruler {} realm {} != person realm {}",
                ruler.id, ruler.realm_id, person.realm_id
            )));
        }
        if person.name != ruler.name {
            return Err(CoreError::InvalidPopulation(format!(
                "ruler {} name {} != person name {}",
                ruler.id, ruler.name, person.name
            )));
        }
        let realm = realm_by_id.get(&ruler.realm_id).ok_or_else(|| {
            CoreError::InvalidPopulation(format!("ruler {} realm missing", ruler.id))
        })?;
        // ruling house = first house ID among the realm's houses (house-01, house-04, ...)
        let mut realm_houses: Vec<&House> = population
            .houses
            .iter()
            .filter(|h| h.realm_id == realm.id)
            .collect();
        realm_houses.sort_by(|a, b| a.id.cmp(&b.id));
        if realm_houses.is_empty() {
            return Err(CoreError::InvalidPopulation(format!(
                "no house for realm {}",
                realm.id
            )));
        }
        let ruling = realm_houses[0];
        if person.id != ruling.head_person_id {
            return Err(CoreError::InvalidPopulation(format!(
                "ruler {} person {} != ruling house {} head {}",
                ruler.id, person.id, ruling.id, ruling.head_person_id
            )));
        }
        if ruling.seat_territory_id != realm.capital_territory_id {
            return Err(CoreError::InvalidPopulation(format!(
                "ruling house {} seat {} != capital {}",
                ruling.id, ruling.seat_territory_id, realm.capital_territory_id
            )));
        }
        if person.home_territory_id != realm.capital_territory_id {
            return Err(CoreError::InvalidPopulation(format!(
                "linked person {} home {} != capital {}",
                person.id, person.home_territory_id, realm.capital_territory_id
            )));
        }
        if ruler.seat_territory_id != realm.capital_territory_id {
            return Err(CoreError::InvalidPopulation(format!(
                "ruler {} seat != capital",
                ruler.id
            )));
        }
    }
    for ruler in &world.rulers {
        if !linked_rulers.contains(&ruler.id) {
            return Err(CoreError::InvalidPopulation(format!(
                "ruler {} has no person link",
                ruler.id
            )));
        }
    }

    // Seat selection rule re-check per realm
    for realm in &world.realms {
        let expected = select_house_seats(realm)?;
        let mut realm_houses: Vec<&House> = population
            .houses
            .iter()
            .filter(|h| h.realm_id == realm.id)
            .collect();
        realm_houses.sort_by(|a, b| a.id.cmp(&b.id));
        if realm_houses.len() != HOUSES_PER_REALM {
            return Err(CoreError::InvalidPopulation(format!(
                "realm {} house count mismatch in seat check",
                realm.id
            )));
        }
        for (i, h) in realm_houses.iter().enumerate() {
            if h.seat_territory_id != expected[i] {
                return Err(CoreError::InvalidPopulation(format!(
                    "house {} seat {} != expected {}",
                    h.id, h.seat_territory_id, expected[i]
                )));
            }
        }
    }

    Ok(())
}

/// 국가 3개 가문 거점: 수도 + 나머지 territory ID 오름차순 앞 2개.
fn select_house_seats(realm: &crate::world::Realm) -> Result<[String; 3], CoreError> {
    let capital = &realm.capital_territory_id;
    let mut others: Vec<String> = realm
        .territory_ids
        .iter()
        .filter(|t| *t != capital)
        .cloned()
        .collect();
    others.sort();
    if others.len() < 2 {
        return Err(CoreError::InvalidPopulation(format!(
            "realm {} needs at least 2 non-capital territories for seats",
            realm.id
        )));
    }
    Ok([capital.clone(), others[0].clone(), others[1].clone()])
}

/// member index에 대한 known_parent_ids (가문 내 고정 규칙).
fn parent_ids_for_member(member_index: usize, member_ids: &[String]) -> Vec<String> {
    match member_index {
        0 | 1 => Vec::new(),              // Elder
        2 => vec![member_ids[0].clone()], // current[0] → elder[0]
        3 => vec![member_ids[1].clone()], // current[1] → elder[1]
        4 => vec![member_ids[0].clone()], // current[2] → elder[0]
        5 => vec![member_ids[2].clone()], // young[0] → current[0]
        6 => vec![member_ids[3].clone()], // young[1] → current[1]
        7 => vec![member_ids[4].clone()], // young[2] → current[2]
        _ => Vec::new(),
    }
}

/// 가문 8명 display name 배정. ruling house head(member 2)는 ruler_name을 먼저 예약한다.
fn assign_house_names(
    house_index: usize,
    population_seed: u64,
    is_ruling_house: bool,
    ruler_name: &str,
) -> Result<[String; PERSONS_PER_HOUSE], CoreError> {
    let pool_len = GIVEN_NAME_POOL.len();
    let mut names: [String; PERSONS_PER_HOUSE] = std::array::from_fn(|_| String::new());
    let mut used = BTreeSet::new();

    // ruling house: head 이름을 먼저 예약해 다른 member가 동일 이름을 고르지 못하게 한다.
    if is_ruling_house {
        names[2] = ruler_name.to_string();
        used.insert(ruler_name.to_string());
    }

    for (m, slot) in names.iter_mut().enumerate() {
        if is_ruling_house && m == 2 {
            continue;
        }
        // deterministic start from house/member/seed; walk pool until unique in house
        let start = (house_index
            .wrapping_mul(17)
            .wrapping_add(m.wrapping_mul(31))
            .wrapping_add((population_seed as usize).wrapping_mul(13)))
            % pool_len;
        let mut chosen = None;
        for step in 0..pool_len {
            let candidate = GIVEN_NAME_POOL[(start + step) % pool_len];
            if !used.contains(candidate) {
                chosen = Some(candidate.to_string());
                break;
            }
        }
        let name = chosen.ok_or_else(|| {
            CoreError::InvalidPopulation(format!(
                "exhausted given-name pool for house_index={house_index} member={m}"
            ))
        })?;
        used.insert(name.clone());
        *slot = name;
    }

    Ok(names)
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::population::generation_for_member;

    #[test]
    fn generate_fixed_counts_seed_1() {
        let d = generate_dynastic_world(1).expect("dynastic");
        assert_eq!(d.schema_version, DYNASTIC_WORLD_SCHEMA_VERSION);
        assert_eq!(d.population.houses.len(), 18);
        assert_eq!(d.population.persons.len(), 144);
        assert_eq!(d.population.ruler_links.len(), 6);
    }

    #[test]
    fn same_seed_structure_and_bytes_equal() {
        let a = generate_dynastic_world(1).expect("a");
        let b = generate_dynastic_world(1).expect("b");
        assert_eq!(a, b);
        assert_eq!(
            a.to_compact_json_bytes().unwrap(),
            b.to_compact_json_bytes().unwrap()
        );
    }

    #[test]
    fn seed_1_and_2_differ() {
        let a = generate_dynastic_world(1).expect("1");
        let b = generate_dynastic_world(2).expect("2");
        assert_ne!(a, b);
        assert_ne!(
            a.to_compact_json_bytes().unwrap(),
            b.to_compact_json_bytes().unwrap()
        );
    }

    #[test]
    fn world_bytes_unchanged_by_population() {
        let w1 = generate_world(1).expect("w1");
        let d1 = generate_dynastic_world(1).expect("d1");
        assert_eq!(
            w1.to_compact_json_bytes().unwrap(),
            d1.world.to_compact_json_bytes().unwrap()
        );
        assert_eq!(w1, d1.world);
    }

    /// realm 벡터 순서만 뒤집어 도 house-01..18이 realm ID 정규 순으로 배정된다.
    #[test]
    fn house_ids_follow_realm_id_order_not_vector_order() {
        let mut world = generate_world(1).expect("world");
        world.realms.reverse();
        validate_world(&world).expect("reordered realms still valid");

        let pop = generate_population(&world).expect("population");
        let mut by_realm: BTreeMap<String, Vec<String>> = BTreeMap::new();
        for h in &pop.houses {
            by_realm
                .entry(h.realm_id.clone())
                .or_default()
                .push(h.id.clone());
        }
        for ids in by_realm.values_mut() {
            ids.sort();
        }
        assert_eq!(
            by_realm.get("realm-01").cloned().unwrap_or_default(),
            vec![
                "house-01".to_string(),
                "house-02".to_string(),
                "house-03".to_string()
            ]
        );
        assert_eq!(
            by_realm.get("realm-02").cloned().unwrap_or_default(),
            vec![
                "house-04".to_string(),
                "house-05".to_string(),
                "house-06".to_string()
            ]
        );
        assert_eq!(
            by_realm.get("realm-06").cloned().unwrap_or_default(),
            vec![
                "house-16".to_string(),
                "house-17".to_string(),
                "house-18".to_string()
            ]
        );
        validate_population(&world, &pop).expect("validate reordered");
    }

    /// ruling house head 이름을 먼저 예약하면 fixture와 충돌해도 가문 내 이름 중복이 없다.
    #[test]
    fn assign_house_names_reserves_ruler_name_first() {
        // GIVEN_NAME_POOL 첫 이름과 동일한 ruler 이름 → member 0 시작 후보와 충돌 가능
        let ruler_name = GIVEN_NAME_POOL[0];
        let names = assign_house_names(0, 1, true, ruler_name).expect("names");
        assert_eq!(names[2], ruler_name);
        let set: BTreeSet<&str> = names.iter().map(String::as_str).collect();
        assert_eq!(
            set.len(),
            PERSONS_PER_HOUSE,
            "in-house names must be unique"
        );
    }

    /// member 슬롯별 세대와 head==member_ids[2] 고정 규칙을 검증한다.
    #[test]
    fn member_slot_generations_and_head_is_member_2() {
        let d = generate_dynastic_world(1).expect("dynastic");
        let by_id: BTreeMap<_, _> = d
            .population
            .persons
            .iter()
            .map(|p| (p.id.as_str(), p))
            .collect();
        for house in &d.population.houses {
            assert_eq!(house.head_person_id, house.member_ids[2]);
            for (m, mid) in house.member_ids.iter().enumerate() {
                assert_eq!(
                    by_id[mid.as_str()].generation,
                    generation_for_member(m),
                    "house {} member {m}",
                    house.id
                );
            }
        }
    }

    /// validate_population이 잘못된 member 슬롯 세대를 거부한다.
    #[test]
    fn validate_rejects_wrong_member_slot_generation() {
        let mut d = generate_dynastic_world(1).expect("dynastic");
        // house-01 member 0을 Current로 바꿔 슬롯 규칙 위반
        let mid0 = d.population.houses[0].member_ids[0].clone();
        let p = d
            .population
            .persons
            .iter_mut()
            .find(|p| p.id == mid0)
            .expect("person");
        p.generation = GenerationBand::Current;
        // 세대 총계를 맞추기 위해 member 2를 Elder로 (슬롯 규칙은 여전히 깨짐)
        let mid2 = d.population.houses[0].member_ids[2].clone();
        let p2 = d
            .population
            .persons
            .iter_mut()
            .find(|p| p.id == mid2)
            .expect("person");
        p2.generation = GenerationBand::Elder;

        let err = validate_population(&d.world, &d.population).expect_err("must fail");
        let msg = err.to_string();
        assert!(
            msg.contains("member_ids[") || msg.contains("generation"),
            "unexpected err: {msg}"
        );
    }
}
