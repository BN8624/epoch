// M2.1 초기 혼인·혈통망 생성·검증 — RNG 없이 기존 M1 구조에서 파생

use crate::context::{HouseIdentity, PersonIdentity, RealmIdentity};
use crate::contextgen::validate_initial_context;
use crate::error::CoreError;
use crate::family::{
    FAMILY_WORLD_SCHEMA_VERSION, FamilyWorld, InitialFamilyNetwork, MARRIAGE_COUNT, Marriage,
    PARENTAGE_COUNT, ParentageLink,
};
use crate::political::ActiveRole;
use crate::politicalgen::validate_political_roster;
use crate::population::{GenerationBand, HOUSES_PER_REALM, House, Person};
use crate::populationgen::validate_population;
use crate::rights::{ClaimBasis, RightsWorld};
use crate::rightsgen::validate_initial_rights;
use crate::world::WORLD_REALM_COUNT;
use crate::worldgen::validate_world;
use std::collections::{BTreeMap, BTreeSet};

fn invalid_family(msg: impl Into<String>) -> CoreError {
    CoreError::InvalidFamily(msg.into())
}

fn map_layer_error(err: CoreError) -> CoreError {
    match err {
        CoreError::InvalidWorld(msg) => invalid_family(format!("world: {msg}")),
        CoreError::InvalidPopulation(msg) => invalid_family(format!("population: {msg}")),
        CoreError::InvalidPolitical(msg) => invalid_family(format!("political: {msg}")),
        CoreError::InvalidContext(msg) => invalid_family(format!("context: {msg}")),
        CoreError::InvalidRights(msg) => invalid_family(format!("rights: {msg}")),
        other => other,
    }
}

fn validate_lower_layers(rights_world: &RightsWorld) -> Result<(), CoreError> {
    let context_world = &rights_world.context_world;
    validate_world(&context_world.political.dynastic.world).map_err(map_layer_error)?;
    validate_population(
        &context_world.political.dynastic.world,
        &context_world.political.dynastic.population,
    )
    .map_err(map_layer_error)?;
    validate_political_roster(
        &context_world.political.dynastic,
        &context_world.political.roster,
    )
    .map_err(map_layer_error)?;
    validate_initial_context(&context_world.political, &context_world.context)
        .map_err(map_layer_error)?;
    validate_initial_rights(context_world, &rights_world.rights).map_err(map_layer_error)?;
    Ok(())
}

struct RealmFamilyRoles<'a> {
    realm_id: &'a str,
    h0: &'a House,
    h1: &'a House,
    h2: &'a House,
    h0_head: &'a Person,
    h1_head: &'a Person,
    h2_head: &'a Person,
    h0_current: &'a Person,
    marriage_a_child: &'a Person,
    marriage_b_child: &'a Person,
}

fn require_person<'a>(
    persons: &BTreeMap<&str, &'a Person>,
    id: &str,
    what: &str,
) -> Result<&'a Person, CoreError> {
    persons
        .get(id)
        .copied()
        .ok_or_else(|| invalid_family(format!("{what} person {id} missing")))
}

fn require_house<'a>(
    houses: &BTreeMap<&str, &'a House>,
    id: &str,
    what: &str,
) -> Result<&'a House, CoreError> {
    houses
        .get(id)
        .copied()
        .ok_or_else(|| invalid_family(format!("{what} house {id} missing")))
}

fn member_at<'a>(
    house: &House,
    index: usize,
    persons: &BTreeMap<&str, &'a Person>,
    what: &str,
) -> Result<&'a Person, CoreError> {
    let id = house.member_ids.get(index).ok_or_else(|| {
        invalid_family(format!(
            "house {} missing member_ids[{index}] for {what}",
            house.id
        ))
    })?;
    require_person(persons, id.as_str(), what)
}

fn house_identity<'a>(
    identities: &BTreeMap<&str, &'a HouseIdentity>,
    house_id: &str,
) -> Result<&'a HouseIdentity, CoreError> {
    identities
        .get(house_id)
        .copied()
        .ok_or_else(|| invalid_family(format!("missing house identity {house_id}")))
}

fn person_identity<'a>(
    identities: &BTreeMap<&str, &'a PersonIdentity>,
    person_id: &str,
) -> Result<&'a PersonIdentity, CoreError> {
    identities
        .get(person_id)
        .copied()
        .ok_or_else(|| invalid_family(format!("missing person identity {person_id}")))
}

fn realm_identity<'a>(
    identities: &BTreeMap<&str, &'a RealmIdentity>,
    realm_id: &str,
) -> Result<&'a RealmIdentity, CoreError> {
    identities
        .get(realm_id)
        .copied()
        .ok_or_else(|| invalid_family(format!("missing realm identity {realm_id}")))
}

fn sorted_pair(a: &str, b: &str, what: &str) -> Result<Vec<String>, CoreError> {
    if a == b {
        return Err(invalid_family(format!("{what} pair is not distinct: {a}")));
    }
    let mut pair = vec![a.to_string(), b.to_string()];
    pair.sort();
    Ok(pair)
}

fn family_lookups(rights_world: &RightsWorld) -> FamilyLookups<'_> {
    let pop = &rights_world.context_world.political.dynastic.population;
    FamilyLookups {
        person_by_id: pop.persons.iter().map(|p| (p.id.as_str(), p)).collect(),
        house_by_id: pop.houses.iter().map(|h| (h.id.as_str(), h)).collect(),
        realm_identity_by_id: rights_world
            .context_world
            .context
            .realm_identities
            .iter()
            .map(|r| (r.realm_id.as_str(), r))
            .collect(),
        house_identity_by_id: rights_world
            .context_world
            .context
            .house_identities
            .iter()
            .map(|h| (h.house_id.as_str(), h))
            .collect(),
        person_identity_by_id: rights_world
            .context_world
            .context
            .person_identities
            .iter()
            .map(|p| (p.person_id.as_str(), p))
            .collect(),
        realm_rights_by_id: rights_world
            .rights
            .realms
            .iter()
            .map(|r| (r.realm_id.as_str(), r))
            .collect(),
        active_by_person: rights_world
            .context_world
            .political
            .roster
            .active_actors
            .iter()
            .map(|a| (a.person_id.as_str(), a))
            .collect(),
        supporting: rights_world
            .context_world
            .political
            .roster
            .supporting_person_ids
            .iter()
            .map(|s| s.as_str())
            .collect(),
    }
}

struct FamilyLookups<'a> {
    person_by_id: BTreeMap<&'a str, &'a Person>,
    house_by_id: BTreeMap<&'a str, &'a House>,
    realm_identity_by_id: BTreeMap<&'a str, &'a RealmIdentity>,
    house_identity_by_id: BTreeMap<&'a str, &'a HouseIdentity>,
    person_identity_by_id: BTreeMap<&'a str, &'a PersonIdentity>,
    realm_rights_by_id: BTreeMap<&'a str, &'a crate::rights::RealmRights>,
    active_by_person: BTreeMap<&'a str, &'a crate::political::ActiveActor>,
    supporting: BTreeSet<&'a str>,
}

fn sorted_realms(rights_world: &RightsWorld) -> Result<Vec<&crate::world::Realm>, CoreError> {
    let mut realms: Vec<&crate::world::Realm> = rights_world
        .context_world
        .political
        .dynastic
        .world
        .realms
        .iter()
        .collect();
    realms.sort_by(|a, b| a.id.cmp(&b.id));
    if realms.len() != WORLD_REALM_COUNT {
        return Err(invalid_family(format!(
            "realm count {} != {WORLD_REALM_COUNT}",
            realms.len()
        )));
    }
    Ok(realms)
}

fn classify_realm_roles<'a>(
    realm_id: &'a str,
    lookups: &'a FamilyLookups<'a>,
    houses_in_realm: &[&'a House],
) -> Result<RealmFamilyRoles<'a>, CoreError> {
    if houses_in_realm.len() != HOUSES_PER_REALM {
        return Err(invalid_family(format!(
            "realm {realm_id} houses {} != {HOUSES_PER_REALM}",
            houses_in_realm.len()
        )));
    }
    let rr = lookups
        .realm_rights_by_id
        .get(realm_id)
        .copied()
        .ok_or_else(|| invalid_family(format!("missing realm rights {realm_id}")))?;
    let incumbent = require_person(
        &lookups.person_by_id,
        rr.incumbent_person_id.as_str(),
        "incumbent",
    )?;
    if incumbent.realm_id != realm_id {
        return Err(invalid_family(format!(
            "incumbent {} realm {} != {realm_id}",
            incumbent.id, incumbent.realm_id
        )));
    }
    let h0 = require_house(&lookups.house_by_id, incumbent.house_id.as_str(), "H0")?;
    if h0.realm_id != realm_id {
        return Err(invalid_family(format!(
            "H0 {} realm {} != {realm_id}",
            h0.id, h0.realm_id
        )));
    }
    if h0.head_person_id != incumbent.id {
        return Err(invalid_family(format!(
            "incumbent {} is not H0 {} head {}",
            incumbent.id, h0.id, h0.head_person_id
        )));
    }
    let ri = realm_identity(&lookups.realm_identity_by_id, realm_id)?;
    let h0_identity = house_identity(&lookups.house_identity_by_id, h0.id.as_str())?;
    if h0_identity.culture_id != ri.majority_culture_id
        || h0_identity.religion_id != ri.majority_religion_id
    {
        return Err(invalid_family(format!(
            "H0 {} is not majority identity of {realm_id}",
            h0.id
        )));
    }

    let mut h1: Option<&House> = None;
    let mut h2: Option<&House> = None;
    for house in houses_in_realm {
        if house.id == h0.id {
            continue;
        }
        if house.realm_id != realm_id {
            return Err(invalid_family(format!(
                "house {} realm {} != {realm_id}",
                house.id, house.realm_id
            )));
        }
        let hi = house_identity(&lookups.house_identity_by_id, house.id.as_str())?;
        let religious_minority =
            hi.culture_id == ri.majority_culture_id && hi.religion_id != ri.majority_religion_id;
        let cultural_minority =
            hi.culture_id != ri.majority_culture_id && hi.religion_id == ri.majority_religion_id;
        if religious_minority && cultural_minority {
            return Err(invalid_family(format!(
                "house {} matches both H1 and H2 identity",
                house.id
            )));
        }
        if religious_minority {
            if h1.is_some() {
                return Err(invalid_family(format!(
                    "realm {realm_id} has multiple religious-minority houses"
                )));
            }
            h1 = Some(*house);
        } else if cultural_minority {
            if h2.is_some() {
                return Err(invalid_family(format!(
                    "realm {realm_id} has multiple cultural-minority houses"
                )));
            }
            h2 = Some(*house);
        } else {
            return Err(invalid_family(format!(
                "house {} is neither H1 nor H2 of {realm_id}",
                house.id
            )));
        }
    }
    let h1 = h1.ok_or_else(|| {
        invalid_family(format!(
            "realm {realm_id} missing religious-minority house H1"
        ))
    })?;
    let h2 = h2.ok_or_else(|| {
        invalid_family(format!(
            "realm {realm_id} missing cultural-minority house H2"
        ))
    })?;
    if h0.id == h1.id || h0.id == h2.id || h1.id == h2.id {
        return Err(invalid_family(format!(
            "realm {realm_id} house roles are not distinct"
        )));
    }

    let h0_head = require_person(&lookups.person_by_id, h0.head_person_id.as_str(), "H0 head")?;
    let h1_head = require_person(&lookups.person_by_id, h1.head_person_id.as_str(), "H1 head")?;
    let h2_head = require_person(&lookups.person_by_id, h2.head_person_id.as_str(), "H2 head")?;
    let h0_current = member_at(h0, 3, &lookups.person_by_id, "H0 ruling-house current")?;
    let marriage_a_child = member_at(h0, 5, &lookups.person_by_id, "Marriage A child")?;
    let marriage_b_child = member_at(h2, 5, &lookups.person_by_id, "Marriage B child")?;

    Ok(RealmFamilyRoles {
        realm_id,
        h0,
        h1,
        h2,
        h0_head,
        h1_head,
        h2_head,
        h0_current,
        marriage_a_child,
        marriage_b_child,
    })
}

fn classify_all_realms<'a>(
    rights_world: &'a RightsWorld,
    lookups: &'a FamilyLookups<'a>,
) -> Result<Vec<RealmFamilyRoles<'a>>, CoreError> {
    let realms = sorted_realms(rights_world)?;
    let mut houses_by_realm: BTreeMap<&str, Vec<&House>> = BTreeMap::new();
    for house in &rights_world
        .context_world
        .political
        .dynastic
        .population
        .houses
    {
        houses_by_realm
            .entry(house.realm_id.as_str())
            .or_default()
            .push(house);
    }
    let mut roles = Vec::with_capacity(WORLD_REALM_COUNT);
    for realm in realms {
        let houses = houses_by_realm
            .get(realm.id.as_str())
            .ok_or_else(|| invalid_family(format!("missing houses for realm {}", realm.id)))?;
        roles.push(classify_realm_roles(realm.id.as_str(), lookups, houses)?);
    }
    if roles.len() != WORLD_REALM_COUNT {
        return Err(invalid_family(format!(
            "classified realms {} != {WORLD_REALM_COUNT}",
            roles.len()
        )));
    }
    Ok(roles)
}

fn make_marriage(
    id: String,
    left: &Person,
    right: &Person,
    realm_id: &str,
) -> Result<Marriage, CoreError> {
    if left.id == right.id {
        return Err(invalid_family(format!(
            "marriage {id} spouses are the same person {}",
            left.id
        )));
    }
    if left.house_id == right.house_id {
        return Err(invalid_family(format!(
            "marriage {id} is same-house {}",
            left.house_id
        )));
    }
    if left.realm_id != realm_id || right.realm_id != realm_id {
        return Err(invalid_family(format!(
            "marriage {id} realm mismatch {} / {} vs {realm_id}",
            left.realm_id, right.realm_id
        )));
    }
    Ok(Marriage {
        id,
        spouse_person_ids: sorted_pair(left.id.as_str(), right.id.as_str(), "spouse")?,
        house_ids: sorted_pair(left.house_id.as_str(), right.house_id.as_str(), "house")?,
        realm_ids: vec![realm_id.to_string()],
    })
}

fn make_parentage(
    id: String,
    marriage_id: String,
    child: &Person,
    left: &Person,
    right: &Person,
) -> Result<ParentageLink, CoreError> {
    Ok(ParentageLink {
        id,
        marriage_id,
        child_person_id: child.id.clone(),
        parent_person_ids: sorted_pair(left.id.as_str(), right.id.as_str(), "parent")?,
    })
}

/// RightsWorld에서 초기 혼인·양친 연결을 파생한다. RNG를 사용하지 않는다.
pub fn derive_initial_family(
    rights_world: &RightsWorld,
) -> Result<InitialFamilyNetwork, CoreError> {
    validate_lower_layers(rights_world)?;
    let lookups = family_lookups(rights_world);
    let roles = classify_all_realms(rights_world, &lookups)?;

    let mut marriages = Vec::with_capacity(MARRIAGE_COUNT);
    let mut parentages = Vec::with_capacity(PARENTAGE_COUNT);
    for (realm_idx, role) in roles.iter().enumerate() {
        let marriage_a_id = format!("marriage-{:02}", realm_idx * 2 + 1);
        let marriage_b_id = format!("marriage-{:02}", realm_idx * 2 + 2);
        let parentage_a_id = format!("parentage-{:02}", realm_idx * 2 + 1);
        let parentage_b_id = format!("parentage-{:02}", realm_idx * 2 + 2);

        marriages.push(make_marriage(
            marriage_a_id.clone(),
            role.h0_head,
            role.h1_head,
            role.realm_id,
        )?);
        marriages.push(make_marriage(
            marriage_b_id.clone(),
            role.h0_current,
            role.h2_head,
            role.realm_id,
        )?);
        parentages.push(make_parentage(
            parentage_a_id,
            marriage_a_id,
            role.marriage_a_child,
            role.h0_head,
            role.h1_head,
        )?);
        parentages.push(make_parentage(
            parentage_b_id,
            marriage_b_id,
            role.marriage_b_child,
            role.h0_current,
            role.h2_head,
        )?);
    }

    marriages.sort_by(|a, b| a.id.cmp(&b.id));
    parentages.sort_by(|a, b| a.id.cmp(&b.id));
    let family = InitialFamilyNetwork {
        marriages,
        parentages,
    };
    validate_initial_family(rights_world, &family)?;
    Ok(family)
}

/// seed에서 FamilyWorld를 생성한다.
pub fn generate_family_world(seed: u64) -> Result<FamilyWorld, CoreError> {
    let rights_world = crate::rightsgen::generate_rights_world(seed)?;
    validate_lower_layers(&rights_world)?;
    let family = derive_initial_family(&rights_world)?;
    validate_initial_family(&rights_world, &family)?;
    Ok(FamilyWorld {
        schema_version: FAMILY_WORLD_SCHEMA_VERSION,
        seed,
        rights_world,
        family,
    })
}

/// Family layer에서 확인되는 부모 ID를 조회한다. 권리 전파는 계산하지 않는다.
pub fn effective_parent_ids(
    rights_world: &RightsWorld,
    family: &InitialFamilyNetwork,
    person_id: &str,
) -> Result<Vec<String>, CoreError> {
    let person = rights_world
        .context_world
        .political
        .dynastic
        .population
        .persons
        .iter()
        .find(|p| p.id == person_id)
        .ok_or_else(|| invalid_family(format!("unknown person {person_id}")))?;
    let mut found: Option<&ParentageLink> = None;
    for link in &family.parentages {
        if link.child_person_id == person_id {
            if found.is_some() {
                return Err(invalid_family(format!(
                    "duplicate parentage for person {person_id}"
                )));
            }
            found = Some(link);
        }
    }
    match found {
        Some(link) => Ok(link.parent_person_ids.clone()),
        None => Ok(person.known_parent_ids.clone()),
    }
}

fn require_sorted_unique_ids(ids: &[String], what: &str) -> Result<(), CoreError> {
    for window in ids.windows(2) {
        let a = window
            .first()
            .ok_or_else(|| invalid_family(format!("{what} window")))?;
        let b = window
            .get(1)
            .ok_or_else(|| invalid_family(format!("{what} window")))?;
        if a >= b {
            return Err(invalid_family(format!(
                "{what} not strictly sorted: {a} / {b}"
            )));
        }
    }
    Ok(())
}

fn is_known_parent_child(left: &Person, right: &Person) -> bool {
    left.known_parent_ids.iter().any(|id| id == &right.id)
        || right.known_parent_ids.iter().any(|id| id == &left.id)
}

/// 초기 혼인·양친 연결 불변식을 검사한다. 실패 시 fail closed.
pub fn validate_initial_family(
    rights_world: &RightsWorld,
    family: &InitialFamilyNetwork,
) -> Result<(), CoreError> {
    validate_lower_layers(rights_world)?;
    let lookups = family_lookups(rights_world);
    let roles = classify_all_realms(rights_world, &lookups)?;

    if family.marriages.len() != MARRIAGE_COUNT {
        return Err(invalid_family(format!(
            "marriages {} != {MARRIAGE_COUNT}",
            family.marriages.len()
        )));
    }
    if family.parentages.len() != PARENTAGE_COUNT {
        return Err(invalid_family(format!(
            "parentages {} != {PARENTAGE_COUNT}",
            family.parentages.len()
        )));
    }

    let marriage_ids: Vec<String> = family.marriages.iter().map(|m| m.id.clone()).collect();
    let parentage_ids: Vec<String> = family.parentages.iter().map(|p| p.id.clone()).collect();
    require_sorted_unique_ids(&marriage_ids, "marriages")?;
    require_sorted_unique_ids(&parentage_ids, "parentages")?;

    let mut marriage_by_id: BTreeMap<&str, &Marriage> = BTreeMap::new();
    let mut spouse_seen: BTreeSet<&str> = BTreeSet::new();
    let mut spouse_pairs: BTreeSet<Vec<&str>> = BTreeSet::new();
    let mut marriages_by_realm: BTreeMap<&str, usize> = BTreeMap::new();
    let mut interfaith = 0usize;
    let mut intercultural = 0usize;

    for (idx, marriage) in family.marriages.iter().enumerate() {
        let expected_id = format!("marriage-{:02}", idx + 1);
        if marriage.id != expected_id {
            return Err(invalid_family(format!(
                "marriage id {} != {expected_id}",
                marriage.id
            )));
        }
        if marriage_by_id
            .insert(marriage.id.as_str(), marriage)
            .is_some()
        {
            return Err(invalid_family(format!(
                "duplicate marriage {}",
                marriage.id
            )));
        }
        if marriage.spouse_person_ids.len() != 2 {
            return Err(invalid_family(format!(
                "marriage {} spouse count {} != 2",
                marriage.id,
                marriage.spouse_person_ids.len()
            )));
        }
        if marriage.house_ids.len() != 2 {
            return Err(invalid_family(format!(
                "marriage {} house count {} != 2",
                marriage.id,
                marriage.house_ids.len()
            )));
        }
        if marriage.realm_ids.len() != 1 {
            return Err(invalid_family(format!(
                "marriage {} realm count {} != 1",
                marriage.id,
                marriage.realm_ids.len()
            )));
        }
        require_sorted_unique_ids(
            &marriage.spouse_person_ids,
            &format!("{} spouses", marriage.id),
        )?;
        require_sorted_unique_ids(&marriage.house_ids, &format!("{} houses", marriage.id))?;
        require_sorted_unique_ids(&marriage.realm_ids, &format!("{} realms", marriage.id))?;

        let left_id = marriage
            .spouse_person_ids
            .first()
            .ok_or_else(|| invalid_family(format!("marriage {} missing spouse 0", marriage.id)))?;
        let right_id = marriage
            .spouse_person_ids
            .get(1)
            .ok_or_else(|| invalid_family(format!("marriage {} missing spouse 1", marriage.id)))?;
        if left_id == right_id {
            return Err(invalid_family(format!(
                "marriage {} spouses are the same person {left_id}",
                marriage.id
            )));
        }
        let left = require_person(&lookups.person_by_id, left_id.as_str(), "spouse")?;
        let right = require_person(&lookups.person_by_id, right_id.as_str(), "spouse")?;
        if left.generation != GenerationBand::Current || right.generation != GenerationBand::Current
        {
            return Err(invalid_family(format!(
                "marriage {} spouses are not both Current",
                marriage.id
            )));
        }
        if !lookups.active_by_person.contains_key(left.id.as_str())
            || !lookups.active_by_person.contains_key(right.id.as_str())
        {
            return Err(invalid_family(format!(
                "marriage {} spouses are not both Active",
                marriage.id
            )));
        }
        if lookups.supporting.contains(left.id.as_str())
            || lookups.supporting.contains(right.id.as_str())
        {
            return Err(invalid_family(format!(
                "marriage {} includes a Supporting spouse",
                marriage.id
            )));
        }
        if left.house_id == right.house_id {
            return Err(invalid_family(format!(
                "marriage {} is same-house {}",
                marriage.id, left.house_id
            )));
        }
        if left.realm_id != right.realm_id {
            return Err(invalid_family(format!(
                "marriage {} spouses have different realms",
                marriage.id
            )));
        }
        let realm_id = marriage
            .realm_ids
            .first()
            .ok_or_else(|| invalid_family(format!("marriage {} missing realm", marriage.id)))?;
        if left.realm_id != *realm_id {
            return Err(invalid_family(format!(
                "marriage {} realm {realm_id} != spouse realm {}",
                marriage.id, left.realm_id
            )));
        }
        let expected_houses =
            sorted_pair(left.house_id.as_str(), right.house_id.as_str(), "house")?;
        if marriage.house_ids != expected_houses {
            return Err(invalid_family(format!(
                "marriage {} house_ids do not match spouses",
                marriage.id
            )));
        }
        let left_house = require_house(&lookups.house_by_id, left.house_id.as_str(), "spouse")?;
        let right_house = require_house(&lookups.house_by_id, right.house_id.as_str(), "spouse")?;
        if left_house.realm_id != *realm_id || right_house.realm_id != *realm_id {
            return Err(invalid_family(format!(
                "marriage {} house realm mismatch",
                marriage.id
            )));
        }
        if is_known_parent_child(left, right) {
            return Err(invalid_family(format!(
                "marriage {} spouses are parent-child",
                marriage.id
            )));
        }
        if !spouse_seen.insert(left.id.as_str()) || !spouse_seen.insert(right.id.as_str()) {
            return Err(invalid_family(format!(
                "person appears in more than one marriage around {}",
                marriage.id
            )));
        }
        let pair = vec![left.id.as_str(), right.id.as_str()];
        if !spouse_pairs.insert(pair) {
            return Err(invalid_family(format!(
                "duplicate spouse pair in {}",
                marriage.id
            )));
        }

        let left_ident = person_identity(&lookups.person_identity_by_id, left.id.as_str())?;
        let right_ident = person_identity(&lookups.person_identity_by_id, right.id.as_str())?;
        let same_culture = left_ident.culture_id == right_ident.culture_id;
        let same_religion = left_ident.religion_id == right_ident.religion_id;
        if same_culture && !same_religion {
            interfaith += 1;
        } else if !same_culture && same_religion {
            intercultural += 1;
        } else {
            return Err(invalid_family(format!(
                "marriage {} identity is neither interfaith nor intercultural",
                marriage.id
            )));
        }
        *marriages_by_realm.entry(realm_id.as_str()).or_insert(0) += 1;
    }

    if spouse_seen.len() != MARRIAGE_COUNT * 2 {
        return Err(invalid_family(format!(
            "unique spouses {} != {}",
            spouse_seen.len(),
            MARRIAGE_COUNT * 2
        )));
    }
    let active_ids: BTreeSet<&str> = lookups.active_by_person.keys().copied().collect();
    if spouse_seen != active_ids {
        return Err(invalid_family(
            "married persons are not exactly the Active actors".to_string(),
        ));
    }
    for supporting_id in &lookups.supporting {
        if spouse_seen.contains(supporting_id) {
            return Err(invalid_family(format!(
                "supporting person {supporting_id} is married"
            )));
        }
    }
    if marriages_by_realm.len() != WORLD_REALM_COUNT {
        return Err(invalid_family("marriage realm coverage incomplete"));
    }
    for (realm_id, count) in &marriages_by_realm {
        if *count != 2 {
            return Err(invalid_family(format!(
                "realm {realm_id} marriages {count} != 2"
            )));
        }
    }
    if interfaith != WORLD_REALM_COUNT || intercultural != WORLD_REALM_COUNT {
        return Err(invalid_family(format!(
            "identity counts interfaith={interfaith} intercultural={intercultural}"
        )));
    }

    let mut parentage_by_id: BTreeMap<&str, &ParentageLink> = BTreeMap::new();
    let mut child_seen: BTreeSet<&str> = BTreeSet::new();
    let mut parentages_by_realm: BTreeMap<&str, usize> = BTreeMap::new();

    for (idx, link) in family.parentages.iter().enumerate() {
        let expected_id = format!("parentage-{:02}", idx + 1);
        if link.id != expected_id {
            return Err(invalid_family(format!(
                "parentage id {} != {expected_id}",
                link.id
            )));
        }
        if parentage_by_id.insert(link.id.as_str(), link).is_some() {
            return Err(invalid_family(format!("duplicate parentage {}", link.id)));
        }
        if link.parent_person_ids.len() != 2 {
            return Err(invalid_family(format!(
                "parentage {} parent count {} != 2",
                link.id,
                link.parent_person_ids.len()
            )));
        }
        require_sorted_unique_ids(&link.parent_person_ids, &format!("{} parents", link.id))?;
        let marriage = marriage_by_id
            .get(link.marriage_id.as_str())
            .ok_or_else(|| {
                invalid_family(format!(
                    "parentage {} unknown marriage {}",
                    link.id, link.marriage_id
                ))
            })?;
        if link.parent_person_ids != marriage.spouse_person_ids {
            return Err(invalid_family(format!(
                "parentage {} parents do not match marriage {} spouses",
                link.id, marriage.id
            )));
        }
        let child = require_person(
            &lookups.person_by_id,
            link.child_person_id.as_str(),
            "child",
        )?;
        if child.generation != GenerationBand::Young {
            return Err(invalid_family(format!(
                "parentage {} child {} is not Young",
                link.id, child.id
            )));
        }
        if !child_seen.insert(child.id.as_str()) {
            return Err(invalid_family(format!(
                "duplicate child {} in {}",
                child.id, link.id
            )));
        }
        let parent_left_id = link
            .parent_person_ids
            .first()
            .ok_or_else(|| invalid_family(format!("parentage {} missing parent 0", link.id)))?;
        let parent_right_id = link
            .parent_person_ids
            .get(1)
            .ok_or_else(|| invalid_family(format!("parentage {} missing parent 1", link.id)))?;
        let parent_left = require_person(&lookups.person_by_id, parent_left_id.as_str(), "parent")?;
        let parent_right =
            require_person(&lookups.person_by_id, parent_right_id.as_str(), "parent")?;
        if child.id == parent_left.id || child.id == parent_right.id {
            return Err(invalid_family(format!(
                "parentage {} child is also a parent",
                link.id
            )));
        }
        if parent_left.generation != GenerationBand::Current
            || parent_right.generation != GenerationBand::Current
        {
            return Err(invalid_family(format!(
                "parentage {} parents are not both Current",
                link.id
            )));
        }
        if parent_left.house_id == parent_right.house_id {
            return Err(invalid_family(format!(
                "parentage {} parents share house {}",
                link.id, parent_left.house_id
            )));
        }
        if child.house_id != parent_left.house_id && child.house_id != parent_right.house_id {
            return Err(invalid_family(format!(
                "parentage {} child {} house is not a parent house",
                link.id, child.id
            )));
        }
        let child_house = require_house(&lookups.house_by_id, child.house_id.as_str(), "child")?;
        if child.realm_id != child_house.realm_id {
            return Err(invalid_family(format!(
                "parentage {} child {} realm {} != house realm {}",
                link.id, child.id, child.realm_id, child_house.realm_id
            )));
        }
        let known: BTreeSet<&str> = child.known_parent_ids.iter().map(String::as_str).collect();
        let parents: BTreeSet<&str> = [parent_left.id.as_str(), parent_right.id.as_str()]
            .into_iter()
            .collect();
        if !known.is_subset(&parents) {
            return Err(invalid_family(format!(
                "parentage {} drops existing known parent of {}",
                link.id, child.id
            )));
        }
        let overlap: BTreeSet<&str> = known.intersection(&parents).copied().collect();
        if overlap.len() != 1 || known.len() != 1 {
            return Err(invalid_family(format!(
                "parentage {} must keep exactly one existing known parent",
                link.id
            )));
        }
        let realm_id = marriage
            .realm_ids
            .first()
            .ok_or_else(|| invalid_family(format!("marriage {} missing realm", marriage.id)))?;
        *parentages_by_realm.entry(realm_id.as_str()).or_insert(0) += 1;
    }

    if child_seen.len() != PARENTAGE_COUNT {
        return Err(invalid_family(format!(
            "unique children {} != {PARENTAGE_COUNT}",
            child_seen.len()
        )));
    }
    for (realm_id, count) in &parentages_by_realm {
        if *count != 2 {
            return Err(invalid_family(format!(
                "realm {realm_id} parentages {count} != 2"
            )));
        }
    }

    let mut marriage_a = 0usize;
    let mut marriage_b = 0usize;
    let mut direct_claimants: BTreeSet<&str> = BTreeSet::new();
    for claim in &rights_world.rights.claims {
        if claim.basis == ClaimBasis::DirectDescent {
            direct_claimants.insert(claim.claimant_person_id.as_str());
        }
    }
    if direct_claimants.len() != WORLD_REALM_COUNT {
        return Err(invalid_family(format!(
            "direct claimants {} != {WORLD_REALM_COUNT}",
            direct_claimants.len()
        )));
    }

    for (realm_idx, role) in roles.iter().enumerate() {
        let marriage_a_id = format!("marriage-{:02}", realm_idx * 2 + 1);
        let marriage_b_id = format!("marriage-{:02}", realm_idx * 2 + 2);
        let parentage_a_id = format!("parentage-{:02}", realm_idx * 2 + 1);
        let parentage_b_id = format!("parentage-{:02}", realm_idx * 2 + 2);
        let marriage_a_rec = marriage_by_id
            .get(marriage_a_id.as_str())
            .ok_or_else(|| invalid_family(format!("missing Marriage A {marriage_a_id}")))?;
        let marriage_b_rec = marriage_by_id
            .get(marriage_b_id.as_str())
            .ok_or_else(|| invalid_family(format!("missing Marriage B {marriage_b_id}")))?;
        let parentage_a = parentage_by_id
            .get(parentage_a_id.as_str())
            .ok_or_else(|| invalid_family(format!("missing parentage A {parentage_a_id}")))?;
        let parentage_b = parentage_by_id
            .get(parentage_b_id.as_str())
            .ok_or_else(|| invalid_family(format!("missing parentage B {parentage_b_id}")))?;

        let expected_a_spouses = sorted_pair(
            role.h0_head.id.as_str(),
            role.h1_head.id.as_str(),
            "Marriage A spouse",
        )?;
        let expected_b_spouses = sorted_pair(
            role.h0_current.id.as_str(),
            role.h2_head.id.as_str(),
            "Marriage B spouse",
        )?;
        if marriage_a_rec.spouse_person_ids != expected_a_spouses {
            return Err(invalid_family(format!(
                "Marriage A {marriage_a_id} spouses are not H0 head × H1 head"
            )));
        }
        if marriage_b_rec.spouse_person_ids != expected_b_spouses {
            return Err(invalid_family(format!(
                "Marriage B {marriage_b_id} spouses are not H0 current × H2 head"
            )));
        }
        let expected_a_houses =
            sorted_pair(role.h0.id.as_str(), role.h1.id.as_str(), "Marriage A house")?;
        let expected_b_houses =
            sorted_pair(role.h0.id.as_str(), role.h2.id.as_str(), "Marriage B house")?;
        if marriage_a_rec.house_ids != expected_a_houses {
            return Err(invalid_family(format!(
                "Marriage A {marriage_a_id} houses are not H0 × H1"
            )));
        }
        if marriage_b_rec.house_ids != expected_b_houses {
            return Err(invalid_family(format!(
                "Marriage B {marriage_b_id} houses are not H0 × H2"
            )));
        }
        if marriage_a_rec.realm_ids.first().map(String::as_str) != Some(role.realm_id)
            || marriage_b_rec.realm_ids.first().map(String::as_str) != Some(role.realm_id)
        {
            return Err(invalid_family(format!(
                "realm {} marriage realm_ids mismatch",
                role.realm_id
            )));
        }

        let a_left = person_identity(&lookups.person_identity_by_id, role.h0_head.id.as_str())?;
        let a_right = person_identity(&lookups.person_identity_by_id, role.h1_head.id.as_str())?;
        if a_left.culture_id != a_right.culture_id || a_left.religion_id == a_right.religion_id {
            return Err(invalid_family(format!(
                "Marriage A {marriage_a_id} is not same-culture interfaith"
            )));
        }
        let b_left = person_identity(&lookups.person_identity_by_id, role.h0_current.id.as_str())?;
        let b_right = person_identity(&lookups.person_identity_by_id, role.h2_head.id.as_str())?;
        if b_left.culture_id == b_right.culture_id || b_left.religion_id != b_right.religion_id {
            return Err(invalid_family(format!(
                "Marriage B {marriage_b_id} is not intercultural same-faith"
            )));
        }

        if parentage_a.marriage_id != marriage_a_id
            || parentage_a.child_person_id != role.marriage_a_child.id
        {
            return Err(invalid_family(format!(
                "parentage {parentage_a_id} is not Marriage A child {}",
                role.marriage_a_child.id
            )));
        }
        if parentage_b.marriage_id != marriage_b_id
            || parentage_b.child_person_id != role.marriage_b_child.id
        {
            return Err(invalid_family(format!(
                "parentage {parentage_b_id} is not Marriage B child {}",
                role.marriage_b_child.id
            )));
        }
        if !direct_claimants.contains(role.marriage_a_child.id.as_str()) {
            return Err(invalid_family(format!(
                "Marriage A child {} is not the direct claimant",
                role.marriage_a_child.id
            )));
        }
        if role.marriage_a_child.house_id != role.h0.id {
            return Err(invalid_family(format!(
                "direct claimant {} is not in H0 {}",
                role.marriage_a_child.id, role.h0.id
            )));
        }
        if role.marriage_b_child.house_id != role.h2.id {
            return Err(invalid_family(format!(
                "Marriage B child {} is not in H2 {}",
                role.marriage_b_child.id, role.h2.id
            )));
        }

        let h0_head_actor = lookups
            .active_by_person
            .get(role.h0_head.id.as_str())
            .ok_or_else(|| invalid_family(format!("H0 head {} is not Active", role.h0_head.id)))?;
        if h0_head_actor.primary_role != ActiveRole::Ruler {
            return Err(invalid_family(format!(
                "H0 head {} primary role {:?} != Ruler",
                role.h0_head.id, h0_head_actor.primary_role
            )));
        }
        let h1_head_actor = lookups
            .active_by_person
            .get(role.h1_head.id.as_str())
            .ok_or_else(|| invalid_family(format!("H1 head {} is not Active", role.h1_head.id)))?;
        if h1_head_actor.primary_role != ActiveRole::HouseHead {
            return Err(invalid_family(format!(
                "H1 head {} primary role {:?} != HouseHead",
                role.h1_head.id, h1_head_actor.primary_role
            )));
        }
        let h2_head_actor = lookups
            .active_by_person
            .get(role.h2_head.id.as_str())
            .ok_or_else(|| invalid_family(format!("H2 head {} is not Active", role.h2_head.id)))?;
        if h2_head_actor.primary_role != ActiveRole::HouseHead {
            return Err(invalid_family(format!(
                "H2 head {} primary role {:?} != HouseHead",
                role.h2_head.id, h2_head_actor.primary_role
            )));
        }
        let h0_current_actor = lookups
            .active_by_person
            .get(role.h0_current.id.as_str())
            .ok_or_else(|| {
                invalid_family(format!("H0 current {} is not Active", role.h0_current.id))
            })?;
        if h0_current_actor.primary_role != ActiveRole::RulingHouseCurrent {
            return Err(invalid_family(format!(
                "H0 current {} primary role {:?} != RulingHouseCurrent",
                role.h0_current.id, h0_current_actor.primary_role
            )));
        }

        marriage_a += 1;
        marriage_b += 1;
    }

    if marriage_a != WORLD_REALM_COUNT || marriage_b != WORLD_REALM_COUNT {
        return Err(invalid_family(format!(
            "marriage type counts A={marriage_a} B={marriage_b}"
        )));
    }
    if rights_world.rights.claims.len() != crate::rights::SUCCESSION_CLAIM_COUNT {
        return Err(invalid_family(
            "family layer must not change the 12 existing claims".to_string(),
        ));
    }

    Ok(())
}
