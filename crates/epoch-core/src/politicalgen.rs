// M1.3 정치 활동 계층 생성·검증 — RNG 없이 M1.2 구조에서 파생

use crate::error::CoreError;
use crate::political::{
    ACTIVE_ACTOR_COUNT, ACTIVE_PER_REALM, ActivationReason, ActiveActor, ActiveRole,
    HOUSE_HEAD_ACTIVE_COUNT, POLITICAL_WORLD_SCHEMA_VERSION, PoliticalRoster, PoliticalWorld,
    RULER_ACTIVE_COUNT, RULING_HOUSE_CURRENT_ACTIVE_COUNT, SUPPORTING_PER_REALM,
    SUPPORTING_PERSON_COUNT, activation_reason_order,
};
use crate::population::{
    DynasticWorld, GenerationBand, HOUSES_PER_REALM, PERSON_COUNT, PERSONS_PER_HOUSE,
};
use crate::populationgen::{generate_dynastic_world, validate_population};
use crate::worldgen::validate_world;
use std::collections::{BTreeMap, BTreeSet};

/// DynasticWorld에서 정치 roster를 결정론적으로 파생한다. RNG를 사용하지 않는다.
pub fn derive_political_roster(dynastic: &DynasticWorld) -> Result<PoliticalRoster, CoreError> {
    // 깨진 입력은 조용히 받아들이지 않는다.
    validate_world(&dynastic.world).map_err(|e| match e {
        CoreError::InvalidWorld(msg) => CoreError::InvalidPolitical(format!("world: {msg}")),
        other => other,
    })?;
    validate_population(&dynastic.world, &dynastic.population).map_err(|e| match e {
        CoreError::InvalidPopulation(msg) => {
            CoreError::InvalidPolitical(format!("population: {msg}"))
        }
        other => other,
    })?;

    let pop = &dynastic.population;
    let person_by_id: BTreeMap<&str, &crate::population::Person> =
        pop.persons.iter().map(|p| (p.id.as_str(), p)).collect();

    // person_id → (realm, reasons set) 누적 후 primary_role 결정
    let mut reason_map: BTreeMap<String, (String, BTreeSet<ActivationReason>)> = BTreeMap::new();

    // 5.1 Rulers — 6명
    let mut ruler_person_ids: BTreeSet<String> = BTreeSet::new();
    for link in &pop.ruler_links {
        let person = person_by_id.get(link.person_id.as_str()).ok_or_else(|| {
            CoreError::InvalidPolitical(format!("ruler link person {} missing", link.person_id))
        })?;
        if !ruler_person_ids.insert(person.id.clone()) {
            return Err(CoreError::InvalidPolitical(format!(
                "duplicate ruler person {}",
                person.id
            )));
        }
        let entry = reason_map
            .entry(person.id.clone())
            .or_insert_with(|| (person.realm_id.clone(), BTreeSet::new()));
        if entry.0 != person.realm_id {
            return Err(CoreError::InvalidPolitical(format!(
                "person {} realm conflict",
                person.id
            )));
        }
        entry.1.insert(ActivationReason::Ruler);
        entry.1.insert(ActivationReason::HouseHead);
    }
    if ruler_person_ids.len() != RULER_ACTIVE_COUNT {
        return Err(CoreError::InvalidPolitical(format!(
            "ruler person count {} != {RULER_ACTIVE_COUNT}",
            ruler_person_ids.len()
        )));
    }

    // 가문을 realm별로 묶고 house ID 정렬 → [0] = ruling house
    let mut houses_by_realm: BTreeMap<String, Vec<&crate::population::House>> = BTreeMap::new();
    for house in &pop.houses {
        houses_by_realm
            .entry(house.realm_id.clone())
            .or_default()
            .push(house);
    }
    for houses in houses_by_realm.values_mut() {
        houses.sort_by(|a, b| a.id.cmp(&b.id));
    }

    // 6. Non-ruling house heads — 12명
    let mut non_ruling_head_count = 0usize;
    for houses in houses_by_realm.values() {
        if houses.len() != HOUSES_PER_REALM {
            return Err(CoreError::InvalidPolitical(format!(
                "realm houses count {} != {HOUSES_PER_REALM}",
                houses.len()
            )));
        }
        for (local_idx, house) in houses.iter().enumerate() {
            let is_ruling = local_idx == 0;
            let head_id = &house.head_person_id;
            let person = person_by_id.get(head_id.as_str()).ok_or_else(|| {
                CoreError::InvalidPolitical(format!("house {} head {} missing", house.id, head_id))
            })?;
            if is_ruling {
                // ruling head는 ruler로 이미 Active; 여기서 추가 선택하지 않음
                if !ruler_person_ids.contains(head_id) {
                    return Err(CoreError::InvalidPolitical(format!(
                        "ruling house {} head {} not ruler-linked",
                        house.id, head_id
                    )));
                }
                continue;
            }
            // non-ruling: must not be ruler-linked
            if ruler_person_ids.contains(head_id) {
                return Err(CoreError::InvalidPolitical(format!(
                    "non-ruling house {} head {} is ruler-linked",
                    house.id, head_id
                )));
            }
            let entry = reason_map
                .entry(person.id.clone())
                .or_insert_with(|| (person.realm_id.clone(), BTreeSet::new()));
            if entry.0 != person.realm_id {
                return Err(CoreError::InvalidPolitical(format!(
                    "person {} realm conflict",
                    person.id
                )));
            }
            entry.1.insert(ActivationReason::HouseHead);
            non_ruling_head_count += 1;
        }
    }
    if non_ruling_head_count != HOUSE_HEAD_ACTIVE_COUNT {
        return Err(CoreError::InvalidPolitical(format!(
            "non-ruling house head count {} != {HOUSE_HEAD_ACTIVE_COUNT}",
            non_ruling_head_count
        )));
    }

    // 7. Ruling-house additional Current = member_ids[3] — 6명
    let mut ruling_current_count = 0usize;
    for houses in houses_by_realm.values() {
        let ruling = houses[0];
        if ruling.member_ids.len() != PERSONS_PER_HOUSE {
            return Err(CoreError::InvalidPolitical(format!(
                "ruling house {} member_ids len {} != {PERSONS_PER_HOUSE}",
                ruling.id,
                ruling.member_ids.len()
            )));
        }
        let additional_id = &ruling.member_ids[3];
        if additional_id == &ruling.head_person_id || additional_id == &ruling.member_ids[2] {
            return Err(CoreError::InvalidPolitical(format!(
                "ruling house {} member_ids[3] equals head",
                ruling.id
            )));
        }
        if ruler_person_ids.contains(additional_id) {
            return Err(CoreError::InvalidPolitical(format!(
                "ruling house {} member_ids[3] {} is ruler-linked",
                ruling.id, additional_id
            )));
        }
        let person = person_by_id.get(additional_id.as_str()).ok_or_else(|| {
            CoreError::InvalidPolitical(format!(
                "ruling house {} member_ids[3] {} missing",
                ruling.id, additional_id
            ))
        })?;
        if person.generation != GenerationBand::Current {
            return Err(CoreError::InvalidPolitical(format!(
                "ruling house {} member_ids[3] {} is not Current",
                ruling.id, additional_id
            )));
        }
        if reason_map.contains_key(additional_id) {
            return Err(CoreError::InvalidPolitical(format!(
                "ruling house {} member_ids[3] {} already active",
                ruling.id, additional_id
            )));
        }
        let entry = reason_map
            .entry(person.id.clone())
            .or_insert_with(|| (person.realm_id.clone(), BTreeSet::new()));
        entry.1.insert(ActivationReason::RulingHouseCurrent);
        ruling_current_count += 1;
    }
    if ruling_current_count != RULING_HOUSE_CURRENT_ACTIVE_COUNT {
        return Err(CoreError::InvalidPolitical(format!(
            "ruling house current count {} != {RULING_HOUSE_CURRENT_ACTIVE_COUNT}",
            ruling_current_count
        )));
    }

    // ActiveActor 조립 + primary_role 결정
    let mut active_actors: Vec<ActiveActor> = Vec::with_capacity(ACTIVE_ACTOR_COUNT);
    for (person_id, (realm_id, reasons_set)) in reason_map {
        let primary_role = primary_role_from_reasons(&reasons_set)?;
        let mut activation_reasons: Vec<ActivationReason> = reasons_set.into_iter().collect();
        activation_reasons.sort_by_key(|r| activation_reason_order(*r));
        active_actors.push(ActiveActor {
            person_id,
            realm_id,
            primary_role,
            activation_reasons,
        });
    }

    if active_actors.len() != ACTIVE_ACTOR_COUNT {
        return Err(CoreError::InvalidPolitical(format!(
            "active actor count {} != {ACTIVE_ACTOR_COUNT}",
            active_actors.len()
        )));
    }

    // 13. 안정적 ordering: person_id 오름차순
    active_actors.sort_by(|a, b| a.person_id.cmp(&b.person_id));

    let active_ids: BTreeSet<String> = active_actors.iter().map(|a| a.person_id.clone()).collect();
    if active_ids.len() != ACTIVE_ACTOR_COUNT {
        return Err(CoreError::InvalidPolitical(
            "active person ids not unique".to_string(),
        ));
    }

    // 8. Supporting = 전체 - Active
    let mut supporting_person_ids: Vec<String> = pop
        .persons
        .iter()
        .filter(|p| !active_ids.contains(&p.id))
        .map(|p| p.id.clone())
        .collect();
    supporting_person_ids.sort();

    if supporting_person_ids.len() != SUPPORTING_PERSON_COUNT {
        return Err(CoreError::InvalidPolitical(format!(
            "supporting count {} != {SUPPORTING_PERSON_COUNT}",
            supporting_person_ids.len()
        )));
    }

    let roster = PoliticalRoster {
        active_actors,
        supporting_person_ids,
    };
    validate_political_roster(dynastic, &roster)?;
    Ok(roster)
}

fn primary_role_from_reasons(
    reasons: &BTreeSet<ActivationReason>,
) -> Result<ActiveRole, CoreError> {
    if reasons.contains(&ActivationReason::Ruler) {
        Ok(ActiveRole::Ruler)
    } else if reasons.contains(&ActivationReason::HouseHead) {
        Ok(ActiveRole::HouseHead)
    } else if reasons.contains(&ActivationReason::RulingHouseCurrent) {
        Ok(ActiveRole::RulingHouseCurrent)
    } else {
        Err(CoreError::InvalidPolitical(
            "active actor has no activation reasons".to_string(),
        ))
    }
}

/// seed에서 왕조 세계와 정치 roster를 함께 생성한다.
pub fn generate_political_world(seed: u64) -> Result<PoliticalWorld, CoreError> {
    let dynastic = generate_dynastic_world(seed)?;
    validate_population(&dynastic.world, &dynastic.population)?;
    let roster = derive_political_roster(&dynastic)?;
    validate_political_roster(&dynastic, &roster)?;
    Ok(PoliticalWorld {
        schema_version: POLITICAL_WORLD_SCHEMA_VERSION,
        seed,
        dynastic,
        roster,
    })
}

/// 정치 roster 불변식을 검사한다. 실패 시 fail closed.
pub fn validate_political_roster(
    dynastic: &DynasticWorld,
    roster: &PoliticalRoster,
) -> Result<(), CoreError> {
    let pop = &dynastic.population;
    let person_by_id: BTreeMap<&str, &crate::population::Person> =
        pop.persons.iter().map(|p| (p.id.as_str(), p)).collect();

    if pop.persons.len() != PERSON_COUNT {
        return Err(CoreError::InvalidPolitical(format!(
            "person count {} != {PERSON_COUNT}",
            pop.persons.len()
        )));
    }

    // counts
    if roster.active_actors.len() != ACTIVE_ACTOR_COUNT {
        return Err(CoreError::InvalidPolitical(format!(
            "active count {} != {ACTIVE_ACTOR_COUNT}",
            roster.active_actors.len()
        )));
    }
    if roster.supporting_person_ids.len() != SUPPORTING_PERSON_COUNT {
        return Err(CoreError::InvalidPolitical(format!(
            "supporting count {} != {SUPPORTING_PERSON_COUNT}",
            roster.supporting_person_ids.len()
        )));
    }

    // active ordering + uniqueness
    let mut prev_active: Option<&str> = None;
    let mut active_ids: BTreeSet<String> = BTreeSet::new();
    for actor in &roster.active_actors {
        if !active_ids.insert(actor.person_id.clone()) {
            return Err(CoreError::InvalidPolitical(format!(
                "duplicate active person {}",
                actor.person_id
            )));
        }
        if let Some(prev) = prev_active
            && actor.person_id.as_str() < prev
        {
            return Err(CoreError::InvalidPolitical(
                "active_actors not sorted by person_id".to_string(),
            ));
        }
        prev_active = Some(actor.person_id.as_str());
    }

    // supporting ordering + uniqueness
    let mut prev_sup: Option<&str> = None;
    let mut supporting_ids: BTreeSet<String> = BTreeSet::new();
    for sid in &roster.supporting_person_ids {
        if !supporting_ids.insert(sid.clone()) {
            return Err(CoreError::InvalidPolitical(format!(
                "duplicate supporting person {sid}"
            )));
        }
        if let Some(prev) = prev_sup
            && sid.as_str() < prev
        {
            return Err(CoreError::InvalidPolitical(
                "supporting_person_ids not sorted by person_id".to_string(),
            ));
        }
        prev_sup = Some(sid.as_str());
        if active_ids.contains(sid) {
            return Err(CoreError::InvalidPolitical(format!(
                "person {sid} in both active and supporting"
            )));
        }
        if !person_by_id.contains_key(sid.as_str()) {
            return Err(CoreError::InvalidPolitical(format!(
                "supporting person {sid} missing"
            )));
        }
    }

    // coverage: Active ∪ Supporting = all, intersection empty
    if active_ids.len() + supporting_ids.len() != PERSON_COUNT {
        return Err(CoreError::InvalidPolitical(format!(
            "coverage {} != {PERSON_COUNT}",
            active_ids.len() + supporting_ids.len()
        )));
    }
    for p in &pop.persons {
        if !active_ids.contains(&p.id) && !supporting_ids.contains(&p.id) {
            return Err(CoreError::InvalidPolitical(format!(
                "person {} not in active or supporting",
                p.id
            )));
        }
    }

    // role counts
    let mut ruler_n = 0usize;
    let mut house_head_n = 0usize;
    let mut rhc_n = 0usize;
    let mut active_per_realm: BTreeMap<String, usize> = BTreeMap::new();

    // houses by realm for role checks
    let mut houses_by_realm: BTreeMap<String, Vec<&crate::population::House>> = BTreeMap::new();
    for house in &pop.houses {
        houses_by_realm
            .entry(house.realm_id.clone())
            .or_default()
            .push(house);
    }
    for houses in houses_by_realm.values_mut() {
        houses.sort_by(|a, b| a.id.cmp(&b.id));
    }

    let ruler_person_ids: BTreeSet<&str> = pop
        .ruler_links
        .iter()
        .map(|l| l.person_id.as_str())
        .collect();

    // all house heads
    let all_head_ids: BTreeSet<&str> = pop
        .houses
        .iter()
        .map(|h| h.head_person_id.as_str())
        .collect();

    for actor in &roster.active_actors {
        let person = person_by_id.get(actor.person_id.as_str()).ok_or_else(|| {
            CoreError::InvalidPolitical(format!("active person {} missing", actor.person_id))
        })?;
        if person.realm_id != actor.realm_id {
            return Err(CoreError::InvalidPolitical(format!(
                "active {} realm {} != person realm {}",
                actor.person_id, actor.realm_id, person.realm_id
            )));
        }
        if person.generation != GenerationBand::Current {
            return Err(CoreError::InvalidPolitical(format!(
                "active {} is not Current",
                actor.person_id
            )));
        }
        *active_per_realm.entry(actor.realm_id.clone()).or_insert(0) += 1;

        // activation_reasons sorted and non-empty
        if actor.activation_reasons.is_empty() {
            return Err(CoreError::InvalidPolitical(format!(
                "active {} has empty activation_reasons",
                actor.person_id
            )));
        }
        for w in actor.activation_reasons.windows(2) {
            if activation_reason_order(w[0]) > activation_reason_order(w[1]) {
                return Err(CoreError::InvalidPolitical(format!(
                    "active {} activation_reasons not ordered",
                    actor.person_id
                )));
            }
        }
        // unique reasons
        let reason_set: BTreeSet<_> = actor.activation_reasons.iter().copied().collect();
        if reason_set.len() != actor.activation_reasons.len() {
            return Err(CoreError::InvalidPolitical(format!(
                "active {} duplicate activation_reasons",
                actor.person_id
            )));
        }

        match actor.primary_role {
            ActiveRole::Ruler => {
                ruler_n += 1;
                if !ruler_person_ids.contains(actor.person_id.as_str()) {
                    return Err(CoreError::InvalidPolitical(format!(
                        "primary Ruler {} not ruler-linked",
                        actor.person_id
                    )));
                }
                if !all_head_ids.contains(actor.person_id.as_str()) {
                    return Err(CoreError::InvalidPolitical(format!(
                        "primary Ruler {} is not a house head",
                        actor.person_id
                    )));
                }
                if !reason_set.contains(&ActivationReason::Ruler)
                    || !reason_set.contains(&ActivationReason::HouseHead)
                {
                    return Err(CoreError::InvalidPolitical(format!(
                        "primary Ruler {} missing required activation reasons",
                        actor.person_id
                    )));
                }
                // expected reasons exactly {Ruler, HouseHead}
                if reason_set.len() != 2 {
                    return Err(CoreError::InvalidPolitical(format!(
                        "primary Ruler {} unexpected activation reasons",
                        actor.person_id
                    )));
                }
            }
            ActiveRole::HouseHead => {
                house_head_n += 1;
                if !all_head_ids.contains(actor.person_id.as_str()) {
                    return Err(CoreError::InvalidPolitical(format!(
                        "primary HouseHead {} is not a house head",
                        actor.person_id
                    )));
                }
                if ruler_person_ids.contains(actor.person_id.as_str()) {
                    return Err(CoreError::InvalidPolitical(format!(
                        "primary HouseHead {} is ruler-linked",
                        actor.person_id
                    )));
                }
                if actor.activation_reasons != [ActivationReason::HouseHead] {
                    return Err(CoreError::InvalidPolitical(format!(
                        "primary HouseHead {} reasons must be [HouseHead]",
                        actor.person_id
                    )));
                }
            }
            ActiveRole::RulingHouseCurrent => {
                rhc_n += 1;
                let houses = houses_by_realm.get(&actor.realm_id).ok_or_else(|| {
                    CoreError::InvalidPolitical(format!("no houses for realm {}", actor.realm_id))
                })?;
                let ruling = houses[0];
                if ruling.member_ids.get(3).map(|s| s.as_str()) != Some(actor.person_id.as_str()) {
                    return Err(CoreError::InvalidPolitical(format!(
                        "primary RulingHouseCurrent {} != ruling house {} member_ids[3]",
                        actor.person_id, ruling.id
                    )));
                }
                if actor.person_id == ruling.head_person_id {
                    return Err(CoreError::InvalidPolitical(format!(
                        "primary RulingHouseCurrent {} is house head",
                        actor.person_id
                    )));
                }
                if ruler_person_ids.contains(actor.person_id.as_str()) {
                    return Err(CoreError::InvalidPolitical(format!(
                        "primary RulingHouseCurrent {} is ruler-linked",
                        actor.person_id
                    )));
                }
                if actor.activation_reasons != [ActivationReason::RulingHouseCurrent] {
                    return Err(CoreError::InvalidPolitical(format!(
                        "primary RulingHouseCurrent {} reasons must be [RulingHouseCurrent]",
                        actor.person_id
                    )));
                }
            }
        }
    }

    if ruler_n != RULER_ACTIVE_COUNT {
        return Err(CoreError::InvalidPolitical(format!(
            "Ruler primary count {ruler_n} != {RULER_ACTIVE_COUNT}"
        )));
    }
    if house_head_n != HOUSE_HEAD_ACTIVE_COUNT {
        return Err(CoreError::InvalidPolitical(format!(
            "HouseHead primary count {house_head_n} != {HOUSE_HEAD_ACTIVE_COUNT}"
        )));
    }
    if rhc_n != RULING_HOUSE_CURRENT_ACTIVE_COUNT {
        return Err(CoreError::InvalidPolitical(format!(
            "RulingHouseCurrent primary count {rhc_n} != {RULING_HOUSE_CURRENT_ACTIVE_COUNT}"
        )));
    }

    // all ruler-linked persons must be active
    for pid in &ruler_person_ids {
        if !active_ids.contains(*pid) {
            return Err(CoreError::InvalidPolitical(format!(
                "ruler-linked person {pid} not active"
            )));
        }
    }

    // all non-ruling house heads must be active
    for houses in houses_by_realm.values() {
        for (i, house) in houses.iter().enumerate() {
            if i == 0 {
                continue;
            }
            if !active_ids.contains(&house.head_person_id) {
                return Err(CoreError::InvalidPolitical(format!(
                    "non-ruling head {} not active",
                    house.head_person_id
                )));
            }
        }
        let additional = &houses[0].member_ids[3];
        if !active_ids.contains(additional) {
            return Err(CoreError::InvalidPolitical(format!(
                "ruling house member_ids[3] {additional} not active"
            )));
        }
    }

    // realm distribution: active 4 / supporting 20 per realm
    let realm_ids: BTreeSet<String> = dynastic.world.realms.iter().map(|r| r.id.clone()).collect();
    if realm_ids.len() != 6 {
        return Err(CoreError::InvalidPolitical(format!(
            "realm count {} != 6",
            realm_ids.len()
        )));
    }
    for rid in &realm_ids {
        let a = *active_per_realm.get(rid).unwrap_or(&0);
        if a != ACTIVE_PER_REALM {
            return Err(CoreError::InvalidPolitical(format!(
                "realm {rid} active {a} != {ACTIVE_PER_REALM}"
            )));
        }
        let s = pop
            .persons
            .iter()
            .filter(|p| p.realm_id == *rid && supporting_ids.contains(&p.id))
            .count();
        if s != SUPPORTING_PER_REALM {
            return Err(CoreError::InvalidPolitical(format!(
                "realm {rid} supporting {s} != {SUPPORTING_PER_REALM}"
            )));
        }
        // per-realm role composition
        let mut r = 0usize;
        let mut h = 0usize;
        let mut c = 0usize;
        for actor in &roster.active_actors {
            if actor.realm_id != *rid {
                continue;
            }
            match actor.primary_role {
                ActiveRole::Ruler => r += 1,
                ActiveRole::HouseHead => h += 1,
                ActiveRole::RulingHouseCurrent => c += 1,
            }
        }
        if r != 1 || h != 2 || c != 1 {
            return Err(CoreError::InvalidPolitical(format!(
                "realm {rid} role mix ruler={r} house_head={h} rhc={c}"
            )));
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::populationgen::generate_dynastic_world;

    #[test]
    fn derive_counts_and_roles() {
        let d = generate_dynastic_world(1).expect("dynastic");
        let roster = derive_political_roster(&d).expect("roster");
        assert_eq!(roster.active_actors.len(), 24);
        assert_eq!(roster.supporting_person_ids.len(), 120);
        let rulers = roster
            .active_actors
            .iter()
            .filter(|a| a.primary_role == ActiveRole::Ruler)
            .count();
        let heads = roster
            .active_actors
            .iter()
            .filter(|a| a.primary_role == ActiveRole::HouseHead)
            .count();
        let rhc = roster
            .active_actors
            .iter()
            .filter(|a| a.primary_role == ActiveRole::RulingHouseCurrent)
            .count();
        assert_eq!(rulers, 6);
        assert_eq!(heads, 12);
        assert_eq!(rhc, 6);
    }

    #[test]
    fn same_seed_political_equality() {
        let a = generate_political_world(1).expect("a");
        let b = generate_political_world(1).expect("b");
        assert_eq!(a, b);
        assert_eq!(
            a.to_compact_json_bytes().unwrap(),
            b.to_compact_json_bytes().unwrap()
        );
        let c = generate_political_world(2).expect("c");
        assert_ne!(
            a.to_compact_json_bytes().unwrap(),
            c.to_compact_json_bytes().unwrap()
        );
    }
}
