// M1.4 초기 정치 맥락 생성·검증 — RNG 없이 기존 stable ID에서 파생

use crate::context::{
    CONTEXT_WORLD_SCHEMA_VERSION, CROSS_REALM_RELATION_COUNT, CULTURE_AMBER, CULTURE_COUNT,
    CULTURE_RIVER, CULTURE_STONE, ContextWorld, Culture, FAITH_ANCESTRAL, FAITH_SOLAR,
    HouseIdentity, HouseRelation, HouseRelationKind, INFORMATION_COUNT, INTRA_REALM_RELATION_COUNT,
    InformationConfidence, InformationItem, InformationScope, InformationTopic,
    InitialPoliticalContext, PRIVATE_CONFIRMED_INFORMATION_COUNT,
    PRIVATE_UNVERIFIED_INFORMATION_COUNT, PROMISE_COUNT, PUBLIC_CONFIRMED_INFORMATION_COUNT,
    PersonIdentity, Promise, RELATION_COUNT, RELIGION_COUNT, RealmIdentity, Religion,
};
use crate::error::CoreError;
use crate::political::PoliticalWorld;
use crate::politicalgen::{generate_political_world, validate_political_roster};
use crate::population::{HOUSE_COUNT, HOUSES_PER_REALM, PERSON_COUNT, PERSONS_PER_HOUSE};
use crate::populationgen::validate_population;
use crate::world::WORLD_REALM_COUNT;
use crate::worldgen::validate_world;
use std::collections::{BTreeMap, BTreeSet};

/// 고정 문화 fixture (생성기 없음).
fn fixed_cultures() -> Vec<Culture> {
    vec![
        Culture {
            id: CULTURE_AMBER.to_string(),
            name: "Amber".to_string(),
        },
        Culture {
            id: CULTURE_RIVER.to_string(),
            name: "River".to_string(),
        },
        Culture {
            id: CULTURE_STONE.to_string(),
            name: "Stone".to_string(),
        },
    ]
}

/// 고정 종교 fixture (religion_id 오름차순).
fn fixed_religions() -> Vec<Religion> {
    vec![
        Religion {
            id: FAITH_ANCESTRAL.to_string(),
            name: "Ancestral Faith".to_string(),
        },
        Religion {
            id: FAITH_SOLAR.to_string(),
            name: "Solar Faith".to_string(),
        },
    ]
}

/// realm majority culture (realm 정규 순서 index 0..5).
fn majority_culture_for_realm_index(index: usize) -> &'static str {
    match index {
        0 | 1 => CULTURE_AMBER,
        2 | 3 => CULTURE_RIVER,
        4 | 5 => CULTURE_STONE,
        _ => CULTURE_AMBER,
    }
}

/// realm majority religion (realm 정규 순서 index 0..5).
fn majority_religion_for_realm_index(index: usize) -> &'static str {
    if index.is_multiple_of(2) {
        FAITH_SOLAR
    } else {
        FAITH_ANCESTRAL
    }
}

/// 반대 종교.
fn opposite_religion(religion_id: &str) -> &'static str {
    if religion_id == FAITH_SOLAR {
        FAITH_ANCESTRAL
    } else {
        FAITH_SOLAR
    }
}

/// 문화 순환: amber → river → stone → amber.
fn next_culture(culture_id: &str) -> &'static str {
    if culture_id == CULTURE_AMBER {
        CULTURE_RIVER
    } else if culture_id == CULTURE_RIVER {
        CULTURE_STONE
    } else {
        CULTURE_AMBER
    }
}

/// house pair를 `a < b` 로 canonicalize.
fn canonicalize_pair(a: &str, b: &str) -> (String, String) {
    if a < b {
        (a.to_string(), b.to_string())
    } else {
        (b.to_string(), a.to_string())
    }
}

/// realm별 house를 ID 오름차순으로 묶는다 (local[0]=ruling).
fn houses_by_realm_sorted(
    political: &PoliticalWorld,
) -> Result<BTreeMap<String, Vec<&crate::population::House>>, CoreError> {
    let mut map: BTreeMap<String, Vec<&crate::population::House>> = BTreeMap::new();
    for house in &political.dynastic.population.houses {
        map.entry(house.realm_id.clone()).or_default().push(house);
    }
    for (realm_id, houses) in map.iter_mut() {
        houses.sort_by(|a, b| a.id.cmp(&b.id));
        if houses.len() != HOUSES_PER_REALM {
            return Err(CoreError::InvalidContext(format!(
                "realm {realm_id} houses {} != {HOUSES_PER_REALM}",
                houses.len()
            )));
        }
    }
    if map.len() != WORLD_REALM_COUNT {
        return Err(CoreError::InvalidContext(format!(
            "realm house groups {} != {WORLD_REALM_COUNT}",
            map.len()
        )));
    }
    Ok(map)
}

/// PoliticalWorld에서 초기 정치 맥락을 파생한다. RNG를 사용하지 않는다.
pub fn derive_initial_context(
    political: &PoliticalWorld,
) -> Result<InitialPoliticalContext, CoreError> {
    // 깨진 입력은 panic 없이 fail closed.
    validate_world(&political.dynastic.world).map_err(|e| match e {
        CoreError::InvalidWorld(msg) => CoreError::InvalidContext(format!("world: {msg}")),
        other => other,
    })?;
    validate_population(&political.dynastic.world, &political.dynastic.population).map_err(
        |e| match e {
            CoreError::InvalidPopulation(msg) => {
                CoreError::InvalidContext(format!("population: {msg}"))
            }
            other => other,
        },
    )?;
    validate_political_roster(&political.dynastic, &political.roster).map_err(|e| match e {
        CoreError::InvalidPolitical(msg) => CoreError::InvalidContext(format!("political: {msg}")),
        other => other,
    })?;

    let pop = &political.dynastic.population;
    let person_by_id: BTreeMap<&str, &crate::population::Person> =
        pop.persons.iter().map(|p| (p.id.as_str(), p)).collect();

    let cultures = fixed_cultures();
    let religions = fixed_religions();

    // Realm identities — realm ID 정규 순서
    let mut realms: Vec<_> = political.dynastic.world.realms.iter().collect();
    realms.sort_by(|a, b| a.id.cmp(&b.id));
    if realms.len() != WORLD_REALM_COUNT {
        return Err(CoreError::InvalidContext(format!(
            "realm count {} != {WORLD_REALM_COUNT}",
            realms.len()
        )));
    }

    let mut realm_identities: Vec<RealmIdentity> = Vec::with_capacity(WORLD_REALM_COUNT);
    for (idx, realm) in realms.iter().enumerate() {
        realm_identities.push(RealmIdentity {
            realm_id: realm.id.clone(),
            majority_culture_id: majority_culture_for_realm_index(idx).to_string(),
            majority_religion_id: majority_religion_for_realm_index(idx).to_string(),
        });
    }
    realm_identities.sort_by(|a, b| a.realm_id.cmp(&b.realm_id));

    let realm_identity_by_id: BTreeMap<&str, &RealmIdentity> = realm_identities
        .iter()
        .map(|r| (r.realm_id.as_str(), r))
        .collect();

    let houses_by_realm = houses_by_realm_sorted(political)?;

    // House identities
    let mut house_identities: Vec<HouseIdentity> = Vec::with_capacity(HOUSE_COUNT);
    for realm in &realms {
        let rid = realm.id.as_str();
        let realm_id = realm_identity_by_id.get(rid).ok_or_else(|| {
            CoreError::InvalidContext(format!("missing realm identity for {rid}"))
        })?;
        let houses = houses_by_realm
            .get(rid)
            .ok_or_else(|| CoreError::InvalidContext(format!("missing houses for realm {rid}")))?;
        let maj_c = realm_id.majority_culture_id.as_str();
        let maj_r = realm_id.majority_religion_id.as_str();
        for (local_idx, house) in houses.iter().enumerate() {
            let (culture_id, religion_id) = match local_idx {
                0 => (maj_c.to_string(), maj_r.to_string()),
                1 => (maj_c.to_string(), opposite_religion(maj_r).to_string()),
                2 => (next_culture(maj_c).to_string(), maj_r.to_string()),
                _ => {
                    return Err(CoreError::InvalidContext(format!(
                        "unexpected local house index {local_idx} in realm {rid}"
                    )));
                }
            };
            house_identities.push(HouseIdentity {
                house_id: house.id.clone(),
                culture_id,
                religion_id,
            });
        }
    }
    house_identities.sort_by(|a, b| a.house_id.cmp(&b.house_id));

    let house_identity_by_id: BTreeMap<&str, &HouseIdentity> = house_identities
        .iter()
        .map(|h| (h.house_id.as_str(), h))
        .collect();

    // Person identities — House inheritance
    let mut person_identities: Vec<PersonIdentity> = Vec::with_capacity(PERSON_COUNT);
    for person in &pop.persons {
        let house_id = house_identity_by_id
            .get(person.house_id.as_str())
            .ok_or_else(|| {
                CoreError::InvalidContext(format!(
                    "person {} house {} missing identity",
                    person.id, person.house_id
                ))
            })?;
        person_identities.push(PersonIdentity {
            person_id: person.id.clone(),
            culture_id: house_id.culture_id.clone(),
            religion_id: house_id.religion_id.clone(),
        });
    }
    person_identities.sort_by(|a, b| a.person_id.cmp(&b.person_id));

    // Relations — intra-realm 18 + cross-realm 6
    let mut relations: Vec<HouseRelation> = Vec::with_capacity(RELATION_COUNT);
    let mut relation_pairs: BTreeSet<(String, String)> = BTreeSet::new();

    for realm in &realms {
        let houses = houses_by_realm.get(realm.id.as_str()).ok_or_else(|| {
            CoreError::InvalidContext(format!("missing houses for realm {}", realm.id))
        })?;
        let h0 = houses
            .first()
            .ok_or_else(|| CoreError::InvalidContext("empty houses for realm".to_string()))?;
        let h1 = houses.get(1).ok_or_else(|| {
            CoreError::InvalidContext(format!("realm {} missing local[1] house", realm.id))
        })?;
        let h2 = houses.get(2).ok_or_else(|| {
            CoreError::InvalidContext(format!("realm {} missing local[2] house", realm.id))
        })?;

        let triples = [
            (h0.id.as_str(), h1.id.as_str(), HouseRelationKind::Rival),
            (
                h0.id.as_str(),
                h2.id.as_str(),
                HouseRelationKind::Cooperative,
            ),
            (
                h1.id.as_str(),
                h2.id.as_str(),
                HouseRelationKind::Competitive,
            ),
        ];
        for (a, b, kind) in triples {
            let (house_a_id, house_b_id) = canonicalize_pair(a, b);
            if !relation_pairs.insert((house_a_id.clone(), house_b_id.clone())) {
                return Err(CoreError::InvalidContext(format!(
                    "duplicate relation pair {house_a_id}-{house_b_id}"
                )));
            }
            relations.push(HouseRelation {
                house_a_id,
                house_b_id,
                kind,
            });
        }
    }

    // Cross-realm ruling house ring (all Cooperative)
    let ruling_houses: Vec<&crate::population::House> = realms
        .iter()
        .map(|r| {
            houses_by_realm
                .get(r.id.as_str())
                .and_then(|hs| hs.first().copied())
                .ok_or_else(|| {
                    CoreError::InvalidContext(format!("missing ruling house for {}", r.id))
                })
        })
        .collect::<Result<Vec<_>, _>>()?;

    for i in 0..ruling_houses.len() {
        let a = ruling_houses
            .get(i)
            .ok_or_else(|| CoreError::InvalidContext(format!("ruling house index {i} missing")))?;
        let b = ruling_houses
            .get((i + 1) % ruling_houses.len())
            .ok_or_else(|| {
                CoreError::InvalidContext(format!("ruling house ring neighbor of {i} missing"))
            })?;
        let (house_a_id, house_b_id) = canonicalize_pair(a.id.as_str(), b.id.as_str());
        if !relation_pairs.insert((house_a_id.clone(), house_b_id.clone())) {
            return Err(CoreError::InvalidContext(format!(
                "duplicate cross relation pair {house_a_id}-{house_b_id}"
            )));
        }
        relations.push(HouseRelation {
            house_a_id,
            house_b_id,
            kind: HouseRelationKind::Cooperative,
        });
    }

    relations.sort_by(|a, b| {
        a.house_a_id
            .cmp(&b.house_a_id)
            .then_with(|| a.house_b_id.cmp(&b.house_b_id))
    });

    // Promises — ruler → two non-ruling heads, same reward_key per realm
    let mut promises: Vec<Promise> = Vec::with_capacity(PROMISE_COUNT);
    let mut promise_seq = 1usize;
    for realm in &realms {
        let houses = houses_by_realm.get(realm.id.as_str()).ok_or_else(|| {
            CoreError::InvalidContext(format!("missing houses for realm {}", realm.id))
        })?;
        let ruling = houses
            .first()
            .ok_or_else(|| CoreError::InvalidContext("empty houses for realm".to_string()))?;
        let h1 = houses.get(1).ok_or_else(|| {
            CoreError::InvalidContext(format!("realm {} missing local[1]", realm.id))
        })?;
        let h2 = houses.get(2).ok_or_else(|| {
            CoreError::InvalidContext(format!("realm {} missing local[2]", realm.id))
        })?;

        // promisor = ruling house head (ruler person)
        let promisor = ruling.head_person_id.clone();
        if !person_by_id.contains_key(promisor.as_str()) {
            return Err(CoreError::InvalidContext(format!(
                "promisor {promisor} missing"
            )));
        }
        let reward_key = format!("reward:{}:council-seat", realm.id);

        for promisee_house in [h1, h2] {
            let promisee = promisee_house.head_person_id.clone();
            if !person_by_id.contains_key(promisee.as_str()) {
                return Err(CoreError::InvalidContext(format!(
                    "promisee {promisee} missing"
                )));
            }
            let mut known_by = vec![promisor.clone(), promisee.clone()];
            known_by.sort();
            promises.push(Promise {
                id: format!("promise-{promise_seq:02}"),
                realm_id: realm.id.clone(),
                promisor_person_id: promisor.clone(),
                promisee_person_id: promisee,
                reward_key: reward_key.clone(),
                known_by_person_ids: known_by,
            });
            promise_seq += 1;
        }
    }
    promises.sort_by(|a, b| a.id.cmp(&b.id));

    // Information items
    // 1..6 public confirmed ReligiousMinority
    // 7..12 private confirmed PromiseConflict
    // 13..18 private unverified PromiseConflict
    let mut information: Vec<InformationItem> = Vec::with_capacity(INFORMATION_COUNT);
    let mut info_seq = 1usize;

    // promises by realm (sorted by id) for subject references
    let mut promises_by_realm: BTreeMap<String, Vec<&Promise>> = BTreeMap::new();
    for p in &promises {
        promises_by_realm
            .entry(p.realm_id.clone())
            .or_default()
            .push(p);
    }
    for list in promises_by_realm.values_mut() {
        list.sort_by(|a, b| a.id.cmp(&b.id));
    }

    // Public confirmed ReligiousMinority
    for realm in &realms {
        let houses = houses_by_realm.get(realm.id.as_str()).ok_or_else(|| {
            CoreError::InvalidContext(format!("missing houses for realm {}", realm.id))
        })?;
        let minority_house = houses.get(1).ok_or_else(|| {
            CoreError::InvalidContext(format!(
                "realm {} missing religious minority house",
                realm.id
            ))
        })?;
        let mut subject_ids = vec![minority_house.id.clone()];
        subject_ids.sort();
        information.push(InformationItem {
            id: format!("information-{info_seq:02}"),
            realm_id: realm.id.clone(),
            topic: InformationTopic::ReligiousMinority,
            scope: InformationScope::Public,
            confidence: InformationConfidence::Confirmed,
            subject_ids,
            known_by_person_ids: vec![],
        });
        info_seq += 1;
    }

    // Private confirmed PromiseConflict — ruler + ruling member_ids[3]
    for realm in &realms {
        let houses = houses_by_realm.get(realm.id.as_str()).ok_or_else(|| {
            CoreError::InvalidContext(format!("missing houses for realm {}", realm.id))
        })?;
        let ruling = houses
            .first()
            .ok_or_else(|| CoreError::InvalidContext("empty houses".to_string()))?;
        let ruler_id = ruling.head_person_id.clone();
        let rhc_id = ruling.member_ids.get(3).ok_or_else(|| {
            CoreError::InvalidContext(format!("ruling house {} missing member_ids[3]", ruling.id))
        })?;
        if !person_by_id.contains_key(rhc_id.as_str()) {
            return Err(CoreError::InvalidContext(format!(
                "ruling house current {rhc_id} missing"
            )));
        }
        let realm_promises = promises_by_realm.get(realm.id.as_str()).ok_or_else(|| {
            CoreError::InvalidContext(format!("no promises for realm {}", realm.id))
        })?;
        if realm_promises.len() != 2 {
            return Err(CoreError::InvalidContext(format!(
                "realm {} promise count {} != 2",
                realm.id,
                realm_promises.len()
            )));
        }
        let mut subject_ids: Vec<String> = realm_promises.iter().map(|p| p.id.clone()).collect();
        subject_ids.sort();
        let mut known_by = vec![ruler_id, rhc_id.clone()];
        known_by.sort();
        information.push(InformationItem {
            id: format!("information-{info_seq:02}"),
            realm_id: realm.id.clone(),
            topic: InformationTopic::PromiseConflict,
            scope: InformationScope::Private,
            confidence: InformationConfidence::Confirmed,
            subject_ids,
            known_by_person_ids: known_by,
        });
        info_seq += 1;
    }

    // Private unverified PromiseConflict — local[1] head only
    for realm in &realms {
        let houses = houses_by_realm.get(realm.id.as_str()).ok_or_else(|| {
            CoreError::InvalidContext(format!("missing houses for realm {}", realm.id))
        })?;
        let h1 = houses.get(1).ok_or_else(|| {
            CoreError::InvalidContext(format!("realm {} missing local[1]", realm.id))
        })?;
        let head1 = h1.head_person_id.clone();
        if !person_by_id.contains_key(head1.as_str()) {
            return Err(CoreError::InvalidContext(format!(
                "local[1] head {head1} missing"
            )));
        }
        let realm_promises = promises_by_realm.get(realm.id.as_str()).ok_or_else(|| {
            CoreError::InvalidContext(format!("no promises for realm {}", realm.id))
        })?;
        let mut subject_ids: Vec<String> = realm_promises.iter().map(|p| p.id.clone()).collect();
        subject_ids.sort();
        let mut known_by = vec![head1];
        known_by.sort();
        information.push(InformationItem {
            id: format!("information-{info_seq:02}"),
            realm_id: realm.id.clone(),
            topic: InformationTopic::PromiseConflict,
            scope: InformationScope::Private,
            confidence: InformationConfidence::Unverified,
            subject_ids,
            known_by_person_ids: known_by,
        });
        info_seq += 1;
    }

    information.sort_by(|a, b| a.id.cmp(&b.id));

    let context = InitialPoliticalContext {
        cultures,
        religions,
        realm_identities,
        house_identities,
        person_identities,
        relations,
        promises,
        information,
    };
    validate_initial_context(political, &context)?;
    Ok(context)
}

/// seed에서 ContextWorld를 생성한다.
pub fn generate_context_world(seed: u64) -> Result<ContextWorld, CoreError> {
    let political = generate_political_world(seed)?;
    let context = derive_initial_context(&political)?;
    Ok(ContextWorld {
        schema_version: CONTEXT_WORLD_SCHEMA_VERSION,
        seed,
        political,
        context,
    })
}

/// 초기 정치 맥락 불변식을 검사한다. 실패 시 fail closed.
pub fn validate_initial_context(
    political: &PoliticalWorld,
    context: &InitialPoliticalContext,
) -> Result<(), CoreError> {
    validate_world(&political.dynastic.world).map_err(|e| match e {
        CoreError::InvalidWorld(msg) => CoreError::InvalidContext(format!("world: {msg}")),
        other => other,
    })?;
    validate_population(&political.dynastic.world, &political.dynastic.population).map_err(
        |e| match e {
            CoreError::InvalidPopulation(msg) => {
                CoreError::InvalidContext(format!("population: {msg}"))
            }
            other => other,
        },
    )?;
    validate_political_roster(&political.dynastic, &political.roster).map_err(|e| match e {
        CoreError::InvalidPolitical(msg) => CoreError::InvalidContext(format!("political: {msg}")),
        other => other,
    })?;

    let pop = &political.dynastic.population;
    let person_ids: BTreeSet<&str> = pop.persons.iter().map(|p| p.id.as_str()).collect();
    let house_ids: BTreeSet<&str> = pop.houses.iter().map(|h| h.id.as_str()).collect();
    let realm_ids: BTreeSet<&str> = political
        .dynastic
        .world
        .realms
        .iter()
        .map(|r| r.id.as_str())
        .collect();

    // Cultures
    if context.cultures.len() != CULTURE_COUNT {
        return Err(CoreError::InvalidContext(format!(
            "cultures {} != {CULTURE_COUNT}",
            context.cultures.len()
        )));
    }
    let expected_cultures = [CULTURE_AMBER, CULTURE_RIVER, CULTURE_STONE];
    for (i, c) in context.cultures.iter().enumerate() {
        if c.id != expected_cultures[i] {
            return Err(CoreError::InvalidContext(format!(
                "culture[{i}] id {} != {}",
                c.id, expected_cultures[i]
            )));
        }
    }
    // sorted by id
    for w in context.cultures.windows(2) {
        if w[0].id >= w[1].id {
            return Err(CoreError::InvalidContext(
                "cultures not sorted by id".to_string(),
            ));
        }
    }

    // Religions
    if context.religions.len() != RELIGION_COUNT {
        return Err(CoreError::InvalidContext(format!(
            "religions {} != {RELIGION_COUNT}",
            context.religions.len()
        )));
    }
    let expected_religions = [FAITH_ANCESTRAL, FAITH_SOLAR];
    for (i, r) in context.religions.iter().enumerate() {
        if r.id != expected_religions[i] {
            return Err(CoreError::InvalidContext(format!(
                "religion[{i}] id {} != {}",
                r.id, expected_religions[i]
            )));
        }
    }
    for w in context.religions.windows(2) {
        if w[0].id >= w[1].id {
            return Err(CoreError::InvalidContext(
                "religions not sorted by id".to_string(),
            ));
        }
    }

    // Realm identities
    if context.realm_identities.len() != WORLD_REALM_COUNT {
        return Err(CoreError::InvalidContext(format!(
            "realm_identities {} != {WORLD_REALM_COUNT}",
            context.realm_identities.len()
        )));
    }
    for w in context.realm_identities.windows(2) {
        if w[0].realm_id >= w[1].realm_id {
            return Err(CoreError::InvalidContext(
                "realm_identities not sorted by realm_id".to_string(),
            ));
        }
    }
    let culture_set: BTreeSet<&str> = context.cultures.iter().map(|c| c.id.as_str()).collect();
    let religion_set: BTreeSet<&str> = context.religions.iter().map(|r| r.id.as_str()).collect();
    let mut realm_identity_by_id: BTreeMap<&str, &RealmIdentity> = BTreeMap::new();
    for (idx, ri) in context.realm_identities.iter().enumerate() {
        if !realm_ids.contains(ri.realm_id.as_str()) {
            return Err(CoreError::InvalidContext(format!(
                "realm identity {} unknown realm",
                ri.realm_id
            )));
        }
        if !culture_set.contains(ri.majority_culture_id.as_str()) {
            return Err(CoreError::InvalidContext(format!(
                "realm {} unknown culture {}",
                ri.realm_id, ri.majority_culture_id
            )));
        }
        if !religion_set.contains(ri.majority_religion_id.as_str()) {
            return Err(CoreError::InvalidContext(format!(
                "realm {} unknown religion {}",
                ri.realm_id, ri.majority_religion_id
            )));
        }
        if majority_culture_for_realm_index(idx) != ri.majority_culture_id.as_str() {
            return Err(CoreError::InvalidContext(format!(
                "realm {} culture assignment mismatch",
                ri.realm_id
            )));
        }
        if majority_religion_for_realm_index(idx) != ri.majority_religion_id.as_str() {
            return Err(CoreError::InvalidContext(format!(
                "realm {} religion assignment mismatch",
                ri.realm_id
            )));
        }
        if realm_identity_by_id
            .insert(ri.realm_id.as_str(), ri)
            .is_some()
        {
            return Err(CoreError::InvalidContext(format!(
                "duplicate realm identity {}",
                ri.realm_id
            )));
        }
    }

    // House identities
    if context.house_identities.len() != HOUSE_COUNT {
        return Err(CoreError::InvalidContext(format!(
            "house_identities {} != {HOUSE_COUNT}",
            context.house_identities.len()
        )));
    }
    for w in context.house_identities.windows(2) {
        if w[0].house_id >= w[1].house_id {
            return Err(CoreError::InvalidContext(
                "house_identities not sorted by house_id".to_string(),
            ));
        }
    }
    let mut house_identity_by_id: BTreeMap<&str, &HouseIdentity> = BTreeMap::new();
    let mut culture_house_counts: BTreeMap<&str, usize> = BTreeMap::new();
    let mut religion_house_counts: BTreeMap<&str, usize> = BTreeMap::new();
    for hi in &context.house_identities {
        if !house_ids.contains(hi.house_id.as_str()) {
            return Err(CoreError::InvalidContext(format!(
                "house identity {} unknown house",
                hi.house_id
            )));
        }
        if !culture_set.contains(hi.culture_id.as_str()) {
            return Err(CoreError::InvalidContext(format!(
                "house {} unknown culture {}",
                hi.house_id, hi.culture_id
            )));
        }
        if !religion_set.contains(hi.religion_id.as_str()) {
            return Err(CoreError::InvalidContext(format!(
                "house {} unknown religion {}",
                hi.house_id, hi.religion_id
            )));
        }
        if house_identity_by_id
            .insert(hi.house_id.as_str(), hi)
            .is_some()
        {
            return Err(CoreError::InvalidContext(format!(
                "duplicate house identity {}",
                hi.house_id
            )));
        }
        *culture_house_counts
            .entry(hi.culture_id.as_str())
            .or_insert(0) += 1;
        *religion_house_counts
            .entry(hi.religion_id.as_str())
            .or_insert(0) += 1;
    }
    for c in &expected_cultures {
        if *culture_house_counts.get(c).unwrap_or(&0) != 6 {
            return Err(CoreError::InvalidContext(format!(
                "house culture {c} count {} != 6",
                culture_house_counts.get(c).unwrap_or(&0)
            )));
        }
    }
    for r in &expected_religions {
        if *religion_house_counts.get(r).unwrap_or(&0) != 9 {
            return Err(CoreError::InvalidContext(format!(
                "house religion {r} count {} != 9",
                religion_house_counts.get(r).unwrap_or(&0)
            )));
        }
    }

    // Verify house assignment rules per realm
    let houses_by_realm = houses_by_realm_sorted(political)?;
    for (realm_id, houses) in &houses_by_realm {
        let ri = realm_identity_by_id.get(realm_id.as_str()).ok_or_else(|| {
            CoreError::InvalidContext(format!("missing realm identity {realm_id}"))
        })?;
        let maj_c = ri.majority_culture_id.as_str();
        let maj_r = ri.majority_religion_id.as_str();
        let h0 = houses
            .first()
            .ok_or_else(|| CoreError::InvalidContext("empty houses".to_string()))?;
        let h1 = houses
            .get(1)
            .ok_or_else(|| CoreError::InvalidContext("missing local[1]".to_string()))?;
        let h2 = houses
            .get(2)
            .ok_or_else(|| CoreError::InvalidContext("missing local[2]".to_string()))?;
        let i0 = house_identity_by_id.get(h0.id.as_str()).ok_or_else(|| {
            CoreError::InvalidContext(format!("missing house identity {}", h0.id))
        })?;
        let i1 = house_identity_by_id.get(h1.id.as_str()).ok_or_else(|| {
            CoreError::InvalidContext(format!("missing house identity {}", h1.id))
        })?;
        let i2 = house_identity_by_id.get(h2.id.as_str()).ok_or_else(|| {
            CoreError::InvalidContext(format!("missing house identity {}", h2.id))
        })?;
        if i0.culture_id != maj_c || i0.religion_id != maj_r {
            return Err(CoreError::InvalidContext(format!(
                "ruling house {} identity mismatch",
                h0.id
            )));
        }
        if i1.culture_id != maj_c || i1.religion_id != opposite_religion(maj_r) {
            return Err(CoreError::InvalidContext(format!(
                "religious minority house {} identity mismatch",
                h1.id
            )));
        }
        if i2.culture_id != next_culture(maj_c) || i2.religion_id != maj_r {
            return Err(CoreError::InvalidContext(format!(
                "cultural minority house {} identity mismatch",
                h2.id
            )));
        }
        // realm must have both culture and religion minorities
        if i0.culture_id == i2.culture_id {
            return Err(CoreError::InvalidContext(format!(
                "realm {realm_id} missing cultural minority"
            )));
        }
        if i0.religion_id == i1.religion_id {
            return Err(CoreError::InvalidContext(format!(
                "realm {realm_id} missing religious minority"
            )));
        }
    }

    // Person identities
    if context.person_identities.len() != PERSON_COUNT {
        return Err(CoreError::InvalidContext(format!(
            "person_identities {} != {PERSON_COUNT}",
            context.person_identities.len()
        )));
    }
    for w in context.person_identities.windows(2) {
        if w[0].person_id >= w[1].person_id {
            return Err(CoreError::InvalidContext(
                "person_identities not sorted by person_id".to_string(),
            ));
        }
    }
    let mut culture_person_counts: BTreeMap<&str, usize> = BTreeMap::new();
    let mut religion_person_counts: BTreeMap<&str, usize> = BTreeMap::new();
    let mut person_identity_ids: BTreeSet<&str> = BTreeSet::new();
    let house_of_person: BTreeMap<&str, &str> = pop
        .persons
        .iter()
        .map(|p| (p.id.as_str(), p.house_id.as_str()))
        .collect();
    for pi in &context.person_identities {
        if !person_ids.contains(pi.person_id.as_str()) {
            return Err(CoreError::InvalidContext(format!(
                "person identity {} unknown person",
                pi.person_id
            )));
        }
        if !person_identity_ids.insert(pi.person_id.as_str()) {
            return Err(CoreError::InvalidContext(format!(
                "duplicate person identity {}",
                pi.person_id
            )));
        }
        let hid = house_of_person.get(pi.person_id.as_str()).ok_or_else(|| {
            CoreError::InvalidContext(format!("person {} house missing", pi.person_id))
        })?;
        let hi = house_identity_by_id.get(hid).ok_or_else(|| {
            CoreError::InvalidContext(format!("person {} house identity missing", pi.person_id))
        })?;
        if pi.culture_id != hi.culture_id || pi.religion_id != hi.religion_id {
            return Err(CoreError::InvalidContext(format!(
                "person {} identity does not inherit house {}",
                pi.person_id, hid
            )));
        }
        *culture_person_counts
            .entry(pi.culture_id.as_str())
            .or_insert(0) += 1;
        *religion_person_counts
            .entry(pi.religion_id.as_str())
            .or_insert(0) += 1;
    }
    if person_identity_ids.len() != PERSON_COUNT {
        return Err(CoreError::InvalidContext(
            "person identity coverage incomplete".to_string(),
        ));
    }
    for c in &expected_cultures {
        if *culture_person_counts.get(c).unwrap_or(&0) != 48 {
            return Err(CoreError::InvalidContext(format!(
                "person culture {c} count {} != 48",
                culture_person_counts.get(c).unwrap_or(&0)
            )));
        }
    }
    for r in &expected_religions {
        if *religion_person_counts.get(r).unwrap_or(&0) != 72 {
            return Err(CoreError::InvalidContext(format!(
                "person religion {r} count {} != 72",
                religion_person_counts.get(r).unwrap_or(&0)
            )));
        }
    }

    // Relations
    if context.relations.len() != RELATION_COUNT {
        return Err(CoreError::InvalidContext(format!(
            "relations {} != {RELATION_COUNT}",
            context.relations.len()
        )));
    }
    for w in context.relations.windows(2) {
        let cmp = w[0]
            .house_a_id
            .cmp(&w[1].house_a_id)
            .then_with(|| w[0].house_b_id.cmp(&w[1].house_b_id));
        if cmp != std::cmp::Ordering::Less {
            return Err(CoreError::InvalidContext(
                "relations not sorted by (house_a_id, house_b_id)".to_string(),
            ));
        }
    }
    let mut pair_set: BTreeSet<(&str, &str)> = BTreeSet::new();
    let mut intra = 0usize;
    let mut cross = 0usize;
    let house_realm: BTreeMap<&str, &str> = pop
        .houses
        .iter()
        .map(|h| (h.id.as_str(), h.realm_id.as_str()))
        .collect();
    for rel in &context.relations {
        if rel.house_a_id >= rel.house_b_id {
            return Err(CoreError::InvalidContext(format!(
                "relation not canonical: {} >= {}",
                rel.house_a_id, rel.house_b_id
            )));
        }
        if !house_ids.contains(rel.house_a_id.as_str())
            || !house_ids.contains(rel.house_b_id.as_str())
        {
            return Err(CoreError::InvalidContext(format!(
                "relation references unknown house {}-{}",
                rel.house_a_id, rel.house_b_id
            )));
        }
        if !pair_set.insert((rel.house_a_id.as_str(), rel.house_b_id.as_str())) {
            return Err(CoreError::InvalidContext(format!(
                "duplicate relation pair {}-{}",
                rel.house_a_id, rel.house_b_id
            )));
        }
        let ra = *house_realm.get(rel.house_a_id.as_str()).unwrap_or(&"");
        let rb = *house_realm.get(rel.house_b_id.as_str()).unwrap_or(&"");
        if ra == rb {
            intra += 1;
        } else {
            cross += 1;
            if rel.kind != HouseRelationKind::Cooperative {
                return Err(CoreError::InvalidContext(format!(
                    "cross-realm relation {}-{} must be Cooperative",
                    rel.house_a_id, rel.house_b_id
                )));
            }
        }
    }
    if intra != INTRA_REALM_RELATION_COUNT {
        return Err(CoreError::InvalidContext(format!(
            "intra-realm relations {intra} != {INTRA_REALM_RELATION_COUNT}"
        )));
    }
    if cross != CROSS_REALM_RELATION_COUNT {
        return Err(CoreError::InvalidContext(format!(
            "cross-realm relations {cross} != {CROSS_REALM_RELATION_COUNT}"
        )));
    }

    // Verify intra-realm kinds
    for (realm_id, houses) in &houses_by_realm {
        let h0 = houses.first().map(|h| h.id.as_str()).unwrap_or("");
        let h1 = houses.get(1).map(|h| h.id.as_str()).unwrap_or("");
        let h2 = houses.get(2).map(|h| h.id.as_str()).unwrap_or("");
        let expected = [
            (canonicalize_pair(h0, h1), HouseRelationKind::Rival),
            (canonicalize_pair(h0, h2), HouseRelationKind::Cooperative),
            (canonicalize_pair(h1, h2), HouseRelationKind::Competitive),
        ];
        for ((a, b), kind) in &expected {
            let found = context
                .relations
                .iter()
                .find(|r| r.house_a_id == *a && r.house_b_id == *b && r.kind == *kind);
            if found.is_none() {
                return Err(CoreError::InvalidContext(format!(
                    "realm {realm_id} missing relation {a}-{b} kind {kind:?}"
                )));
            }
        }
    }

    // Promises
    if context.promises.len() != PROMISE_COUNT {
        return Err(CoreError::InvalidContext(format!(
            "promises {} != {PROMISE_COUNT}",
            context.promises.len()
        )));
    }
    for w in context.promises.windows(2) {
        if w[0].id >= w[1].id {
            return Err(CoreError::InvalidContext(
                "promises not sorted by id".to_string(),
            ));
        }
    }
    let mut promise_ids: BTreeSet<&str> = BTreeSet::new();
    let mut promises_by_realm: BTreeMap<&str, Vec<&Promise>> = BTreeMap::new();
    let mut reward_keys: BTreeSet<String> = BTreeSet::new();
    for p in &context.promises {
        if !promise_ids.insert(p.id.as_str()) {
            return Err(CoreError::InvalidContext(format!(
                "duplicate promise id {}",
                p.id
            )));
        }
        if !realm_ids.contains(p.realm_id.as_str()) {
            return Err(CoreError::InvalidContext(format!(
                "promise {} unknown realm {}",
                p.id, p.realm_id
            )));
        }
        if !person_ids.contains(p.promisor_person_id.as_str())
            || !person_ids.contains(p.promisee_person_id.as_str())
        {
            return Err(CoreError::InvalidContext(format!(
                "promise {} references unknown person",
                p.id
            )));
        }
        // known_by exactly promisor + promisee, sorted, unique
        if p.known_by_person_ids.len() != 2 {
            return Err(CoreError::InvalidContext(format!(
                "promise {} known_by len {} != 2",
                p.id,
                p.known_by_person_ids.len()
            )));
        }
        let mut expected_known = vec![p.promisor_person_id.as_str(), p.promisee_person_id.as_str()];
        expected_known.sort();
        let actual: Vec<&str> = p.known_by_person_ids.iter().map(|s| s.as_str()).collect();
        if actual != expected_known {
            return Err(CoreError::InvalidContext(format!(
                "promise {} known_by mismatch",
                p.id
            )));
        }
        for w in p.known_by_person_ids.windows(2) {
            if w[0] >= w[1] {
                return Err(CoreError::InvalidContext(format!(
                    "promise {} known_by not sorted unique",
                    p.id
                )));
            }
        }
        promises_by_realm
            .entry(p.realm_id.as_str())
            .or_default()
            .push(p);
    }

    for (realm_id, houses) in &houses_by_realm {
        let plist = promises_by_realm.get(realm_id.as_str()).ok_or_else(|| {
            CoreError::InvalidContext(format!("realm {realm_id} has no promises"))
        })?;
        if plist.len() != 2 {
            return Err(CoreError::InvalidContext(format!(
                "realm {realm_id} promises {} != 2",
                plist.len()
            )));
        }
        let ruling = houses
            .first()
            .ok_or_else(|| CoreError::InvalidContext("empty houses".to_string()))?;
        let h1 = houses
            .get(1)
            .ok_or_else(|| CoreError::InvalidContext("missing local[1]".to_string()))?;
        let h2 = houses
            .get(2)
            .ok_or_else(|| CoreError::InvalidContext("missing local[2]".to_string()))?;
        let expected_promisor = ruling.head_person_id.as_str();
        let mut promisees: BTreeSet<&str> = BTreeSet::new();
        let mut reward: Option<&str> = None;
        for p in plist {
            if p.promisor_person_id != expected_promisor {
                return Err(CoreError::InvalidContext(format!(
                    "realm {realm_id} promisor {} != ruler {}",
                    p.promisor_person_id, expected_promisor
                )));
            }
            if !promisees.insert(p.promisee_person_id.as_str()) {
                return Err(CoreError::InvalidContext(format!(
                    "realm {realm_id} duplicate promisee"
                )));
            }
            match reward {
                None => reward = Some(p.reward_key.as_str()),
                Some(r) if r == p.reward_key.as_str() => {}
                Some(_) => {
                    return Err(CoreError::InvalidContext(format!(
                        "realm {realm_id} promises have different reward_key"
                    )));
                }
            }
        }
        let expected_key = format!("reward:{realm_id}:council-seat");
        if reward != Some(expected_key.as_str()) {
            return Err(CoreError::InvalidContext(format!(
                "realm {realm_id} reward_key mismatch"
            )));
        }
        if !reward_keys.insert(expected_key.clone()) {
            return Err(CoreError::InvalidContext(format!(
                "duplicate reward_key across realms: {expected_key}"
            )));
        }
        let expected_promisees: BTreeSet<&str> =
            [h1.head_person_id.as_str(), h2.head_person_id.as_str()]
                .into_iter()
                .collect();
        if promisees != expected_promisees {
            return Err(CoreError::InvalidContext(format!(
                "realm {realm_id} promisees are not the two non-ruling heads"
            )));
        }
    }

    // Information
    if context.information.len() != INFORMATION_COUNT {
        return Err(CoreError::InvalidContext(format!(
            "information {} != {INFORMATION_COUNT}",
            context.information.len()
        )));
    }
    for w in context.information.windows(2) {
        if w[0].id >= w[1].id {
            return Err(CoreError::InvalidContext(
                "information not sorted by id".to_string(),
            ));
        }
    }
    let mut info_ids: BTreeSet<&str> = BTreeSet::new();
    let mut public_confirmed = 0usize;
    let mut private_confirmed = 0usize;
    let mut private_unverified = 0usize;

    for item in &context.information {
        if !info_ids.insert(item.id.as_str()) {
            return Err(CoreError::InvalidContext(format!(
                "duplicate information id {}",
                item.id
            )));
        }
        if !realm_ids.contains(item.realm_id.as_str()) {
            return Err(CoreError::InvalidContext(format!(
                "information {} unknown realm",
                item.id
            )));
        }
        // subject_ids sorted unique
        for w in item.subject_ids.windows(2) {
            if w[0] >= w[1] {
                return Err(CoreError::InvalidContext(format!(
                    "information {} subject_ids not sorted unique",
                    item.id
                )));
            }
        }
        // known_by sorted unique
        for w in item.known_by_person_ids.windows(2) {
            if w[0] >= w[1] {
                return Err(CoreError::InvalidContext(format!(
                    "information {} known_by not sorted unique",
                    item.id
                )));
            }
        }
        for kid in &item.known_by_person_ids {
            if !person_ids.contains(kid.as_str()) {
                return Err(CoreError::InvalidContext(format!(
                    "information {} knower {kid} missing",
                    item.id
                )));
            }
        }
        match item.scope {
            InformationScope::Public => {
                if !item.known_by_person_ids.is_empty() {
                    return Err(CoreError::InvalidContext(format!(
                        "public information {} must have empty known_by",
                        item.id
                    )));
                }
            }
            InformationScope::Private => {
                if item.known_by_person_ids.is_empty() {
                    return Err(CoreError::InvalidContext(format!(
                        "private information {} must have non-empty known_by",
                        item.id
                    )));
                }
            }
        }

        match (item.topic, item.scope, item.confidence) {
            (
                InformationTopic::ReligiousMinority,
                InformationScope::Public,
                InformationConfidence::Confirmed,
            ) => {
                public_confirmed += 1;
                if item.subject_ids.len() != 1 {
                    return Err(CoreError::InvalidContext(format!(
                        "information {} ReligiousMinority subject count",
                        item.id
                    )));
                }
                let sid = item.subject_ids[0].as_str();
                if !house_ids.contains(sid) {
                    return Err(CoreError::InvalidContext(format!(
                        "information {} subject house {sid} missing",
                        item.id
                    )));
                }
                // must be local[1] of realm
                let houses = houses_by_realm.get(item.realm_id.as_str()).ok_or_else(|| {
                    CoreError::InvalidContext(format!(
                        "information {} realm houses missing",
                        item.id
                    ))
                })?;
                let expected = houses
                    .get(1)
                    .map(|h| h.id.as_str())
                    .ok_or_else(|| CoreError::InvalidContext("missing local[1]".to_string()))?;
                if sid != expected {
                    return Err(CoreError::InvalidContext(format!(
                        "information {} subject {sid} != religious minority {expected}",
                        item.id
                    )));
                }
            }
            (
                InformationTopic::PromiseConflict,
                InformationScope::Private,
                InformationConfidence::Confirmed,
            ) => {
                private_confirmed += 1;
                validate_promise_conflict_subjects(item, &promises_by_realm)?;
                // knowers = ruler + ruling member_ids[3]
                let houses = houses_by_realm.get(item.realm_id.as_str()).ok_or_else(|| {
                    CoreError::InvalidContext(format!(
                        "information {} realm houses missing",
                        item.id
                    ))
                })?;
                let ruling = houses
                    .first()
                    .ok_or_else(|| CoreError::InvalidContext("empty houses".to_string()))?;
                let rhc = ruling.member_ids.get(3).ok_or_else(|| {
                    CoreError::InvalidContext(format!(
                        "ruling house {} missing member_ids[3]",
                        ruling.id
                    ))
                })?;
                let mut expected = vec![ruling.head_person_id.as_str(), rhc.as_str()];
                expected.sort();
                let actual: Vec<&str> = item
                    .known_by_person_ids
                    .iter()
                    .map(|s| s.as_str())
                    .collect();
                if actual != expected {
                    return Err(CoreError::InvalidContext(format!(
                        "information {} confirmed conflict known_by mismatch",
                        item.id
                    )));
                }
            }
            (
                InformationTopic::PromiseConflict,
                InformationScope::Private,
                InformationConfidence::Unverified,
            ) => {
                private_unverified += 1;
                validate_promise_conflict_subjects(item, &promises_by_realm)?;
                let houses = houses_by_realm.get(item.realm_id.as_str()).ok_or_else(|| {
                    CoreError::InvalidContext(format!(
                        "information {} realm houses missing",
                        item.id
                    ))
                })?;
                let h1 = houses
                    .get(1)
                    .ok_or_else(|| CoreError::InvalidContext("missing local[1]".to_string()))?;
                if item.known_by_person_ids != [h1.head_person_id.clone()] {
                    return Err(CoreError::InvalidContext(format!(
                        "information {} unverified known_by must be local[1] head only",
                        item.id
                    )));
                }
            }
            other => {
                return Err(CoreError::InvalidContext(format!(
                    "information {} unexpected topic/scope/confidence {:?}",
                    item.id, other
                )));
            }
        }
    }

    if public_confirmed != PUBLIC_CONFIRMED_INFORMATION_COUNT {
        return Err(CoreError::InvalidContext(format!(
            "public confirmed {public_confirmed} != {PUBLIC_CONFIRMED_INFORMATION_COUNT}"
        )));
    }
    if private_confirmed != PRIVATE_CONFIRMED_INFORMATION_COUNT {
        return Err(CoreError::InvalidContext(format!(
            "private confirmed {private_confirmed} != {PRIVATE_CONFIRMED_INFORMATION_COUNT}"
        )));
    }
    if private_unverified != PRIVATE_UNVERIFIED_INFORMATION_COUNT {
        return Err(CoreError::InvalidContext(format!(
            "private unverified {private_unverified} != {PRIVATE_UNVERIFIED_INFORMATION_COUNT}"
        )));
    }

    // Confirmed vs unverified knower sets differ; second promisee does not know conflict
    for (realm_id, houses) in &houses_by_realm {
        let confirmed = context.information.iter().find(|i| {
            i.realm_id == *realm_id
                && i.topic == InformationTopic::PromiseConflict
                && i.confidence == InformationConfidence::Confirmed
        });
        let unverified = context.information.iter().find(|i| {
            i.realm_id == *realm_id
                && i.topic == InformationTopic::PromiseConflict
                && i.confidence == InformationConfidence::Unverified
        });
        let (c, u) = match (confirmed, unverified) {
            (Some(c), Some(u)) => (c, u),
            _ => {
                return Err(CoreError::InvalidContext(format!(
                    "realm {realm_id} missing confirmed/unverified PromiseConflict"
                )));
            }
        };
        let c_set: BTreeSet<&str> = c.known_by_person_ids.iter().map(|s| s.as_str()).collect();
        let u_set: BTreeSet<&str> = u.known_by_person_ids.iter().map(|s| s.as_str()).collect();
        if c_set == u_set {
            return Err(CoreError::InvalidContext(format!(
                "realm {realm_id} confirmed/unverified knower sets equal"
            )));
        }
        if !c_set.is_disjoint(&u_set) {
            return Err(CoreError::InvalidContext(format!(
                "realm {realm_id} confirmed/unverified knower sets overlap"
            )));
        }
        let h2 = houses
            .get(2)
            .ok_or_else(|| CoreError::InvalidContext("missing local[2]".to_string()))?;
        let head2 = h2.head_person_id.as_str();
        if c_set.contains(head2) || u_set.contains(head2) {
            return Err(CoreError::InvalidContext(format!(
                "realm {realm_id} second promisee {head2} must not know conflict info"
            )));
        }
    }

    // member_ids length safety already via get(); no panic paths for empty
    let _ = PERSONS_PER_HOUSE;

    Ok(())
}

fn validate_promise_conflict_subjects(
    item: &InformationItem,
    promises_by_realm: &BTreeMap<&str, Vec<&Promise>>,
) -> Result<(), CoreError> {
    let plist = promises_by_realm
        .get(item.realm_id.as_str())
        .ok_or_else(|| {
            CoreError::InvalidContext(format!("information {} no promises for realm", item.id))
        })?;
    if plist.len() != 2 {
        return Err(CoreError::InvalidContext(format!(
            "information {} realm promise count != 2",
            item.id
        )));
    }
    let mut expected: Vec<&str> = plist.iter().map(|p| p.id.as_str()).collect();
    expected.sort();
    let actual: Vec<&str> = item.subject_ids.iter().map(|s| s.as_str()).collect();
    if actual != expected {
        return Err(CoreError::InvalidContext(format!(
            "information {} PromiseConflict subjects mismatch",
            item.id
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_counts() {
        let cw = generate_context_world(1).expect("context");
        assert_eq!(cw.context.cultures.len(), 3);
        assert_eq!(cw.context.religions.len(), 2);
        assert_eq!(cw.context.realm_identities.len(), 6);
        assert_eq!(cw.context.house_identities.len(), 18);
        assert_eq!(cw.context.person_identities.len(), 144);
        assert_eq!(cw.context.relations.len(), 24);
        assert_eq!(cw.context.promises.len(), 12);
        assert_eq!(cw.context.information.len(), 18);
    }

    #[test]
    fn same_seed_equality() {
        let a = generate_context_world(1).expect("a");
        let b = generate_context_world(1).expect("b");
        assert_eq!(a, b);
        assert_eq!(
            a.to_compact_json_bytes().unwrap(),
            b.to_compact_json_bytes().unwrap()
        );
        let c = generate_context_world(2).expect("c");
        assert_ne!(
            a.to_compact_json_bytes().unwrap(),
            c.to_compact_json_bytes().unwrap()
        );
    }

    #[test]
    fn malformed_empty_context_no_panic() {
        let political = generate_political_world(1).expect("political");
        let empty = InitialPoliticalContext {
            cultures: vec![],
            religions: vec![],
            realm_identities: vec![],
            house_identities: vec![],
            person_identities: vec![],
            relations: vec![],
            promises: vec![],
            information: vec![],
        };
        let err = validate_initial_context(&political, &empty).unwrap_err();
        assert!(matches!(err, CoreError::InvalidContext(_)));
    }
}
